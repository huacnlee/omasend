#[derive(Clone, Copy, Debug)]
struct TestRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
// The actual NSH engine drives application commands into real HTTPS sockets.
use super::{control, panels, session::Session};
use nsh_core::{
    ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, Size, ui::prelude::*,
};
use nsh_engine::{Engine, EngineConfig, EngineControl, Environment, MountSink};
use omasend::{
    localsend::{Device, Node, NodeConfig, TransferEvent},
    model::SendItem,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

struct Bootstrap {
    config: NodeConfig,
    peer: Device,
    file: std::path::PathBuf,
    identity: crossbeam_channel::Sender<Device>,
}
static BOOTSTRAP: Mutex<Option<Bootstrap>> = Mutex::new(None);

#[component]
fn TestApp() -> impl IntoElement {
    let Bootstrap {
        config,
        peer,
        file,
        identity,
    } = BOOTSTRAP.lock().unwrap().take().unwrap();
    let appearance = super::appearance::install_for_test();
    let session = Session::new();
    session.state.borrow_mut().selected = Some(peer.fingerprint.clone());
    session.state.borrow_mut().devices.push(peer);
    session.start_with(Ok(config));
    session.input(async move {
        Ok(vec![
            SendItem::File(file),
            SendItem::Text("hello from NSH".into()),
        ])
    });
    let own = session.identity;
    let reported = std::cell::Cell::new(false);
    effect(move || {
        if let Some(device) = own.get()
            && !reported.replace(true)
        {
            identity.send(device).unwrap();
        }
    });
    let language = session.clone();
    let previewer = session.clone();
    let sender = session.clone();
    let disabled = session.clone();
    let ready = session.clone();
    view()
        .child(
            super::editor()
                .label("Stable draft")
                .value("draft stays mounted".to_owned()),
        )
        .child(control(
            "Send",
            move || disabled.cannot_send(),
            move || sender.send(),
        ))
        .child(text(move || {
            if ready.cannot_send() {
                "Starting"
            } else {
                "Ready"
            }
        }))
        .child(control("Switch language", false, move || {
            language.set_language(if language.language.get() == omasend::i18n::Language::En {
                omasend::i18n::Language::ZhCn
            } else {
                omasend::i18n::Language::En
            })
        }))
        .child(control("Preview message", false, move || {
            let id = previewer
                .state
                .borrow()
                .composer
                .iter()
                .find(|item| matches!(item.item, SendItem::Text(_)))
                .map(|item| item.id.clone());
            if let Some(id) = id {
                previewer.preview_item(&id);
            }
        }))
        .child(super::utilities::menu_button(session.clone()))
        .child(panels::content(session.clone()))
        .child(super::preview::overlay(session.clone()))
        .child(super::utilities::overlay(session, appearance))
}

struct Sink {
    tx: crossbeam_channel::Sender<MountBatch>,
    control: Arc<OnceLock<EngineControl>>,
}
impl MountSink for Sink {
    fn submit(&self, batch: MountBatch) {
        self.tx.send(batch).unwrap();
    }
    fn request_frame(&self) {
        let control = self.control.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(16));
            if let Some(control) = control.get() {
                control.frame();
            }
        });
    }
}

