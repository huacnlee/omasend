use super::Home;
use gpui_omarchy::gpui::{Context, Window};
use omasend::updates::UpdateState;

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
        if self.update_state == UpdateState::Checking {
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
            UpdateState::Available { version, .. } => {
                self.language.named("Download {name}…", version)
            }
            UpdateState::Checking => self.language.text("Checking for updates…").into(),
            UpdateState::Current => self.language.text("Up to date").into(),
            UpdateState::NoRelease => self.language.text("No releases yet").into(),
            UpdateState::Failed => self.language.text("Update check failed").into(),
            UpdateState::Idle => String::new(),
        }
    }
    pub fn activate_update(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let UpdateState::Available { url, .. } = &self.update_state {
            cx.open_url(url);
        } else {
            self.check_updates(cx);
        }
    }
}
