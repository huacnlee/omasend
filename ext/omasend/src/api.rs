//! The state and actions the TSX interface reads from `omasend/native`.
//!
//! `Omasend.snapshot()` returns plain data; `Omasend.onChange(listener)`
//! reports each change so the interface reads a fresh snapshot. Every action
//! goes through the app's one [`Session`].
use crate::session::Session;
use nsh_core::{
    element::{Runtime, create_root},
    prelude::*,
};
use omasend::{
    localsend::Device,
    model::{AppState, ComposerItem, SendItem, Transfer, TransferStatus},
};
use std::{cell::OnceCell, time::UNIX_EPOCH};

thread_local! {
    static SESSION: OnceCell<Session> = const { OnceCell::new() };
}

/// The reactive runtime of the session's signals. JS calls into this module
/// run on the engine thread outside any Rust runtime, so every call (and every
/// session task, [`entering`]) enters this one.
pub(crate) fn runtime() -> &'static Runtime {
    thread_local! {
        static RUNTIME: &'static Runtime = Box::leak(Box::default());
    }
    RUNTIME.with(|runtime| *runtime)
}

type Task = std::pin::Pin<Box<dyn Future<Output = ()>>>;

thread_local! {
    /// Tasks queued for the driver, and the driver's waker once it waits.
    static QUEUE: std::cell::RefCell<(Vec<Task>, Option<std::task::Waker>)> =
        const { std::cell::RefCell::new((Vec::new(), None)) };
    static DRIVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `future` under `owner` with [`runtime`] entered, polled by the
/// driver [`drive`] starts. JS calls into this module have no core executor
/// (NSH's `spawn` panics there); the core runs the driver as one task.
pub(crate) fn queue(owner: Owner, future: impl Future<Output = ()> + 'static) {
    let mut future = Box::pin(future);
    let task: Task = Box::pin(std::future::poll_fn(move |cx| {
        let _entered = runtime().enter();
        owner.run(|| future.as_mut().poll(cx))
    }));
    let waker = QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        queue.0.push(task);
        queue.1.take()
    });
    if let Some(waker) = waker {
        waker.wake();
    }
}

/// Starts the driver of [`queue`]d tasks as a core task, once.
fn drive(cx: CallCx<'_>) {
    if DRIVING.with(|driving| driving.replace(true)) {
        return;
    }
    use futures_util::{StreamExt, stream::FuturesUnordered};
    let mut tasks = FuturesUnordered::<Task>::new();
    cx.start(std::future::poll_fn(move |cx| {
        loop {
            let queued = QUEUE.with(|queue| std::mem::take(&mut queue.borrow_mut().0));
            let added = !queued.is_empty();
            tasks.extend(queued);
            match tasks.poll_next_unpin(cx) {
                std::task::Poll::Ready(Some(())) => continue,
                _ if added => continue,
                _ => {}
            }
            // Register before the final check, so a task queued meanwhile wakes us.
            let empty = QUEUE.with(|queue| {
                let mut queue = queue.borrow_mut();
                queue.1 = Some(cx.waker().clone());
                queue.0.is_empty()
            });
            if empty {
                return std::task::Poll::<()>::Pending;
            }
        }
    }));
}

/// Waits `duration` on a thread of its own: the core's timers are out of
/// reach from module calls.
pub(crate) async fn sleep(duration: std::time::Duration) {
    let (tx, rx) = async_channel::bounded::<()>(1);
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        let _ = tx.try_send(());
    });
    let _ = rx.recv().await;
}

/// The app's one session, started on first use.
fn session() -> Session {
    SESSION.with(|cell| {
        cell.get_or_init(|| {
            let (_, session) = create_root(|| {
                let session = Session::persistent();
                session.start();
                session.start_services();
                session
            });
            session
        })
        .clone()
    })
}

/// A LocalSend device.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct DeviceInfo {
    /// The device's fingerprint, its id.
    pub id: String,
    /// The name its owner gave it.
    pub alias: String,
    /// Its model, empty when unknown.
    pub model: String,
    /// Its address.
    pub host: String,
}

