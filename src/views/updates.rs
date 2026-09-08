use super::Home;
use gpui_kit::{Context, Window};
use omasend::updates::UpdateState;
use rust_i18n::t;

impl Home {
    pub fn start_update_checks(&mut self, cx: &mut Context<Self>) {
        self.check_updates(cx);
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(6 * 60 * 60))
                    .await;
                if this.update(cx, |view, cx| view.check_updates(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }
    pub fn check_updates_manually(&mut self, cx: &mut Context<Self>) {
        self.show_update_status = true;
        self.check_updates(cx);
        cx.notify();
    }
    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.update_state,
            UpdateState::Checking | UpdateState::Installing { .. } | UpdateState::Ready { .. }
        ) {
            return;
        }
        // Cancel the previous dismissal so it cannot hide a newer check's result.
        self.update_status_dismiss = None;
        self.update_state = UpdateState::Checking;
        let task = self.runtime.spawn(omasend::updates::check());
        cx.spawn(async move |this, cx| {
            let state = match task.await {
                Ok(Ok(state)) => state,
                result => {
                    tracing::warn!(?result, "Update check failed");
                    UpdateState::Failed
                }
            };
            let _ = this.update(cx, |view, cx| {
                view.update_state = state;
                if view.show_update_status
                    && matches!(
                        view.update_state,
                        UpdateState::Current | UpdateState::NoRelease | UpdateState::Failed
                    )
                {
                    view.update_status_dismiss = Some(cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_secs(5))
                            .await;
                        let _ = this.update(cx, |view, cx| {
                            view.show_update_status = false;
                            cx.notify();
                        });
                    }));
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub fn update_status_label(&self) -> String {
        match &self.update_state {
            UpdateState::Available { version, .. } => t!("update.install", name = version).into(),
            UpdateState::Checking => t!("update.checking").into(),
            UpdateState::Current => t!("update.current").into(),
            UpdateState::NoRelease => t!("update.none").into(),
            UpdateState::Failed => t!("update.check_failed").into(),
            UpdateState::Installing { downloaded, total } => {
                if total.is_some_and(|total| total > 0 && *downloaded >= total) {
                    t!("update.installing").into()
                } else if let Some(total) = total.filter(|total| *total > 0) {
                    format!(
                        "{} {}%",
                        t!("update.downloading"),
                        downloaded.saturating_mul(100) / total
                    )
                } else {
                    t!("update.downloading").into()
                }
            }
            UpdateState::Ready { .. } => t!("update.restart").into(),
            UpdateState::InstallFailed { .. } => t!("update.failed_retry").into(),
            UpdateState::Idle => String::new(),
        }
    }
    pub fn update_action_available(&self) -> bool {
        matches!(
            self.update_state,
            UpdateState::Available { .. }
                | UpdateState::Ready { .. }
                | UpdateState::InstallFailed { .. }
        )
    }

    pub fn activate_update(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        match &self.update_state {
            UpdateState::Ready { .. } => {
                if self.loading_input
                    || self.state.incoming.is_some()
                    || self
                        .state
                        .transfers
                        .iter()
                        .any(|transfer| transfer.status == omasend::model::TransferStatus::Active)
                    || !self.state.composer.is_empty()
                {
                    self.state.error = Some(self.language.text("update.finish_first"));
                    cx.notify();
                    return;
                }
                omasend::updates::RESTART_REQUESTED
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                cx.quit();
            }
            UpdateState::Available { version, .. } | UpdateState::InstallFailed { version } => {
                let version = version.clone();
                self.install_update(version, cx);
            }
            _ => {}
        }
    }

    fn install_update(&mut self, version: String, cx: &mut Context<Self>) {
        self.update_status_dismiss = None;
        self.show_update_status = true;
        self.update_state = UpdateState::Installing {
            downloaded: 0,
            total: None,
        };
        let (sender, receiver) = async_channel::bounded(1);
        let target_version = version.clone();
        let task = self.runtime.spawn_blocking(move || {
            let last_progress = std::sync::Mutex::new(std::time::Instant::now());
            omasend::updates::install::install(&target_version, move |downloaded, total| {
                let mut last = last_progress.lock().unwrap();
                if (last.elapsed() >= std::time::Duration::from_millis(100)
                    || total.is_some_and(|total| downloaded >= total))
                    && sender.try_send((downloaded, total)).is_ok()
                {
                    *last = std::time::Instant::now();
                }
            })
        });
        cx.spawn(async move |this, cx| {
            while let Ok((downloaded, total)) = receiver.recv().await {
                if this
                    .update(cx, |view, cx| {
                        if matches!(view.update_state, UpdateState::Installing { .. }) {
                            view.update_state = UpdateState::Installing { downloaded, total };
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                view.update_state = match result {
                    Ok(Ok(())) => UpdateState::Ready { version },
                    error => {
                        tracing::error!(?error, "Could not install Omasend update");
                        UpdateState::InstallFailed { version }
                    }
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
