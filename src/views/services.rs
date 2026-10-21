//! Root-owned asynchronous services survive closing their menu or modal.
use super::{session::Session, utilities::Surface};
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use anyhow::Context;
use nsh_core::ui::prelude::*;
use omasend::updates::UpdateState;
use std::time::Duration;

impl Session {
    pub fn exit(&self) {
        self.stop_and_close(false);
    }

    fn stop_and_close(&self, restart: bool) {
        let handle = self.handle.get();
        let exit_application = self.exit_application.clone();
        self.busy.set(true);
        self.task(async move {
            if let Some(handle) = handle {
                handle.stop().await;
            }
            if restart {
                omasend::updates::RESTART_REQUESTED
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            exit_application();
        });
    }

    pub fn open_surface(&self, surface: Surface) {
        if self.state.borrow().incoming.is_some() {
            return;
        }
        self.preview.set(None);
        self.history_visible.set(false);
        if surface == Surface::Logs {
            self.refresh_logs();
        }
        self.surface.set(Some(surface));
    }

    pub fn open_history(&self) {
        if self.state.borrow().incoming.is_some() {
            return;
        }
        self.preview.set(None);
        self.surface.set(None);
        self.history_visible.set(true);
    }

    pub fn refresh_logs(&self) {
        self.logs.set(omasend::diagnostics::snapshot());
    }

    pub fn notify(&self, key: &'static str) {
        self.notice.set(Some(key));
        self.notice_revision.update(|revision| *revision += 1);
        let revision = self.notice_revision.get_untracked();
        let epoch = self.notice_revision;
        let notice = self.notice;
        self.owner.run(|| {
            set_timeout(Duration::from_secs(6), move || {
                if epoch.get_untracked() == revision {
                    notice.set(None);
                }
            });
        });
    }

    pub fn open_url(&self, url: String) {
        let this = self.clone();
        self.task(async move {
            #[cfg(any(target_os = "ios", target_os = "android"))]
            let result = super::mobile::open_url(&url).await;
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            let result = blocking(move || open::that(url))
                .await
                .context("Link action stopped")
                .and_then(|result| result.map_err(Into::into));
            match result {
                Ok(()) => {}
                error => this.fail(format!("Cannot open link: {error:?}")),
            }
        });
    }

    pub fn start_services(&self) {
        if cfg!(any(target_os = "ios", target_os = "android")) {
            return;
        }
        self.check_updates_background();
        let this = self.clone();
        self.task(async move {
            loop {
                timer(Duration::from_secs(6 * 60 * 60)).await;
                this.check_updates_background();
            }
        });
    }

    pub fn check_updates(&self) {
        self.update_visible.set(true);
        self.check_updates_background();
    }

    fn check_updates_background(&self) {
        if matches!(
            self.update.get(),
            UpdateState::Checking | UpdateState::Installing { .. } | UpdateState::Ready { .. }
        ) {
            return;
        }
        self.update.set(UpdateState::Checking);
        let this = self.clone();
        self.task(async move {
            this.update.set(match omasend::updates::check().await {
                Ok(state) => state,
                Err(error) => {
                    tracing::warn!(%error, "Update check failed");
                    UpdateState::Failed
                }
            });
        });
    }

    pub fn update_label(&self) -> String {
        let language = self.language.get();
        match self.update.get() {
            UpdateState::Idle => String::new(),
            UpdateState::Checking => language.text("update.checking"),
            UpdateState::Current => language.text("update.current"),
            UpdateState::NoRelease => language.text("update.none"),
            UpdateState::Failed => language.text("update.check_failed"),
            UpdateState::Available { version, .. } => language.named("update.install", &version),
            UpdateState::Installing { downloaded, total } => {
                if total.is_some_and(|total| total > 0 && downloaded >= total) {
                    return language.text("update.installing");
                }
                let label = language.text("update.downloading");
                total
                    .filter(|total| *total > 0)
                    .map(|total| format!("{label} {}%", downloaded.saturating_mul(100) / total))
                    .unwrap_or(label)
            }
            UpdateState::Ready { .. } => language.text("update.restart"),
            UpdateState::InstallFailed { .. } => language.text("update.failed_retry"),
        }
    }
}

pub(super) type InstallProgress = Box<dyn Fn(u64, Option<u64>) + Send + Sync>;
pub(super) type Installer =
    std::sync::Arc<dyn Fn(String, InstallProgress) -> anyhow::Result<()> + Send + Sync>;

impl Session {
    pub fn update_action_available(&self) -> bool {
        matches!(
            self.update.get(),
            UpdateState::Available { .. }
                | UpdateState::Ready { .. }
                | UpdateState::InstallFailed { .. }
        )
    }