#[derive(Default)]
struct Screen {
    texts: HashMap<u32, String>,
    labels: HashMap<u32, String>,
    selected: HashMap<u32, bool>,
    deleted: Vec<u32>,
    inputs: HashMap<u32, nsh_core::InputConfig>,
    test_ids: HashMap<String, u32>,
    frames: HashMap<u32, TestRect>,
    modal_roots: std::collections::HashSet<u32>,
    parents: HashMap<u32, u32>,
    window_drag: HashMap<u32, Option<bool>>,
    desktop_commands: Vec<nsh_core::desktop::DesktopCommand>,
    keymap: Option<Arc<nsh_core::keymap::Keymap>>,
    timed_animations: usize,
    appearance: Option<nsh_core::platform::Appearance>,
    events: HashMap<u32, nsh_core::EventMask>,
}
impl Screen {
    fn apply(&mut self, batch: MountBatch) {
        for item in batch.items {
            match item {
                MountItem::Appearance { appearance } => self.appearance = Some(appearance),
                MountItem::Events { id, events } => {
                    self.events.insert(id, events);
                }
                MountItem::Create {
                    id,
                    kind: nsh_core::ViewKind::Modal,
                } => {
                    self.modal_roots.insert(id);
                }
                MountItem::Text { id, text, .. } => {
                    self.texts.insert(id, text.text.clone());
                }
                MountItem::Command {
                    command: nsh_core::Command::Desktop(command),
                    ..
                } => {
                    self.desktop_commands.push(*command);
                }
                MountItem::Animation(nsh_core::animation::AnimationOp::Start {
                    driver: nsh_core::animation::Driver::Timing { duration_ms, .. },
                    ..
                }) if duration_ms > 0.0 => self.timed_animations += 1,
                MountItem::Keymap(table) => self.keymap = Some(table),
                MountItem::Frame { id, frame } => {
                    self.frames.insert(
                        id,
                        TestRect {
                            x: frame.origin.x.0,
                            y: frame.origin.y.0,
                            width: frame.size.width.0,
                            height: frame.size.height.0,
                        },
                    );
                }
                MountItem::Insert { parent, child, .. } => {
                    self.parents.insert(child, parent);
                }
                MountItem::ViewProps { id, props } => {
                    self.window_drag.insert(id, props.window_drag);
                    self.selected.insert(id, props.accessibility.state.selected);
                    if let Some(test_id) = &props.test_id {
                        self.test_ids.insert(test_id.to_string(), id);
                    }
                    if let Some(label) = &props.accessibility.label {
                        self.labels.insert(id, label.to_string());
                    }
                }
                MountItem::AccessibilityLabel { id, label } => {
                    if let Some(label) = label {
                        self.labels.insert(id, label.to_string());
                    } else {
                        self.labels.remove(&id);
                    }
                }
                MountItem::Input { id, config } => {
                    self.inputs.insert(id, *config);
                }
                MountItem::Delete { id } => {
                    self.deleted.push(id);
                    self.texts.remove(&id);
                    self.labels.remove(&id);
                }
                _ => {}
            }
        }
    }
    fn wait(
        &mut self,
        rx: &crossbeam_channel::Receiver<MountBatch>,
        timeout: Duration,
        predicate: impl Fn(&Self) -> bool,
    ) {
        let deadline = Instant::now() + timeout;
        while !predicate(self) {
            assert!(Instant::now() < deadline, "timed out: {:?}", self.texts);
            if let Ok(batch) = rx.recv_timeout(Duration::from_millis(50)) {
                self.apply(batch);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_round_trip_preserves_bytes_previews_translations_and_controls() {
    let root = tempfile::tempdir().unwrap();
    let receiver_dir = root.path().join("receiver");
    let sender_dir = root.path().join("sender");
    std::fs::create_dir_all(&sender_dir).unwrap();
    let file = sender_dir.join("payload.bin");
    let bytes = b"native\0binary\xff\r\n";
    std::fs::write(&file, bytes).unwrap();
    let receiver = Node::start(NodeConfig {
        alias: "Receiver".into(),
        port: 0,
        downloads: receiver_dir.clone(),
        discovery: false,
    })
    .await
    .unwrap();
    let (identity_tx, identity_rx) = crossbeam_channel::bounded(1);
    *BOOTSTRAP.lock().unwrap() = Some(Bootstrap {
        config: NodeConfig {
            alias: "NSH Sender".into(),
            port: 0,
            downloads: sender_dir.clone(),
            discovery: false,
        },
        peer: receiver.device.clone(),
        file: file.clone(),
        identity: identity_tx,
    });
    let (tx, rx) = crossbeam_channel::unbounded();
    let control = Arc::new(OnceLock::new());
    let engine = Engine::start(
        EngineConfig {
            app: nsh_engine::App::Rust(nsh_core::ui::Root::of::<TestApp>()),
            viewport: Size::new(nsh_core::Points(840.0), nsh_core::Points(630.0)),
            trace: false,
            network: None,
            websocket: None,
            filesystem: None,
            data_dir: None,
            storage: None,
            clipboard: None,
            clock: Default::default(),
            pinned: Default::default(),
            assets: None,
            environment: Environment {
                os: nsh_core::platform::Os::MacOs,
                ..Environment::default()
            },
            header_heights: (nsh_core::Points(0.0), nsh_core::Points(0.0)),
            limits: Default::default(),
            ui_sets: Vec::new(),
            runtime: Some(tokio::runtime::Handle::current()),
        },
        || ApproxTextEngine,
        Sink {
            tx,
            control: control.clone(),
        },
    )
    .unwrap();
    control.set(engine.control()).ok().unwrap();
    let mut screen = Screen::default();
    // Node::start generates a fresh RSA-2048 identity on a worker. Debug
    // prime generation varies substantially; transfer deadlines stay short.
    screen.wait(&rx, Duration::from_secs(60), |s| {
        s.texts.values().any(|text| text == "Ready")
    });
    let send = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Send")
        .unwrap()
        .0;
    let draft = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Stable draft")
        .unwrap()
        .0;
    let settings = screen.press_target(screen.test_ids["app-menu"]);
    let press = |label: &str, screen: &Screen| {
        let id = *screen
            .labels
            .iter()
            .find(|(_, text)| *text == label)
            .unwrap_or_else(|| panic!("missing menu item {label}"))
            .0;
        engine.control().dispatch_event(
            screen.press_target(id),
            EventKind::Press,
            EventPayload::None,
        );
    };
    engine
        .control()
        .dispatch_event(settings, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|l| l == "Theme")
    });
    assert_eq!(
        screen.selected.get(&settings),
        Some(&true),
        "open menu trigger must stay active"
    );
    press("Theme", &screen);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|l| l == "Light")
    });
    press("Light", &screen);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        !s.labels.values().any(|l| l == "Theme")
    });
    assert_eq!(
        screen.selected.get(&settings),
        Some(&false),
        "closed menu trigger must return to ghost state"
    );
    let light_style = screen.inputs.get(&draft).unwrap().style;
    engine
        .control()
        .dispatch_event(settings, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|l| l == "Theme")
    });
    press("Theme", &screen);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|l| l == "Light ✓")
    });
    press("Dark", &screen);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        !s.labels.values().any(|l| l == "Theme")
    });
    assert_ne!(screen.inputs.get(&draft).unwrap().style, light_style);
    engine
        .control()
        .dispatch_event(settings, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|l| l == "About…")
    });
    press("About…", &screen);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.texts
            .values()
            .any(|t| t.contains("Native interface built with Native Shell"))
    });
    press("Close", &screen);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        !s.labels.values().any(|l| l == "Close")
    });
    let switch = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Switch language")
        .unwrap()
        .0;
    engine
        .control()
        .dispatch_event(switch, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.texts.values().any(|text| text == "附近设备")
    });
    engine
        .control()
        .dispatch_event(switch, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.texts.values().any(|text| text == "Nearby")
    });
    let preview = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Preview message")
        .unwrap()
        .0;
    engine
        .control()
        .dispatch_event(preview, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|label| label == "Close")
    });
    let close = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Close")
        .unwrap()
        .0;
    engine
        .control()
        .dispatch_event(close, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        !s.labels.values().any(|label| label == "Close")
    });
    engine
        .control()
        .dispatch_event(send, EventKind::Press, EventPayload::None);
    let receive_id = loop {
        let event = tokio::time::timeout(Duration::from_secs(15), receiver.events.recv())
            .await
            .unwrap()
            .unwrap();
        if let TransferEvent::IncomingRequest { id, files, .. } = event {
            assert_eq!(files.len(), 2);
            assert!(
                files
                    .iter()
                    .any(|file| file.name == "payload.bin" && file.size == bytes.len() as u64)
            );
            assert!(files.iter().any(|file| file.name == "Message.txt"));
            assert!(!receiver_dir.join("payload.bin").exists());
            screen.wait(&rx, Duration::from_secs(10), |s| {
                s.labels.values().filter(|label| *label == "Cancel").count() == 3
                    && !s.labels.values().any(|label| label == "Remove")
            });
            assert_eq!(
                screen
                    .labels
                    .values()
                    .filter(|label| *label == "Preview")
                    .count(),
                1,
                "ordinary files must not advertise an unavailable preview"
            );
            receiver.handle.decide(&id, true).unwrap();
            break id;
        }
    };
    loop {
        let event = tokio::time::timeout(Duration::from_secs(15), receiver.events.recv())
            .await
            .unwrap()
            .unwrap();
        match event {
            TransferEvent::Completed { id, .. } if id == receive_id => break,
            TransferEvent::Failed { error, .. } => panic!("{error}"),
            _ => {}
        }
    }
    assert_eq!(
        std::fs::read(receiver_dir.join("payload.bin")).unwrap(),
        bytes
    );
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.texts.values().any(|text| text.contains("Sent"))
    });
    assert!(
        !screen.deleted.contains(&send),
        "send control was remounted"
    );
    assert!(
        !screen.deleted.contains(&draft),
        "draft input was remounted"
    );
    assert_eq!(screen.labels.get(&draft).unwrap(), "Stable draft");
    let own = identity_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    receiver
        .handle
        .send(
            own.clone(),
            vec![omasend::localsend::Upload::file(file.clone(), "reply.bin".into()).unwrap()],
            None,
        )
        .unwrap();
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|label| label == "Accept")
    });
    assert!(
        !sender_dir.join("reply.bin").exists(),
        "receive bypassed confirmation"
    );
    let accept = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Accept")
        .unwrap()
        .0;
    engine
        .control()
        .dispatch_event(accept, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.texts.values().any(|text| text.contains("Received"))
    });
    assert_eq!(std::fs::read(sender_dir.join("reply.bin")).unwrap(), bytes);
    assert!(screen.labels.values().any(|label| label == "Open"));
    assert!(screen.labels.values().any(|label| label == "Show in Files"));
    // Dispose the real UI while a second request is awaiting confirmation.
    // Its root-owned event loop must drop the node, reject pending work and
    // release the listener even though no UI task remains to drain events.
    let pending = receiver
        .handle
        .send(
            own.clone(),
            vec![omasend::localsend::Upload::file(file, "unaccepted.bin".into()).unwrap()],
            None,
        )
        .unwrap();
    screen.wait(&rx, Duration::from_secs(15), |s| {
        s.labels.values().any(|label| label == "Accept")
    });
    drop(engine);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = receiver.events.recv().await.unwrap();
            if matches!(event, TransferEvent::Failed { id, .. } | TransferEvent::Cancelled { id } if id == pending) {
                break;
            }
        }
    }).await.expect("peer kept waiting after the NSH root was disposed");
    tokio::time::timeout(Duration::from_secs(5), async {
        while tokio::net::TcpStream::connect(("127.0.0.1", own.port))
            .await
            .is_ok()
        {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("LocalSend listener survived engine disposal");
    let rebound = tokio::net::TcpListener::bind(("127.0.0.1", own.port))
        .await
        .expect("disposed NSH node still owns its listening port");
    drop(rebound);
    assert!(!sender_dir.join("unaccepted.bin").exists());
    receiver.shutdown().await;
}

#[component]
fn LayoutApp() -> impl IntoElement {
    let appearance = super::appearance::install_for_test();
    super::shell(Session::new(), appearance)
}

impl Screen {
    fn press_target(&self, mut id: u32) -> u32 {
        loop {
            if self
                .events
                .get(&id)
                .is_some_and(|events| events.contains(EventKind::Press))
            {
                return id;
            }
            id = *self
                .parents
                .get(&id)
                .expect("a menu item has a press target");
        }
    }
    fn bounds(&self, test_id: &str) -> TestRect {
        let mut id = self.test_ids[test_id];
        let mut rect = self.frames[&id];
        while let Some(parent) = self.parents.get(&id) {
            if self.modal_roots.contains(parent) {
                break;
            }
            if let Some(frame) = self.frames.get(parent) {
                rect.x += frame.x;
                rect.y += frame.y;
            }
            id = *parent;
        }
        rect
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn main_window_preserves_original_regions_at_both_supported_sizes() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let control = Arc::new(OnceLock::new());
    let engine = Engine::start(
        EngineConfig {
            app: nsh_engine::App::Rust(nsh_core::ui::Root::of::<LayoutApp>()),
            viewport: Size::new(nsh_core::Points(840.0), nsh_core::Points(630.0)),
            trace: false,
            network: None,
            websocket: None,
            filesystem: None,
            data_dir: None,
            storage: None,
            clipboard: None,
            clock: Default::default(),
            pinned: Default::default(),
            assets: None,
            environment: Environment {
                os: nsh_core::platform::Os::MacOs,
                ..Environment::default()
            },
            header_heights: (nsh_core::Points(0.0), nsh_core::Points(0.0)),
            limits: Default::default(),
            ui_sets: Vec::new(),
            runtime: Some(tokio::runtime::Handle::current()),
        },
        || ApproxTextEngine,
        Sink {
            tx,
            control: control.clone(),
        },
    )
    .unwrap();
    control.set(engine.control()).ok().unwrap();
    let mut screen = Screen::default();
    for (width, height) in [(840.0, 630.0), (640.0, 520.0)] {
        engine
            .control()
            .resize(Size::new(nsh_core::Points(width), nsh_core::Points(height)));
        screen.wait(&rx, Duration::from_secs(10), |s| {
            [
                "app-header",
                "nearby-panel",
                "outbox-panel",
                "composer-area",
                "outbox-actions",
                "status-bar",
            ]
            .iter()
            .all(|name| {
                s.test_ids
                    .get(*name)
                    .is_some_and(|id| s.frames.contains_key(id))
            }) && (s.bounds("status-bar").y + s.bounds("status-bar").height - height).abs() < 0.1
        });
        let header = screen.bounds("app-header");
        let nearby = screen.bounds("nearby-panel");
        let composer = screen.bounds("composer-area");
        let actions = screen.bounds("outbox-actions");
        let status = screen.bounds("status-bar");
        assert!(screen.desktop_commands.iter().any(|command| matches!(command,
            nsh_core::desktop::DesktopCommand::SetTitleBar { style: nsh_core::desktop::TitleBarStyle::Hidden, height: Some(height) } if height.0 == 42.0)));
        assert_eq!(
            screen.window_drag[&screen.test_ids["app-header"]],
            Some(true)
        );
        assert_eq!(
            screen.window_drag[&screen.test_ids["app-menu"]],
            Some(false)
        );
        let menu = screen.bounds("app-menu");
        assert!(
            (menu.width - 28.0).abs() < 0.1 && (menu.height - 28.0).abs() < 0.1,
            "desktop menu must stay a compact ghost button: {menu:?}"
        );
        assert_eq!(header.y, 0.0);
        assert!((header.width - width).abs() < 0.1);
        assert!(nearby.y >= header.height);
        assert!(composer.y >= nearby.y + nearby.height);
        assert!(
            composer.height > 200.0,
            "Outbox must remain the main work area: {composer:?}"
        );
        assert!((composer.width - (width - 32.0)).abs() < 0.1);
        assert!(actions.y >= composer.y + composer.height);
        assert!(actions.y + actions.height <= status.y);
        assert!((status.y + status.height - height).abs() < 0.1);
        assert!(
            screen.inputs.is_empty(),
            "the original home has no permanent message editor"
        );
    }
    // The backends report the space actually occupied by traffic lights or
    // caption buttons. Rust must react to these changes, including fullscreen.
    for (left, right) in [(78.0, 0.0), (0.0, 138.0), (0.0, 0.0)] {
        engine.control().set_environment(Environment {
            title_bar: nsh_core::geometry::Edges {
                top: nsh_core::Points(42.0),
                left: nsh_core::Points(left),
                right: nsh_core::Points(right),
                bottom: nsh_core::Points(0.0),
            },
            ..Environment::default()
        });
        screen.wait(&rx, Duration::from_secs(10), |s| {
            let menu = s.bounds("app-menu");
            (menu.x + menu.width - (640.0 - 16.0 - right)).abs() < 0.1
                && (s.bounds("app-title").x - (16.0 + left + 24.0 + 8.0)).abs() < 0.1
        });
    }
    drop(engine);
}

static INSTALL_FIXTURE: Mutex<Option<super::services::Installer>> = Mutex::new(None);

#[component]
fn UpdateFixtureApp() -> impl IntoElement {
    super::appearance::install_for_test();
    let mut session = Session::new();
    session.installer = INSTALL_FIXTURE.lock().unwrap().take().unwrap();
    session
        .update
        .set(omasend::updates::UpdateState::Available {
            version: "v9.0.0".into(),
            url: String::new(),
        });
    let panel = signal(true);
    let content = session.clone();
    let status = session.clone();
    let draft = session.clone();
    let clear = session.clone();
    let errors = session.clone();
    view()
        .child(when(
            move || panel.get(),
            move || super::services::update_control(content.clone()),
        ))
        .child(control("Hide update panel", false, move || {
            panel.set(false)
        }))
        .child(control("Show update panel", false, move || panel.set(true)))
        .child(text(move || status.update_label()))
        .child(control("Queue draft", false, move || {
            draft.state.borrow_mut().composer.push(
                omasend::model::ComposerItem::new(SendItem::Text("unfinished".into())).unwrap(),
            );
            draft.changed();
        }))
        .child(control("Clear draft", false, move || {
            clear.state.borrow_mut().composer.clear();
            clear.changed();
        }))
        .child(text(move || {
            errors.revision.get();
            errors.state.borrow().error.clone().unwrap_or_default()
        }))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_progress_retry_and_restart_survive_closing_the_starting_surface() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let (advance, gate) = crossbeam_channel::bounded(1);
    let attempts = Arc::new(AtomicUsize::new(0));
    let calls = attempts.clone();
    *INSTALL_FIXTURE.lock().unwrap() = Some(Arc::new(move |version, progress| {
        assert_eq!(version, "v9.0.0");
        let attempt = calls.fetch_add(1, Ordering::SeqCst);
        progress(25, Some(100));
        gate.recv_timeout(Duration::from_secs(30)).unwrap();
        if attempt == 0 {
            anyhow::bail!("simulated verification failure");
        }
        progress(100, Some(100));
        Ok(())
    }));
    struct ResetRestart;
    impl Drop for ResetRestart {
        fn drop(&mut self) {
            omasend::updates::RESTART_REQUESTED.store(false, Ordering::SeqCst);
        }
    }
    let _reset = ResetRestart;
    let (tx, rx) = crossbeam_channel::unbounded();
    let control = Arc::new(OnceLock::new());
    let engine = Engine::start(
        EngineConfig {
            app: nsh_engine::App::Rust(nsh_core::ui::Root::of::<UpdateFixtureApp>()),
            viewport: Size::new(nsh_core::Points(840.0), nsh_core::Points(630.0)),
            trace: false,
            network: None,
            websocket: None,
            filesystem: None,
            data_dir: None,
            storage: None,
            clipboard: None,
            clock: Default::default(),
            pinned: Default::default(),
            assets: None,
            environment: Environment {
                os: nsh_core::platform::Os::MacOs,
                ..Environment::default()
            },
            header_heights: (nsh_core::Points(0.0), nsh_core::Points(0.0)),
            limits: Default::default(),
            ui_sets: Vec::new(),
            runtime: Some(tokio::runtime::Handle::current()),
        },
        || ApproxTextEngine,
        Sink {
            tx,
            control: control.clone(),
        },
    )
    .unwrap();
    control.set(engine.control()).ok().unwrap();
    let mut screen = Screen::default();
    fn press(engine: &Engine, screen: &Screen, label: &str) {
        let id = *screen
            .labels
            .iter()
            .find(|(_, value)| value.as_str() == label)
            .unwrap_or_else(|| panic!("missing {label}: {:?}", screen.labels))
            .0;
        engine.control().dispatch_event(
            screen.press_target(id),
            EventKind::Press,
            EventPayload::None,
        );
    }
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|t| t == "Install v9.0.0…")
    });
    press(&engine, &screen, "Install v9.0.0…");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.texts.values().any(|t| t.contains("25%"))
    });
    press(&engine, &screen, "Hide update panel");
    advance.send(()).unwrap();
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.texts.values().any(|t| t == "Update failed · Retry")
    });
    assert!(!omasend::updates::RESTART_REQUESTED.load(Ordering::SeqCst));
    press(&engine, &screen, "Show update panel");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|t| t == "Update failed · Retry")
    });
    press(&engine, &screen, "Update failed · Retry");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.texts.values().any(|t| t.contains("25%"))
    });
    advance.send(()).unwrap();
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|t| t == "Restart to update")
    });
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    press(&engine, &screen, "Queue draft");
    press(&engine, &screen, "Restart to update");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.texts
            .values()
            .any(|t| t == "Finish transfers and clear Outbox before restarting")
    });
    assert!(
        !screen
            .desktop_commands
            .iter()
            .any(|c| matches!(c, nsh_core::desktop::DesktopCommand::Close))
    );
    assert!(!omasend::updates::RESTART_REQUESTED.load(Ordering::SeqCst));
    press(&engine, &screen, "Clear draft");
    press(&engine, &screen, "Restart to update");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.desktop_commands
            .iter()
            .any(|c| matches!(c, nsh_core::desktop::DesktopCommand::Close))
    });
    assert!(omasend::updates::RESTART_REQUESTED.load(Ordering::SeqCst));
    drop(engine);
}