/// An item waiting to be sent.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct ComposerInfo {
    /// The item's id.
    pub id: String,
    /// "text", "file", "folder", "image" or "video".
    pub kind: String,
    /// The file name, or the text's first line.
    pub name: String,
    /// Its size in bytes.
    pub size: f64,
    /// Its path, empty for text.
    pub path: String,
    /// The id of the transfer sending it, empty when none.
    pub transfer: String,
}

/// A transfer, live or from the history.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct TransferInfo {
    /// The transfer's id.
    pub id: String,
    /// The other device's name.
    pub peer: String,
    /// Whether this device sends.
    pub sending: bool,
    /// Whether it waits for the receiver to accept.
    pub awaiting_acceptance: bool,
    /// "active", "completed", "cancelled" or "failed".
    pub status: String,
    /// Why it failed, empty otherwise.
    pub error: String,
    /// The file names.
    pub files: Vec<String>,
    /// The bytes moved so far.
    pub transferred: f64,
    /// The bytes in total.
    pub total: f64,
    /// The current rate in bytes per second.
    pub bytes_per_second: f64,
    /// Where received files were saved.
    pub paths: Vec<String>,
    /// The received text, empty when none.
    pub text: String,
    /// When it started, in milliseconds since the Unix epoch.
    pub when: f64,
}

/// A request to receive files, waiting for an answer.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct IncomingInfo {
    /// The request's id, for `decide`.
    pub id: String,
    /// The sender.
    pub peer: DeviceInfo,
    /// The offered file names.
    pub files: Vec<String>,
    /// Their size in bytes.
    pub total: f64,
}

/// Everything the interface shows.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct Snapshot {
    /// Whether the app should quit now (after stopping LocalSend).
    pub quit: bool,
    /// This device, once LocalSend runs.
    pub identity: Option<DeviceInfo>,
    /// Whether LocalSend runs.
    pub running: bool,
    /// Whether discovery is scanning.
    pub discovering: bool,
    /// The devices found.
    pub devices: Vec<DeviceInfo>,
    /// The selected device's id, empty when none.
    pub selected: String,
    /// The items to send.
    pub composer: Vec<ComposerInfo>,
    /// The transfers, oldest first.
    pub transfers: Vec<TransferInfo>,
    /// The request waiting for an answer.
    pub incoming: Option<IncomingInfo>,
    /// The last error, empty when none.
    pub error: String,
    /// Whether sending is possible now.
    pub can_send: bool,
    /// The interface language code ("en", "zh-CN").
    pub language: String,
    /// The update status line, empty when there is nothing to say.
    pub update: String,
    /// Whether `activateUpdate` has something to do (install or restart).
    pub update_actionable: bool,
    /// Whether an update check or install is under way.
    pub update_busy: bool,
    /// Whether a network diagnosis runs.
    pub diagnosing: bool,
    /// The diagnosis step running now (a text key), empty otherwise.
    pub diagnosis_step: String,
    /// The last diagnosis report as Markdown, empty before the first run.
    pub diagnosis: String,
}

fn device(device: &Device) -> DeviceInfo {
    DeviceInfo {
        id: device.fingerprint.clone(),
        alias: device.alias.clone(),
        model: device.model.clone(),
        host: device.host.clone(),
    }
}

fn composer(item: &ComposerItem, transfer: Option<String>) -> ComposerInfo {
    let kind = match &item.item {
        SendItem::Text(_) => "text",
        _ if item.is_image() => "image",
        _ if item.is_video() => "video",
        _ if item.item.path().is_some_and(|path| path.is_dir()) => "folder",
        _ => "file",
    };
    ComposerInfo {
        id: item.id.clone(),
        kind: kind.into(),
        name: item.name(),
        size: item.size() as f64,
        path: item
            .item
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        transfer: transfer.unwrap_or_default(),
    }
}

