use omasend::localsend::{Node, NodeConfig, TransferEvent, Upload};
use std::time::Duration;

async fn node(alias: &str, downloads: &std::path::Path) -> Node {
    Node::start(NodeConfig {
        alias: alias.into(),
        port: 0,
        downloads: downloads.into(),
        discovery: false,
    })
    .await
    .unwrap()
}

async fn event(node: &Node) -> TransferEvent {
    tokio::time::timeout(Duration::from_secs(15), node.events.recv())
        .await
        .expect("transfer event timed out")
        .unwrap()
}

#[tokio::test]
async fn https_transfer_requires_acceptance_and_preserves_existing_files() {
    let root = tempfile::tempdir().unwrap();
    let sender_dir = root.path().join("sender");
    let receiver_dir = root.path().join("receiver");
    std::fs::create_dir_all(&sender_dir).unwrap();
    std::fs::create_dir_all(&receiver_dir).unwrap();
    std::fs::write(sender_dir.join("photo.png"), b"binary\0\xff\r\n").unwrap();
    std::fs::write(receiver_dir.join("photo.png"), b"keep me").unwrap();
    let sender = node("Sender", &sender_dir).await;
    let receiver = node("Receiver", &receiver_dir).await;
    let send_id = sender
        .handle
        .send(
            receiver.device.clone(),
            vec![Upload::file(sender_dir.join("photo.png"), "photo.png".into()).unwrap()],
            None,
        )
        .unwrap();
    let receive_id = loop {
        if let TransferEvent::IncomingRequest { id, files, .. } = event(&receiver).await {
            assert_eq!(files[0].name, "photo.png");
            assert_eq!(files[0].size, 10);
            break id;
        }
    };
    assert!(!receiver_dir.join("photo (1).png").exists());
    receiver.handle.decide(&receive_id, true).unwrap();
    loop {
        match event(&receiver).await {
            TransferEvent::Completed { id, paths } if id == receive_id => {
                assert_eq!(
                    paths[0].canonicalize().unwrap(),
                    receiver_dir.join("photo (1).png").canonicalize().unwrap()
                );
                break;
            }
            TransferEvent::Failed { error, .. } => panic!("{error}"),
            _ => {}
        }
    }
    loop {
        match event(&sender).await {
            TransferEvent::Completed { id, .. } if id == send_id => break,
            TransferEvent::Failed { error, .. } => panic!("{error}"),
            _ => {}
        }
    }
    assert_eq!(
        std::fs::read(receiver_dir.join("photo.png")).unwrap(),
        b"keep me"
    );
    assert_eq!(
        std::fs::read(receiver_dir.join("photo (1).png")).unwrap(),
        b"binary\0\xff\r\n"
    );
    let port = receiver.device.port;
    receiver.shutdown().await;
    let rebound = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();
    drop(rebound);
    sender.shutdown().await;
}

#[tokio::test]
async fn refusing_request_does_not_create_a_received_file() {
    let dir = tempfile::tempdir().unwrap();
    let sender = node("Sender", dir.path()).await;
    let receiver_dir = tempfile::tempdir().unwrap();
    let receiver = node("Receiver", receiver_dir.path()).await;
    sender
        .handle
        .send(
            receiver.device.clone(),
            vec![Upload::text("hello".into())],
            None,
        )
        .unwrap();
    loop {
        if let TransferEvent::IncomingRequest { id, .. } = event(&receiver).await {
            receiver.handle.decide(&id, false).unwrap();
            break;
        }
    }
    loop {
        match event(&sender).await {
            TransferEvent::Failed { .. } => break,
            TransferEvent::Completed { .. } => panic!("declined transfer completed"),
            _ => {}
        }
    }
    assert_eq!(std::fs::read_dir(receiver_dir.path()).unwrap().count(), 0);
    sender.shutdown().await;
    receiver.shutdown().await;
}