#[component]
fn NearbyFixtureApp() -> impl IntoElement {
    let appearance = super::appearance::install_for_test();
    let session = Session::new();
    {
        let mut state = session.state.borrow_mut();
        state.devices = (0..12)
            .map(|index| Device {
                fingerprint: format!("fixture-{index}"),
                alias: format!("Nearby {index}"),
                model: "Test phone".into(),
                host: "127.0.0.1".into(),
                port: 53317,
            })
            .collect();
        state.selected = Some("fixture-11".into());
    }
    super::shell(session, appearance)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nearby_expansion_preserves_selection_and_bounds_the_device_list() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let control = Arc::new(OnceLock::new());
    let engine = Engine::start(
        EngineConfig {
            app: nsh_engine::App::Rust(nsh_core::ui::Root::of::<NearbyFixtureApp>()),
            viewport: Size::new(nsh_core::Points(840.0), nsh_core::Points(630.0)),
            trace: false,
            network: None,
            websocket: None,
            filesystem: None,
            data_dir: None,
            storage: None,
            clipboard: None,
            clock: Default::default(),
            pinned: Default::default(),
            assets: None,
            environment: Environment {
                os: nsh_core::platform::Os::MacOs,
                ..Environment::default()
            },
            header_heights: (nsh_core::Points(0.0), nsh_core::Points(0.0)),
            limits: Default::default(),
            ui_sets: Vec::new(),
            runtime: Some(tokio::runtime::Handle::current()),
        },
        || ApproxTextEngine,
        Sink {
            tx,
            control: control.clone(),
        },
    )
    .unwrap();
    control.set(engine.control()).ok().unwrap();
    let mut screen = Screen::default();
    let count = |s: &Screen| {
        s.labels
            .values()
            .filter(|label| label.starts_with("Nearby "))
            .count()
    };
    screen.wait(&rx, Duration::from_secs(10), |s| {
        count(s) == 3 && s.labels.values().any(|l| l == "Nearby 11")
    });
    let selected = *screen
        .labels
        .iter()
        .find(|(_, l)| *l == "Nearby 11")
        .unwrap()
        .0;
    let collapsed_height = screen.bounds("nearby-panel").height;
    let toggle = screen.test_ids["toggle-nearby"];
    engine
        .control()
        .dispatch_event(toggle, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(10), |s| {
        count(s) == 12 && s.bounds("nearby-panel").height > collapsed_height
    });
    assert_eq!(screen.labels.get(&selected).unwrap(), "Nearby 11");
    assert!(screen.bounds("nearby-panel").height < 270.0);
    assert!(screen.bounds("composer-area").height > 150.0);
    engine
        .control()
        .resize(Size::new(nsh_core::Points(640.0), nsh_core::Points(520.0)));
    screen.wait(&rx, Duration::from_secs(10), |s| {
        (s.bounds("status-bar").y + s.bounds("status-bar").height - 520.0).abs() < 0.1
    });
    assert!(screen.bounds("nearby-panel").height < 270.0);
    engine
        .control()
        .dispatch_event(toggle, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(10), |s| count(s) == 2);
    assert_eq!(screen.labels.get(&selected).unwrap(), "Nearby 11");
    assert!(screen.bounds("composer-area").height > 200.0);
    let key = |s: &Screen, name: &str| {
        let stroke =
            nsh_core::keymap::Keystroke::parse(name, nsh_core::platform::Os::MacOs).unwrap();
        let table = s.keymap.as_ref().unwrap();
        assert_eq!(
            table.find(stroke, true, false),
            nsh_core::keymap::KeyMatch::None,
            "device navigation must leave text editing alone"
        );
        let nsh_core::keymap::KeyMatch::Fire(id) = table.find(stroke, false, false) else {
            panic!("missing navigation binding {name}");
        };
        engine.control().dispatch_event(
            nsh_core::ROOT_ID,
            EventKind::KeyCommand,
            EventPayload::Value(nsh_core::PayloadValue::Number(id as f64)),
        );
    };
    let press = |label: &str, screen: &Screen| {
        let id = *screen
            .labels
            .iter()
            .find(|(_, text)| *text == label)
            .unwrap()
            .0;
        engine.control().dispatch_event(
            screen.press_target(id),
            EventKind::Press,
            EventPayload::None,
        );
    };
    engine.control().dispatch_event(
        screen.press_target(screen.test_ids["app-menu"]),
        EventKind::Press,
        EventPayload::None,
    );
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|l| l == "About…")
    });
    let button = screen.bounds("app-menu");
    let popup = screen.bounds("app-menu-popup");
    assert!(
        popup.y >= button.y + button.height,
        "menu must open below its button: {popup:?} {button:?}"
    );
    assert!(popup.x >= 0.0 && popup.x + popup.width <= 640.1);
    assert!(
        (popup.x + popup.width - button.x - button.width).abs() <= 8.0,
        "menu must align with the button's trailing edge: {popup:?} {button:?}"
    );
    key(&screen, "left");
    press("About…", &screen);
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|l| l == "Close")
    });
    press("Close", &screen);
    screen.wait(&rx, Duration::from_secs(10), |s| {
        !s.labels.values().any(|l| l == "Close")
    });
    assert!(
        screen.labels.values().any(|l| l == "Nearby 11"),
        "menu shortcut changed selected device"
    );
    key(&screen, "right");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        !s.labels.values().any(|l| l == "Nearby 11")
    });
    key(&screen, "left");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|l| l == "Nearby 11")
    });
    engine.control().dispatch_event(
        screen.press_target(screen.test_ids["app-menu"]),
        EventKind::Press,
        EventPayload::None,
    );
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|l| l == "Theme")
    });
    press("Theme", &screen);
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|l| l == "Dark")
    });
    let submenu = screen.bounds("app-submenu-popup");
    let trigger = screen.bounds("menu-theme-trigger");
    assert!(
        (submenu.y - trigger.y).abs() <= 8.0,
        "submenu must align with its trigger row: {submenu:?} {trigger:?}"
    );
    assert!(submenu.x >= 0.0 && submenu.x + submenu.width <= 640.1);
    press("Dark", &screen);
    screen.wait(&rx, Duration::from_secs(10), |s| {
        !s.labels.values().any(|l| l == "Theme")
    });
    assert!(screen.labels.values().any(|l| l == "Nearby 11"));
}

