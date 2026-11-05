//! UI-thread state and commands shared by pointer and keyboard entry points.
use anyhow::{Context, Result};
use nsh_core::prelude::*;
use omasend::{
    localsend::{Device, Handle, Node, NodeConfig, TransferEvent},
    model::{AppState, ComposerItem, SendItem},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub(crate) struct Session {
    pub state: Rc<RefCell<AppState>>,
    pub revision: Signal<u64>,
    pub handle: Signal<Option<Handle>>,
    pub identity: Signal<Option<Device>>,
    pub busy: Signal<bool>,
    pub language: Signal<omasend::i18n::Language>,
    pub history_visible: Signal<bool>,
    persist: bool,
    pub preview: Signal<Option<super::preview::Preview>>,
    pub surface: Signal<Option<super::utilities::Surface>>,
    pub update: Signal<omasend::updates::UpdateState>,
    pub update_visible: Signal<bool>,
    pub installer: super::services::Installer,
    pub logs: Signal<String>,
    pub notice: Signal<Option<&'static str>>,
    pub notice_revision: Signal<u64>,
    pub owner: Owner,
    pub exit_application: Rc<dyn Fn()>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(AppState::default())),
            revision: signal(0),
            handle: signal_with(None, SignalEquals::Never),
            identity: signal_with(None, SignalEquals::Never),
            busy: signal(false),
            language: signal(omasend::i18n::Language::En),
            history_visible: signal(false),
            persist: false,
            preview: signal_with(None, SignalEquals::Never),
            surface: signal(None),
            update_visible: signal(false),
            installer: std::sync::Arc::new(|version, progress| {
                omasend::updates::install::install(&version, progress)
            }),
            update: signal(omasend::updates::UpdateState::Idle),
            logs: signal(String::new()),
            notice: signal(None),
            notice_revision: signal(0),
            owner: Owner::current(),
            exit_application: Rc::new(nsh_core::element::desktop::close_window),
        }
    }

    pub fn task<T: 'static>(&self, future: impl Future<Output = T> + 'static) -> Spawned<T> {
        self.owner.run(|| spawn(future))
    }

    pub fn persistent() -> Self {
        let mut session = Self::new();
        session.persist = true;
        #[cfg(target_os = "macos")]
        {
            // The standalone runner retains its window on ordinary close.
            session.exit_application = Rc::new(nsh_platform_macos::quit_app);
        }
        session.language.set(super::preferences::load_language());
        session.state.borrow_mut().transfers = omasend::model::history::load();
        session
    }

    pub fn translated(&self, key: &'static str) -> impl Fn() -> SharedString + use<> {
        let language = self.language;
        move || language.get().text(key).into()
    }

    pub fn set_language(&self, language: omasend::i18n::Language) {
        self.language.set(language);
        if self.persist
            && let Err(error) = super::preferences::save_language(language)
        {
            self.fail(error);
        }
    }

    pub fn visible_transfer_ids(&self) -> Vec<String> {
        self.revision.get();
        let history = self.history_visible.get();
        let state = self.state.borrow();
        let latest = state.transfers.last().map(|t| &t.id);
        state
            .transfers
            .iter()
            .rev()
            .filter(|t| {
                history
                    || t.status == omasend::model::TransferStatus::Active
                    || Some(&t.id) == latest
            })
            .map(|t| t.id.clone())
            .collect()
    }

    pub fn preview_item(&self, id: &str) {
        if self.input_blocked() {
            return;
        }
        let preview = self
            .state
            .borrow()
            .composer
            .iter()
            .find(|item| item.id == id)
            .and_then(|item| match &item.item {
                SendItem::Text(text) => Some(super::preview::Preview::Text(text.clone())),
                item_value if item.is_image() => {
                    Some(super::preview::Preview::Image(item_value.clone()))
                }
                _ => None,
            });
        self.preview.set(preview);
    }

    pub fn clear_history(&self) {
        content_transition();
        self.state.borrow_mut().clear_transfer_history();
        self.save_history();
        self.changed();
    }

    fn save_history(&self) {
        if self.persist {
            omasend::model::history::save(&self.state.borrow().transfers);
        }
    }

    pub fn open_path(&self, path: std::path::PathBuf, reveal: bool) {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        let _ = reveal;
        let this = self.clone();
        self.task(async move {
            #[cfg(any(target_os = "ios", target_os = "android"))]
            let result = super::mobile::share(&path).await;
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            let result = blocking(move || -> Result<()> {
                let path = path
                    .canonicalize()
                    .context("Received file no longer exists")?;
                if reveal {
                    #[cfg(target_os = "macos")]
                    {
                        let status = std::process::Command::new("open")
                            .arg("-R")
                            .arg(&path)
                            .status()?;
                        anyhow::ensure!(status.success(), "Cannot show the received file");
                    }
                    #[cfg(not(target_os = "macos"))]
                    open::that(path.parent().context("Missing file directory")?)?;
                } else {
                    open::that(path)?;
                }
                Ok(())
            })
            .await
            .context("File action stopped")
            .and_then(|result| result);
            if let Err(error) = result {
                this.fail(error);
            }
        });
    }

    pub fn copy_received(&self, id: &str) {
        if let Some(text) = self
            .state
            .borrow()
            .transfers
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.received_text.clone())
        {
            set_clipboard(text);
            self.notify("status.copied");
        }
    }

    pub fn changed(&self) {
        self.revision.update(|n| *n += 1);
    }

    pub fn start(&self) {
        self.start_with(NodeConfig::desktop());
    }

    pub fn start_with(&self, config: Result<NodeConfig>) {
        let this = self.clone();
        self.task(async move {
            let node = match config {
                Ok(config) => Node::start(config).await,
                Err(error) => Err(error),
            };
            match node {
                Ok(node) => {
                    this.identity.set(Some(node.device.clone()));
                    this.handle.set(Some(node.handle.clone()));
                    while let Ok(event) = node.events.recv().await {
                        if matches!(
                            event,
                            TransferEvent::DeviceFound(_)
                                | TransferEvent::DeviceLost(_)
                                | TransferEvent::IncomingRequest { .. }
                                | TransferEvent::Started { .. }
                        ) {
                            content_transition();
                        }
                        let finished = matches!(
                            event,
                            TransferEvent::Completed { .. }
                                | TransferEvent::Cancelled { .. }
                                | TransferEvent::Failed { .. }
                        );
                        if matches!(event, TransferEvent::IncomingRequest { .. }) {
                            this.preview.set(None);
                            this.surface.set(None);
                            this.history_visible.set(false);
                        }
                        this.state.borrow_mut().apply(event);
                        if finished {
                            this.save_history();
                        }
                        this.changed();
                    }
                    this.handle.set(None);
                    node.shutdown().await;
                }
                Err(error) => this.fail(error),
            }
        });
    }

    pub fn input_blocked(&self) -> bool {
        self.revision.get();
        self.busy.get()
            || self.preview.get().is_some()
            || self.surface.get().is_some()
            || self.history_visible.get()
            || self.state.borrow().incoming.is_some()
    }

    pub fn cannot_send(&self) -> bool {
        self.revision.get();
        let state = self.state.borrow();
        self.busy.get()
            || self.preview.get().is_some()
            || self.surface.get().is_some()
            || self.history_visible.get()
            || self.handle.get().is_none()
            || state.incoming.is_some()
            || state.sending()
            || state.composer.is_empty()
            || state.selected_device().is_none()
    }

    pub fn fail(&self, error: impl std::fmt::Display) {
        self.state.borrow_mut().error = Some(error.to_string());
        self.changed();
    }

    pub fn select(&self, id: String) {
        if self.input_blocked() {
            return;
        }
        self.state.borrow_mut().selected = Some(id);
        self.changed();
    }

    pub fn navigate(&self, delta: isize) {
        if self.input_blocked() {
            return;
        }
        content_transition();
        self.state.borrow_mut().navigate(delta);
        self.changed();
    }

    pub fn send(&self) {
        if self.input_blocked() {
            return;
        }
        let Some(handle) = self.handle.get() else {
            return;
        };
        let mut state = self.state.borrow_mut();
        if state.sending() || state.composer.is_empty() || state.incoming.is_some() {
            return;
        }
        let Some(peer) = state.selected_device().cloned() else {
            return;
        };
        let uploads = state
            .composer
            .iter()
            .flat_map(|item| item.uploads.clone())
            .collect();
        match handle.send(peer, uploads, None) {
            Ok(id) => {
                state.track_send(id);
                state.error = None;
            }
            Err(error) => state.error = Some(error.to_string()),
        }
        drop(state);
        self.changed();
    }

    pub fn decide(&self, id: &str, accept: bool) {
        let Some(handle) = self.handle.get() else {
            return;
        };
        match handle.decide(id, accept) {
            Ok(()) => {
                content_transition();
                self.state.borrow_mut().incoming = None;
                self.changed();
            }
            Err(error) => self.fail(error),
        }
    }

    pub fn cancel(&self, id: &str) {
        if let Some(handle) = self.handle.get()
            && let Err(error) = handle.cancel(id)
        {
            self.fail(error);
        }
    }

    pub fn item_transfer(&self, id: &str) -> Option<String> {
        self.revision.get();
        self.state
            .borrow()
            .transfers
            .iter()
            .rev()
            .find(|transfer| {
                transfer.status == omasend::model::TransferStatus::Active
                    && transfer.composer_ids.iter().any(|item| item == id)
            })
            .map(|transfer| transfer.id.clone())
    }

    pub fn remove(&self, id: &str) {
        if self.input_blocked() || self.item_transfer(id).is_some() {
            return;
        }
        content_transition();
        self.state
            .borrow_mut()
            .composer
            .retain(|item| item.id != id);
        self.changed();
    }

    /// One preparation at a time, with filesystem work off the engine thread.
    /// The source and its temporary files stay owned until uploads are prepared.
    pub fn input(&self, source: impl Future<Output = Result<Vec<SendItem>>> + 'static) {
        if self.input_blocked() {
            return;
        }
        self.busy.set(true);
        let this = self.clone();
        self.task(async move {
            let result = match source.await {
                Ok(items) => blocking(move || prepare(items))
                    .await
                    .context("File preparation stopped")
                    .and_then(|result| result),
                Err(error) => Err(error),
            };
            this.busy.set(false);
            match result {
                Ok(items) => {
                    if !items.is_empty() {
                        content_transition();
                    }
                    let mut state = this.state.borrow_mut();
                    state.composer.extend(items);
                    state.error = None;
                    drop(state);
                    this.changed();
                }
                Err(error) => this.fail(error),
            }
        });
    }

    pub fn drop_files(&self, uris: Vec<String>) {
        self.input(async move {
            uris.into_iter()
                .map(|uri| {
                    let uri = url::Url::parse(&uri).context("Invalid dropped file URL")?;
                    anyhow::ensure!(
                        uri.scheme() == "file"
                            && uri.host_str().is_none_or(|host| {
                                host.is_empty() || host.eq_ignore_ascii_case("localhost")
                            }),
                        "Dropped item must be a local file"
                    );
                    let path = uri
                        .to_file_path()
                        .map_err(|_| anyhow::anyhow!("Invalid dropped file path"))?;
                    anyhow::ensure!(path.is_absolute(), "Dropped file path must be absolute");
                    Ok(SendItem::File(path))
                })
                .collect()
        });
    }

    pub fn pick(&self, folders: bool) {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        self.input(super::mobile::pick(folders));
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        self.pick_desktop(folders);
    }

    pub fn pick_photos(&self) {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        self.input(super::mobile::photos());
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        self.fail("The photo library is available on iOS");
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    fn pick_desktop(&self, folders: bool) {
        let title = self.language.get().text("composer.add_to_omasend");
        self.input(async move {
            let dialog = rfd::AsyncFileDialog::new().set_title(title);
            let files = if folders {
                dialog.pick_folders().await
            } else {
                dialog.pick_files().await
            };
            Ok(files
                .unwrap_or_default()
                .into_iter()
                .map(|file| SendItem::File(file.path().to_owned()))
                .collect())
        });
    }

    pub fn paste(&self) {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        self.input(super::mobile::paste());
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        self.input(async {
            #[cfg(target_os = "linux")]
            if std::env::var_os("WAYLAND_DISPLAY").is_some() {
                return omasend::clipboard::wayland::paste().await;
            }
            blocking(super::inputs::paste)
                .await
                .context("Clipboard read stopped")?
        });
    }
}

fn content_transition() {
    if !reduce_motion() {
        animate_layout(nsh_core::LayoutAnimation::all(
            nsh_core::LayoutTransition::new(nsh_core::LayoutCurve::EaseOut, 140.0),
        ));
    }
}

fn prepare(items: Vec<SendItem>) -> Result<Vec<ComposerItem>> {
    items.into_iter().map(ComposerItem::new).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_failed_input_does_not_partially_add_a_batch() {
        let result = prepare(vec![
            SendItem::Text("keep".into()),
            SendItem::File("/nonexistent/omasend-input".into()),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn folder_and_text_use_the_existing_upload_pipeline() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().to_path_buf();
        std::fs::create_dir_all(directory.join("nested")).unwrap();
        std::fs::write(directory.join("nested/data.txt"), b"payload").unwrap();
        let items = prepare(vec![
            SendItem::File(directory.clone()),
            SendItem::Text("hello\nworld".into()),
        ])
        .unwrap();
        assert_eq!(items[0].uploads[0].offer.name, "nested/data.txt");
        assert_eq!(items[0].size(), 7);
        assert_eq!(items[1].size(), 11);
    }
}