fn transfer(transfer: &Transfer) -> TransferInfo {
    let (status, error) = match &transfer.status {
        TransferStatus::Active => ("active", String::new()),
        TransferStatus::Completed => ("completed", String::new()),
        TransferStatus::Cancelled => ("cancelled", String::new()),
        TransferStatus::Failed(error) => ("failed", error.clone()),
    };
    TransferInfo {
        id: transfer.id.clone(),
        peer: transfer.peer.clone(),
        sending: transfer.sending,
        awaiting_acceptance: transfer.awaiting_acceptance,
        status: status.into(),
        error,
        files: transfer
            .files
            .iter()
            .map(|file| file.name.clone())
            .collect(),
        transferred: transfer.transferred as f64,
        total: transfer.total as f64,
        bytes_per_second: transfer.bytes_per_second() as f64,
        paths: transfer
            .paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        text: transfer.received_text.clone().unwrap_or_default(),
        when: transfer
            .when
            .duration_since(UNIX_EPOCH)
            .map_or(0., |when| when.as_millis() as f64),
    }
}

/// The snapshot of `state`; `transfer_of` names the transfer sending a
/// composer item.
pub(crate) fn snapshot_of(
    state: &AppState,
    identity: Option<&Device>,
    running: bool,
    can_send: bool,
    language: &str,
    transfer_of: impl Fn(&str) -> Option<String>,
) -> Snapshot {
    Snapshot {
        identity: identity.map(device),
        running,
        discovering: state.discovering,
        devices: state.devices.iter().map(device).collect(),
        selected: state.selected.clone().unwrap_or_default(),
        composer: state
            .composer
            .iter()
            .map(|item| composer(item, transfer_of(&item.id)))
            .collect(),
        transfers: state.transfers.iter().map(transfer).collect(),
        incoming: state.incoming.as_ref().map(|request| IncomingInfo {
            id: request.id.clone(),
            peer: device(&request.peer),
            files: request.files.iter().map(|file| file.name.clone()).collect(),
            total: request.files.iter().map(|file| file.size as f64).sum(),
        }),
        error: state.error.clone().unwrap_or_default(),
        can_send,
        language: language.into(),
        ..Default::default()
    }
}

/// Omasend's LocalSend client: its state and what the interface can do.
#[module]
pub struct Omasend;

#[module]
impl Omasend {
    /// Everything the interface shows now. Starts LocalSend on first use.
    pub fn snapshot(cx: CallCx) -> Snapshot {
        drive(cx);
        let _entered = runtime().enter();
        let session = session();
        let identity = session.identity.get_untracked();
        let running = session.handle.get_untracked().is_some();
        let can_send = !session.cannot_send();
        let language = session.language.get_untracked().code();
        let update = session.update.get_untracked();
        let update_busy = matches!(
            update,
            omasend::updates::UpdateState::Checking
                | omasend::updates::UpdateState::Installing { .. }
        );
        let update_actionable = session.update_action_available();
        let update = if update_actionable || session.update_visible.get_untracked() {
            session.update_label()
        } else {
            String::new()
        };
        let diagnostics = session.diagnostics;
        let diagnosis = diagnostics
            .report
            .get_untracked()
            .map(|report| report.to_markdown(session.language.get_untracked()))
            .unwrap_or_default();
        let state = session.state.borrow();
        Snapshot {
            quit: session.quit.get_untracked(),
            update,
            update_actionable,
            update_busy,
            diagnosing: diagnostics.running.get_untracked(),
            diagnosis_step: diagnostics.step.get_untracked().unwrap_or_default().into(),
            diagnosis,
            ..snapshot_of(
                &state,
                identity.as_ref(),
                running,
                can_send,
                language,
                |id| session.item_transfer(id),
            )
        }
    }

    /// Calls `listener` after each change to the snapshot.
    pub fn on_change(listener: Callback, cx: CallCx) {
        drive(cx);
        let _entered = runtime().enter();
        let session = session();
        let revision = session.revision;
        let identity = session.identity;
        let handle = session.handle;
        let language = session.language;
        let busy = session.busy;
        let quit = session.quit;
        let update = session.update;
        let update_visible = session.update_visible;
        let diagnostics = session.diagnostics;
        session.owner.run(|| {
            effect(move || {
                update.get();
                update_visible.get();
                diagnostics.running.get();
                diagnostics.step.get();
                diagnostics.report.get();
                revision.get();
                identity.get();
                handle.get();
                language.get();
                busy.get();
                quit.get();
                listener.post(());
            });
        });
    }

