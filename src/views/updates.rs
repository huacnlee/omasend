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
    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.update_state == UpdateState::Checking {
            return;
        }
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
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub fn update_menu_label(&self) -> String {
        match &self.update_state {
            UpdateState::Available { version, .. } => {
                self.language.named("Download {name}…", version)
            }
            state => self
                .language
                .text(match state {
                    UpdateState::Checking => "Checking for updates…",
                    UpdateState::Current => "Up to date · Check again",
                    UpdateState::NoRelease => "No releases yet · Check again",
                    UpdateState::Failed => "Update check failed · Retry",
                    _ => "Check for updates…",
                })
                .into(),
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
