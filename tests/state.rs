use omasend::{
    localsend::{Device, TransferEvent},
    model::{AppState, ComposerItem, SendItem, TransferStatus},
};

#[test]
fn completed_send_only_removes_its_snapshot_and_releases_clipboard_file() {
    let dir = tempfile::tempdir().unwrap();
    let items = omasend::clipboard::read_offer(
        omasend::clipboard::ClipboardKind::Image("image/png".into()),
        b"png",
        dir.path(),
    )
    .unwrap();
    let path = items[0].path().unwrap().to_owned();
    let mut state = AppState::default();
    state
        .composer
        .push(ComposerItem::new(items.into_iter().next().unwrap()).unwrap());
    state.track_send("one".into());
    state
        .composer
        .push(ComposerItem::new(SendItem::Text("later".into())).unwrap());
    state.apply(TransferEvent::Completed {
        id: "one".into(),
        paths: vec![],
    });
    assert_eq!(state.composer.len(), 1);
    assert!(matches!(&state.composer[0].item, SendItem::Text(text) if text == "later"));
    assert!(!path.exists());
    assert_eq!(state.transfers[0].status, TransferStatus::Completed);
}

#[test]
fn failure_retains_composer_and_selection_tracks_device_identity() {
    let mut state = AppState::default();
    state
        .composer
        .push(ComposerItem::new(SendItem::Text("retry".into())).unwrap());
    state.track_send("one".into());
    state.apply(TransferEvent::Failed {
        id: "one".into(),
        error: "offline".into(),
    });
    assert_eq!(state.composer.len(), 1);
    for id in ["a", "b"] {
        state.apply(TransferEvent::DeviceFound(Device {
            fingerprint: id.into(),
            alias: id.into(),
            model: "phone".into(),
            host: "127.0.0.1".into(),
            port: 53317,
        }));
    }
    state.navigate(1);
    assert_eq!(state.selected.as_deref(), Some("b"));
    state.apply(TransferEvent::DeviceLost("a".into()));
    assert_eq!(state.selected.as_deref(), Some("b"));
    state.apply(TransferEvent::DeviceLost("b".into()));
    assert!(state.selected.is_none());
}

#[test]
fn discovery_feedback_distinguishes_a_new_peer_from_a_periodic_update() {
    let mut state = AppState::default();
    state.apply(TransferEvent::DiscoveryActive(true));
    assert!(state.discovering);
    let peer = Device {
        fingerprint: "phone".into(),
        alias: "Phone".into(),
        model: "iPhone".into(),
        host: "192.168.1.2".into(),
        port: 53317,
    };
    state.apply(TransferEvent::DeviceFound(peer.clone()));
    assert_eq!(state.discovery_revision, 1);
    state.apply(TransferEvent::DeviceFound(peer.clone()));
    assert_eq!(
        state.discovery_revision, 1,
        "routine probes must not flash the logo"
    );
    state.apply(TransferEvent::DiscoveryActive(false));
    assert!(!state.discovering);
    state.apply(TransferEvent::DeviceLost(peer.fingerprint.clone()));
    state.apply(TransferEvent::DeviceFound(peer));
    assert_eq!(
        state.discovery_revision, 2,
        "returning peers are new discoveries"
    );
}

#[test]
fn acceptance_is_independent_of_the_first_uploaded_byte() {
    let mut state = AppState::default();
    state.track_send("empty-file".into());
    assert!(state.transfers[0].awaiting_acceptance);
    state.apply(TransferEvent::Accepted {
        id: "empty-file".into(),
    });
    assert!(!state.transfers[0].awaiting_acceptance);
    assert_eq!(state.transfers[0].transferred, 0);
}

#[test]
fn active_peer_survives_discovery_expiry_until_transfer_finishes() {
    let mut state = AppState::default();
    let peer = Device {
        fingerprint: "peer".into(),
        alias: "Same name".into(),
        model: "macOS".into(),
        host: "192.168.1.2".into(),
        port: 53317,
    };
    state.apply(TransferEvent::DeviceFound(peer.clone()));
    state.track_send("transfer".into());
    state.apply(TransferEvent::DeviceLost(peer.fingerprint.clone()));
    assert_eq!(state.devices.len(), 1);
    assert_eq!(state.selected.as_deref(), Some("peer"));
    state.apply(TransferEvent::Completed {
        id: "transfer".into(),
        paths: vec![],
    });
    assert!(state.devices.is_empty());
    state.apply(TransferEvent::DeviceFound(peer.clone()));
    state.track_send("next".into());
    state.apply(TransferEvent::DeviceLost(peer.fingerprint.clone()));
    state.apply(TransferEvent::DeviceFound(peer));
    state.apply(TransferEvent::Cancelled { id: "next".into() });
    assert_eq!(
        state.devices.len(),
        1,
        "rediscovered peers remain after transfer"
    );
}