#[component]
fn MotionFixtureApp() -> impl IntoElement {
    super::appearance::install_for_test();
    let session = Session::new();
    session.state.borrow_mut().discovering = true;
    view()
        .child(text(|| {
            if reduce_motion() {
                "Reduced motion"
            } else {
                "Full motion"
            }
        }))
        .child(super::motion::emblem(session, 64.0))
        .child(control("Motion button", false, || {}))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn motion_preferences_stop_pending_discovery_strokes_and_button_timing() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let control = Arc::new(OnceLock::new());
    let engine = Engine::start(
        EngineConfig {
            app: nsh_engine::App::Rust(nsh_core::ui::Root::of::<MotionFixtureApp>()),
            viewport: Size::new(nsh_core::Points(640.0), nsh_core::Points(520.0)),
            trace: false,
            network: None,
            websocket: None,
            filesystem: None,
            data_dir: None,
            storage: None,
            clipboard: None,
            clock: Default::default(),
            pinned: Default::default(),
            assets: None,
            environment: Environment {
                os: nsh_core::platform::Os::MacOs,
                ..Environment::default()
            },
            header_heights: (nsh_core::Points(0.0), nsh_core::Points(0.0)),
            limits: Default::default(),
            ui_sets: Vec::new(),
            runtime: Some(tokio::runtime::Handle::current()),
        },
        || ApproxTextEngine,
        Sink {
            tx,
            control: control.clone(),
        },
    )
    .unwrap();
    control.set(engine.control()).ok().unwrap();
    let mut screen = Screen::default();
    screen.wait(&rx, Duration::from_secs(10), |s| s.timed_animations > 0);
    engine.control().set_environment(Environment {
        reduce_motion: true,
        ..Environment::default()
    });
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.texts.values().any(|t| t == "Reduced motion")
    });
    let before = screen.timed_animations;
    let button = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Motion button")
        .unwrap()
        .0;
    engine
        .control()
        .dispatch_event(button, EventKind::PressDown, EventPayload::None);
    engine
        .control()
        .dispatch_event(button, EventKind::PressUp, EventPayload::None);
    // Cover all previously queued stroke timers, not just the first commit.
    let deadline = Instant::now() + Duration::from_millis(700);
    while Instant::now() < deadline {
        if let Ok(batch) = rx.recv_timeout(Duration::from_millis(25)) {
            screen.apply(batch);
        }
    }
    assert_eq!(
        screen.timed_animations, before,
        "motion continued after the preference changed"
    );
    engine.control().set_environment(Environment::default());
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.timed_animations > before
    });
}