#[tokio::test]
async fn wrong_fingerprint_does_not_offer_metadata_to_peer() {
    let dir = tempfile::tempdir().unwrap();
    let sender = node("Sender", dir.path()).await;
    let receiver = node("Receiver", dir.path()).await;
    let mut impostor = receiver.device.clone();
    impostor.fingerprint = "00".repeat(32);
    sender
        .handle
        .send(impostor, vec![Upload::text("private".into())], None)
        .unwrap();
    loop {
        match event(&sender).await {
            TransferEvent::Failed { .. } => break,
            TransferEvent::Completed { .. } => panic!("fingerprint mismatch completed"),
            _ => {}
        }
    }
    while let Ok(event) = receiver.events.try_recv() {
        assert!(!matches!(event, TransferEvent::IncomingRequest { .. }));
    }
    sender.shutdown().await;
    receiver.shutdown().await;
}

#[tokio::test]
async fn cancelling_before_acceptance_clears_the_remote_request() {
    let dir = tempfile::tempdir().unwrap();
    let sender = node("Sender", dir.path()).await;
    let receiver = node("Receiver", dir.path()).await;
    let send_id = sender
        .handle
        .send(
            receiver.device.clone(),
            vec![Upload::text("cancel me".into())],
            None,
        )
        .unwrap();
    let receive_id = loop {
        if let TransferEvent::IncomingRequest { id, .. } = event(&receiver).await {
            break id;
        }
    };
    sender.handle.cancel(&send_id).unwrap();
    loop {
        match event(&sender).await {
            TransferEvent::Cancelled { id } if id == send_id => break,
            TransferEvent::Completed { .. } => panic!("cancelled send completed"),
            _ => {}
        }
    }
    loop {
        if let TransferEvent::Cancelled { id } = event(&receiver).await {
            assert_eq!(id, receive_id);
            break;
        }
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    sender.shutdown().await;
    receiver.shutdown().await;
}

async fn raw_offer(
    receiver: &Node,
    name: &str,
    size: u64,
    hash: Option<String>,
) -> (
    std::sync::Arc<localsend::http::client::LsHttpClientV2>,
    tokio::task::JoinHandle<
        Result<
            localsend::http::dto_v2::PrepareUploadResultV2,
            localsend::http::client::ClientError,
        >,
    >,
) {
    use localsend::{
        crypto::cert::generate_self_signed,
        http::{
            client::LsHttpClientV2,
            dto_v2::{PrepareUploadRequestDtoV2, RegisterDtoV2},
        },
        model::{
            discovery::{DeviceType, ProtocolType},
            transfer::FileDto,
        },
    };
    let identity = generate_self_signed().unwrap();
    let client = std::sync::Arc::new(
        LsHttpClientV2::try_new(
            &identity.private_key_pem,
            &identity.certificate_pem,
            Some(receiver.device.fingerprint.clone()),
            Some(Duration::from_secs(15)),
        )
        .unwrap(),
    );
    let file = FileDto {
        id: "file".into(),
        file_name: name.into(),
        size,
        file_type: "application/octet-stream".into(),
        sha256: hash,
        preview: None,
        metadata: None,
    };
    let payload = PrepareUploadRequestDtoV2 {
        info: RegisterDtoV2 {
            alias: "Official core sender".into(),
            version: "2.2".into(),
            device_model: None,
            device_type: Some(DeviceType::Desktop),
            fingerprint: identity.fingerprint,
            port: 53317,
            protocol: ProtocolType::Https,
            download: false,
        },
        files: [("file".into(), file)].into_iter().collect(),
    };
    let sender = client.clone();
    let port = receiver.device.port;
    let task = tokio::spawn(async move {
        sender
            .prepare_upload(
                ProtocolType::Https,
                "127.0.0.1",
                port,
                None,
                payload,
                None,
                tokio_util::sync::CancellationToken::new(),
            )
            .await
    });
    (client, task)
}

#[tokio::test]
async fn unsafe_relative_paths_are_declined_without_a_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let receiver = node("Receiver", dir.path()).await;
    for name in [
        "../escape",
        "/absolute",
        "safe/../../escape",
        "C:\\escape",
        "safe/./file",
    ] {
        let (_, prepare) = raw_offer(&receiver, name, 1, None).await;
        assert!(
            prepare.await.unwrap().is_err(),
            "unsafe path accepted: {name}"
        );
        while let Ok(event) = receiver.events.try_recv() {
            assert!(!matches!(event, TransferEvent::IncomingRequest { .. }));
        }
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    receiver.shutdown().await;
}

#[tokio::test]
async fn truncated_or_corrupted_uploads_do_not_publish_files() {
    let dir = tempfile::tempdir().unwrap();
    let receiver = node("Receiver", dir.path()).await;
    for (size, hash) in [(100, None), (3, Some("00".repeat(32)))] {
        let (client, prepare) = raw_offer(&receiver, "bad.bin", size, hash).await;
        let receive_id = loop {
            if let TransferEvent::IncomingRequest { id, .. } = event(&receiver).await {
                receiver.handle.decide(&id, true).unwrap();
                break id;
            }
        };
        let response = prepare.await.unwrap().unwrap().response.unwrap();
        let result = client
            .upload(
                localsend::model::discovery::ProtocolType::Https,
                "127.0.0.1",
                receiver.device.port,
                None,
                &response.session_id,
                "file",
                &response.files["file"],
                localsend::reqwest::Body::from("bad"),
                tokio_util::sync::CancellationToken::new(),
            )
            .await;
        assert!(result.is_err());
        loop {
            match event(&receiver).await {
                TransferEvent::Failed { id, .. } if id == receive_id => break,
                TransferEvent::Completed { .. } => panic!("invalid upload completed"),
                _ => {}
            }
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
    receiver.shutdown().await;
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_parent_cannot_escape_downloads() {
    let downloads = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), downloads.path().join("redirect")).unwrap();
    let receiver = node("Receiver", downloads.path()).await;
    let (client, prepare) = raw_offer(&receiver, "redirect/file.bin", 3, None).await;
    loop {
        if let TransferEvent::IncomingRequest { id, .. } = event(&receiver).await {
            receiver.handle.decide(&id, true).unwrap();
            break;
        }
    }
    let response = prepare.await.unwrap().unwrap().response.unwrap();
    assert!(
        client
            .upload(
                localsend::model::discovery::ProtocolType::Https,
                "127.0.0.1",
                receiver.device.port,
                None,
                &response.session_id,
                "file",
                &response.files["file"],
                localsend::reqwest::Body::from("bad"),
                tokio_util::sync::CancellationToken::new()
            )
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    receiver.shutdown().await;
}

#[tokio::test]
async fn received_message_exposes_full_text_after_acceptance() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let sender = node("Sender", source.path()).await;
    let receiver = node("Receiver", destination.path()).await;
    let text = "你好\n  long message with whitespace\n".repeat(100);
    sender
        .handle
        .send(
            receiver.device.clone(),
            vec![Upload::text(text.clone())],
            None,
        )
        .unwrap();
    let mut received_text = None;
    loop {
        match event(&receiver).await {
            TransferEvent::IncomingRequest { id, .. } => receiver.handle.decide(&id, true).unwrap(),
            TransferEvent::ReceivedText { text, .. } => received_text = Some(text),
            TransferEvent::Completed { paths, .. } => {
                assert_eq!(received_text.as_deref(), Some(text.as_str()));
                assert_eq!(std::fs::read_to_string(&paths[0]).unwrap(), text);
                break;
            }
            TransferEvent::Failed { error, .. } => panic!("{error}"),
            _ => {}
        }
    }
    sender.shutdown().await;
    receiver.shutdown().await;
}