    /// Selects the device with id `id`.
    pub fn select(id: String) {
        let _entered = runtime().enter();
        session().select(id);
    }

    /// Moves the selection by `delta` devices.
    pub fn navigate(delta: i32) {
        let _entered = runtime().enter();
        session().navigate(delta as isize);
    }

    /// Sends the items to the selected device.
    pub fn send() {
        let _entered = runtime().enter();
        session().send();
    }

    /// Accepts or declines the incoming request `id`.
    pub fn decide(id: String, accept: bool) {
        let _entered = runtime().enter();
        session().decide(&id, accept);
    }

    /// Cancels the transfer `id`.
    pub fn cancel(id: String) {
        let _entered = runtime().enter();
        session().cancel(&id);
    }

    /// Removes the item `id` from the items to send.
    pub fn remove(id: String) {
        let _entered = runtime().enter();
        session().remove(&id);
    }

    /// Lets the user pick files, or folders, to send.
    pub fn pick(folders: bool) {
        let _entered = runtime().enter();
        session().pick(folders);
    }

    /// The theme mode: "system", "light" or "dark".
    pub fn theme() -> String {
        let _entered = runtime().enter();
        crate::session::theme::load().to_owned()
    }

    /// Saves the theme mode; the interface applies it.
    pub fn set_theme(mode: String) {
        let _entered = runtime().enter();
        if let Err(error) = crate::session::theme::save(&mode) {
            session().fail(error);
        }
    }

    /// Whether this is an Omarchy installation: the theme follows Omarchy's.
    pub fn omarchy() -> bool {
        let _entered = runtime().enter();
        crate::session::theme::omarchy()
    }

    /// Adds photos from the library (iOS and Android).
    pub fn pick_photos() {
        let _entered = runtime().enter();
        session().pick_photos();
    }

    /// Adds the clipboard's text, image or files to the items to send.
    pub fn paste() {
        let _entered = runtime().enter();
        session().paste();
    }

    /// Adds dropped files (`file://` URIs) to the items to send.
    pub fn drop_files(uris: Vec<String>) {
        let _entered = runtime().enter();
        session().drop_files(uris);
    }

    /// Clears the finished transfers.
    pub fn clear_history() {
        let _entered = runtime().enter();
        session().clear_history();
    }

    /// Opens a received file, or shows it in its folder when `reveal`.
    pub fn open_path(path: String, reveal: bool) {
        let _entered = runtime().enter();
        session().open_path(path.into(), reveal);
    }

    /// Copies the text received by transfer `id`.
    pub fn copy_received(id: String) {
        let _entered = runtime().enter();
        session().copy_received(&id);
    }

    /// Sets the interface language ("en", "zh-CN").
    pub fn set_language(code: String) {
        let _entered = runtime().enter();
        session().set_language(omasend::i18n::Language::from_locale(&code));
    }

    /// The text for `key` in the current language.
    pub fn text(key: String) -> String {
        let _entered = runtime().enter();
        session().language.get_untracked().text(&key).to_string()
    }

    /// Checks for a new release now and shows the result.
    pub fn check_updates() {
        let _entered = runtime().enter();
        session().check_updates();
    }

    /// Installs the available update, or restarts into the installed one.
    pub fn activate_update() {
        let _entered = runtime().enter();
        session().activate_update();
    }

    /// Runs the network diagnosis, optionally probing `peer` ("host" or
    /// "host:port"). Returns false, without running, when `peer` is invalid.
    pub fn diagnose(peer: String) -> bool {
        let _entered = runtime().enter();
        let session = session();
        session.diagnostics.peer.set(peer);
        session.run_diagnostics();
        !session.diagnostics.invalid_peer.get_untracked()
    }

    /// The recent log lines.
    pub fn logs() -> String {
        let _entered = runtime().enter();
        omasend::diagnostics::snapshot()
    }

    /// Clears the recent log lines.
    pub fn clear_logs() {
        let _entered = runtime().enter();
        omasend::diagnostics::clear();
    }