    pub fn restart_blocked(&self) -> bool {
        self.revision.get();
        let state = self.state.borrow();
        restart_blocked_state(self.busy.get(), &state)
    }

    pub fn activate_update(&self) {
        match self.update.get() {
            UpdateState::Ready { .. } => {
                if self.restart_blocked() {
                    self.fail(self.language.get().text("update.finish_first"));
                    return;
                }
                self.stop_and_close(true);
            }
            UpdateState::Available { version, .. } | UpdateState::InstallFailed { version } => {
                self.install_update(version)
            }
            _ => {}
        }
    }

    fn install_update(&self, version: String) {
        self.update_visible.set(true);
        self.update.set(UpdateState::Installing {
            downloaded: 0,
            total: None,
        });
        let (tx, rx) = async_channel::bounded(1);
        let updates = self.update;
        self.task(async move {
            while let Ok((downloaded, total)) = rx.recv().await {
                if matches!(updates.get(), UpdateState::Installing { .. }) {
                    updates.set(UpdateState::Installing { downloaded, total });
                }
            }
        });
        let this = self.clone();
        let installer = self.installer.clone();
        self.task(async move {
            let target = version.clone();
            let result = blocking(move || {
                let last = std::sync::Mutex::new(None::<std::time::Instant>);
                installer(
                    target,
                    Box::new(move |downloaded, total| {
                        let mut last = last.lock().unwrap_or_else(|poison| poison.into_inner());
                        let finished = total.is_some_and(|total| downloaded >= total);
                        if (last.is_none_or(|last| last.elapsed() >= Duration::from_millis(100))
                            || finished)
                            && tx.try_send((downloaded, total)).is_ok()
                        {
                            *last = Some(std::time::Instant::now());
                        }
                    }),
                )
            })
            .await;
            this.update.set(match result {
                Ok(Ok(())) => UpdateState::Ready { version },
                error => {
                    tracing::error!(?error, "Could not install Omasend update");
                    UpdateState::InstallFailed { version }
                }
            });
        });
    }
}

pub(super) fn update_control(session: Session) -> impl IntoElement {
    let actionable = session.clone();
    let status = session.clone();
    let label = session.clone();
    let activate = session.clone();
    view()
        .flex_row()
        .child(when(
            move || actionable.update_action_available(),
            move || {
                let label = label.clone();
                let activate = activate.clone();
                super::plain_control(
                    move || label.update_label(),
                    false,
                    move || activate.activate_update(),
                )
                .py(px(0.0))
                .text_color(move || super::appearance::theme().primary())
            },
        ))
        .child(when(
            move || status.update_visible.get() && !status.update_action_available(),
            move || {
                let session = session.clone();
                text(move || session.update_label())
            },
        ))
}

fn restart_blocked_state(busy: bool, state: &omasend::model::AppState) -> bool {
    busy || state.incoming.is_some()
        || !state.composer.is_empty()
        || state
            .transfers
            .iter()
            .any(|t| t.status == omasend::model::TransferStatus::Active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omasend::model::{
        AppState, ComposerItem, IncomingRequest, SendItem, Transfer, TransferStatus,
    };

    #[test]
    fn restart_waits_for_preparation_offers_outbox_and_active_transfers() {
        let mut state = AppState::default();
        assert!(!restart_blocked_state(false, &state));
        assert!(restart_blocked_state(true, &state));
        state
            .composer
            .push(ComposerItem::new(SendItem::Text("unsent".into())).unwrap());
        assert!(restart_blocked_state(false, &state));
        state.composer.clear();
        state.incoming = Some(IncomingRequest {
            id: "pending".into(),
            peer: omasend::localsend::Device {
                fingerprint: "peer".into(),
                alias: "Peer".into(),
                model: String::new(),
                host: "127.0.0.1".into(),
                port: 0,
            },
            files: Vec::new(),
        });
        assert!(restart_blocked_state(false, &state));
        state.incoming = None;
        state.transfers.push(Transfer::restored(
            "active".into(),
            "Peer".into(),
            true,
            Vec::new(),
            1,
            Vec::new(),
            TransferStatus::Active,
            std::time::SystemTime::now(),
        ));
        assert!(restart_blocked_state(false, &state));
        for status in [
            TransferStatus::Completed,
            TransferStatus::Cancelled,
            TransferStatus::Failed("failed".into()),
        ] {
            state.transfers[0].status = status;
            assert!(!restart_blocked_state(false, &state));
        }
    }
}