#[component]
fn ProgressFixtureApp() -> impl IntoElement {
    super::appearance::install_for_test();
    let session = Session::new();
    {
        let mut state = session.state.borrow_mut();
        let empty = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        state.composer = vec![
            omasend::model::ComposerItem::new(SendItem::Text("a".repeat(100))).unwrap(),
            omasend::model::ComposerItem::new(SendItem::Text("b".repeat(200))).unwrap(),
            omasend::model::ComposerItem::new(SendItem::TemporaryFile {
                path: Arc::new(empty),
                mime: "application/octet-stream".into(),
            })
            .unwrap(),
        ];
        state.track_send("batch".into());
        state.apply(TransferEvent::Accepted { id: "batch".into() });
        state.apply(TransferEvent::Progress {
            id: "batch".into(),
            transferred: 150,
            total: 300,
        });
    }
    let advance = session.clone();
    let fail = session.clone();
    view()
        .child(panels::outbox(session))
        .child(control("Advance batch", false, move || {
            advance.state.borrow_mut().apply(TransferEvent::Progress {
                id: "batch".into(),
                transferred: 250,
                total: 300,
            });
            advance.changed();
        }))
        .child(control("Fail batch", false, move || {
            fail.state.borrow_mut().apply(TransferEvent::Failed {
                id: "batch".into(),
                error: "offline".into(),
            });
            fail.changed();
        }))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composer_progress_accounts_for_each_item_and_zero_length_files() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let control = Arc::new(OnceLock::new());
    let engine = Engine::start(
        EngineConfig {
            app: nsh_engine::App::Rust(nsh_core::ui::Root::of::<ProgressFixtureApp>()),
            viewport: Size::new(nsh_core::Points(840.0), nsh_core::Points(630.0)),
            trace: false,
            network: None,
            websocket: None,
            filesystem: None,
            data_dir: None,
            storage: None,
            clipboard: None,
            clock: Default::default(),
            pinned: Default::default(),
            assets: None,
            environment: Environment {
                os: nsh_core::platform::Os::MacOs,
                ..Environment::default()
            },
            header_heights: (nsh_core::Points(0.0), nsh_core::Points(0.0)),
            limits: Default::default(),
            ui_sets: Vec::new(),
            runtime: Some(tokio::runtime::Handle::current()),
        },
        || ApproxTextEngine,
        Sink {
            tx,
            control: control.clone(),
        },
    )
    .unwrap();
    control.set(engine.control()).ok().unwrap();
    let mut screen = Screen::default();
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.texts
            .values()
            .any(|t| t.starts_with("50 B / 200 B · 25% · "))
    });
    assert!(
        screen
            .texts
            .values()
            .any(|t| t.starts_with("100 B / 100 B · 100% · "))
    );
    assert!(
        screen
            .texts
            .values()
            .any(|t| t.starts_with("0 B / 0 B · 0% · "))
    );
    let advance = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Advance batch")
        .unwrap()
        .0;
    engine
        .control()
        .dispatch_event(advance, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.texts
            .values()
            .any(|t| t.starts_with("150 B / 200 B · 75% · "))
    });
    let fail = *screen
        .labels
        .iter()
        .find(|(_, label)| *label == "Fail batch")
        .unwrap()
        .0;
    engine
        .control()
        .dispatch_event(fail, EventKind::Press, EventPayload::None);
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().filter(|l| *l == "Remove").count() == 3
    });
    assert!(!screen.texts.values().any(|t| t.contains(" / ")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phone_layout_respects_safe_areas_and_menu_sublevels_have_a_back_action() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let control = Arc::new(OnceLock::new());
    let engine = Engine::start(
        EngineConfig {
            app: nsh_engine::App::Rust(nsh_core::ui::Root::of::<LayoutApp>()),
            viewport: Size::new(nsh_core::Points(402.0), nsh_core::Points(874.0)),
            trace: false,
            network: None,
            websocket: None,
            filesystem: None,
            data_dir: None,
            storage: None,
            clipboard: None,
            clock: Default::default(),
            pinned: Default::default(),
            assets: None,
            environment: Environment {
                os: nsh_core::platform::Os::Ios,
                idiom: nsh_core::platform::Idiom::Phone,
                safe_area: nsh_core::Edges {
                    top: nsh_core::Points(62.0),
                    bottom: nsh_core::Points(34.0),
                    ..Default::default()
                },
                ..Environment::default()
            },
            header_heights: (nsh_core::Points(0.0), nsh_core::Points(0.0)),
            limits: Default::default(),
            ui_sets: Vec::new(),
            runtime: Some(tokio::runtime::Handle::current()),
        },
        || ApproxTextEngine,
        Sink {
            tx,
            control: control.clone(),
        },
    )
    .unwrap();
    control.set(engine.control()).ok().unwrap();
    let mut screen = Screen::default();
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.test_ids.contains_key("app-menu")
    });
    let header = screen.bounds("app-header");
    // Header items are laid out by UIKit; their frames are local to its slots.
    assert!(header.width > 0.0);
    assert!(!screen.desktop_commands.iter().any(|command| matches!(
        command,
        nsh_core::desktop::DesktopCommand::SetTitleBar { .. }
    )));
    let outbox = screen.bounds("outbox-panel");
    assert!(outbox.y >= 62.0, "content under notch: {outbox:?}");
    let actions = screen.bounds("outbox-actions");
    assert!(actions.x >= 0.0 && actions.x + actions.width <= 402.1);
    let status = screen.bounds("status-bar");
    assert!(
        status.y + status.height <= 840.1,
        "status under home indicator: {status:?}"
    );
    let press = |screen: &Screen, label: &str| {
        let id = *screen
            .labels
            .iter()
            .find(|(_, value)| value.as_str() == label)
            .unwrap_or_else(|| panic!("missing {label}: {:?}", screen.labels))
            .0;
        engine.control().dispatch_event(
            screen.press_target(id),
            EventKind::Press,
            EventPayload::None,
        );
    };
    engine.control().dispatch_event(
        screen.press_target(screen.test_ids["app-menu"]),
        EventKind::Press,
        EventPayload::None,
    );
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|v| v == "Theme")
    });
    assert!(
        !screen
            .labels
            .values()
            .any(|v| v == "Exit" || v == "Check for update")
    );
    press(&screen, "Theme");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|v| v == "Back")
    });
    press(&screen, "Back");
    screen.wait(&rx, Duration::from_secs(10), |s| {
        s.labels.values().any(|v| v == "Language")
    });
}