    /// Puts `text` on the clipboard.
    pub fn copy_text(text: String) {
        let _entered = runtime().enter();
        // Nothing waits for the clipboard's answer.
        #[allow(clippy::let_underscore_future)]
        let _ = set_clipboard(text);
    }

    /// Opens `url` in the browser.
    pub fn open_url(url: String) {
        let _entered = runtime().enter();
        session().open_url(url);
    }

    /// The app version and the LocalSend protocol line for About.
    pub fn about() -> Vec<String> {
        let _entered = runtime().enter();
        let language = session().language.get_untracked();
        vec![
            concat!("Omasend · v", env!("CARGO_PKG_VERSION")).into(),
            language.named(
                "about.protocol",
                localsend::model::discovery::PROTOCOL_VERSION_V2,
            ),
        ]
    }

    /// Stops LocalSend and quits.
    pub fn exit() {
        let _entered = runtime().enter();
        session().exit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omasend::localsend::OfferedFile;
    use omasend::model::IncomingRequest;
    use std::time::{Duration, SystemTime};

    fn peer() -> Device {
        Device {
            fingerprint: "fp".into(),
            alias: "Laptop".into(),
            model: "Linux".into(),
            host: "192.168.1.2".into(),
            port: 53317,
        }
    }

    fn file(name: &str, size: u64) -> OfferedFile {
        OfferedFile {
            name: name.into(),
            size,
            mime: "text/plain".into(),
        }
    }

    #[test]
    fn empty_state() {
        let snapshot = snapshot_of(&AppState::default(), None, false, false, "en", |_| None);
        assert_eq!(snapshot.identity, None);
        assert!(snapshot.devices.is_empty());
        assert_eq!(snapshot.selected, "");
        assert_eq!(snapshot.language, "en");
        assert!(!snapshot.running);
    }

    #[test]
    fn devices_selection_and_incoming() {
        let mut state = AppState::default();
        state.devices.push(peer());
        state.selected = Some("fp".into());
        state.error = Some("boom".into());
        state.incoming = Some(IncomingRequest {
            id: "req".into(),
            peer: peer(),
            files: vec![file("a.txt", 3), file("b.txt", 4)],
        });
        let snapshot = snapshot_of(&state, Some(&peer()), true, true, "zh-CN", |_| None);
        assert_eq!(snapshot.identity.unwrap().alias, "Laptop");
        assert_eq!(snapshot.devices[0].id, "fp");
        assert_eq!(snapshot.selected, "fp");
        assert_eq!(snapshot.error, "boom");
        let incoming = snapshot.incoming.unwrap();
        assert_eq!(incoming.id, "req");
        assert_eq!(incoming.files, ["a.txt", "b.txt"]);
        assert_eq!(incoming.total, 7.);
    }

    #[test]
    fn composer_items_and_transfers() {
        let mut state = AppState::default();
        let item = ComposerItem::new(SendItem::Text("hello\nworld".into())).unwrap();
        let id = item.id.clone();
        state.composer.push(item);
        let when = SystemTime::UNIX_EPOCH + Duration::from_millis(1_500);
        state.transfers.push(Transfer::restored(
            "t1".into(),
            "Laptop".into(),
            false,
            vec![file("a.txt", 3)],
            3,
            vec!["/tmp/a.txt".into()],
            TransferStatus::Failed("lost".into()),
            when,
        ));
        let snapshot = snapshot_of(&state, None, true, true, "en", |item| {
            (item == id).then(|| "t1".to_owned())
        });
        let item = &snapshot.composer[0];
        assert_eq!(item.kind, "text");
        assert_eq!(item.name, "hello");
        assert_eq!(item.transfer, "t1");
        assert_eq!(item.path, "");
        let transfer = &snapshot.transfers[0];
        assert_eq!(transfer.status, "failed");
        assert_eq!(transfer.error, "lost");
        assert_eq!(transfer.files, ["a.txt"]);
        assert_eq!(transfer.paths, ["/tmp/a.txt"]);
        assert_eq!(transfer.total, 3.);
        assert_eq!(transfer.when, 1_500.);
    }
}
