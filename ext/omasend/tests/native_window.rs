//! Opt-in fixture for exercising a real Omasend window against the official core.
//! Run while the app is open: cargo test --no-default-features --test native_window -- --ignored --nocapture
use omasend::localsend::{Node, NodeConfig, TransferEvent, Upload};
use std::time::Duration;

#[tokio::test]
#[ignore = "requires a running Omasend window and manual acceptance"]
async fn native_window_round_trip() {
    let downloads = tempfile::tempdir().unwrap();
    let peer = Node::start(NodeConfig {
        alias: "Omasend test peer".into(),
        port: 0,
        downloads: downloads.path().into(),
        discovery: true,
    })
    .await
    .unwrap();
    let fixture = "Omasend native window test\n";
    let local_addresses: Vec<String> = if_addrs::get_if_addrs()
        .unwrap()
        .into_iter()
        .map(|interface| interface.ip().to_string())
        .collect();
    let mut gui_fingerprint = None;
    let mut offered = false;
    let mut sent = false;
    let mut received = false;
    let deadline = tokio::time::sleep(Duration::from_secs(300));
    tokio::pin!(deadline);
    loop {
        let event = tokio::select! {
            event = peer.events.recv() => event.unwrap(),
            _ = &mut deadline => panic!("Native-window test timed out"),
        };
        match event {
            TransferEvent::DeviceFound(device)
                if device.port == 53317
                    && device.model == "Omasend"
                    && local_addresses.contains(&device.host)
                    && !offered =>
            {
                println!(
                    "Found GUI: {}. Accept the incoming test text in the window.",
                    device.alias
                );
                gui_fingerprint = Some(device.fingerprint.clone());
                peer.handle
                    .send(device, vec![Upload::text(fixture.into())], None)
                    .unwrap();
                offered = true;
            }
            TransferEvent::IncomingRequest {
                id,
                files,
                peer: sender,
            } => {
                if Some(&sender.fingerprint) != gui_fingerprint.as_ref() {
                    peer.handle.decide(&id, false).unwrap();
                    continue;
                }
                println!(
                    "GUI offered {} files; accepting in test fixture",
                    files.len()
                );
                peer.handle.decide(&id, true).unwrap();
            }
            TransferEvent::Completed { paths, .. } => {
                if paths.is_empty() {
                    sent = true;
                    println!("GUI received the fixture text");
                } else {
                    for path in paths {
                        assert!(!std::fs::read(path).unwrap().is_empty());
                    }
                    received = true;
                    println!("GUI sent non-empty content to the fixture");
                }
                if sent && received {
                    break;
                }
            }
            TransferEvent::Failed { error, .. } => panic!("{error}"),
            _ => {}
        }
    }
    peer.shutdown().await;
}
