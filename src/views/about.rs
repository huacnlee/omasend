use super::Home;
use gpui_kit::{AnyElement, Context, Window, div, prelude::*, px, rems};
use gpui_omarchy::{
    ActiveTheme, ButtonVariant, IconName, button, dialog, dialog_popup, dialog_title, icon,
};
use localsend::model::discovery::PROTOCOL_VERSION_V2;
use rust_i18n::t;

/// Omasend carries no protocol of its own: discovery, encryption and transfers
/// are LocalSend's. Name that plainly, and let the reader go read the source.
const LOCALSEND_URL: &str = "https://localsend.org";

impl Home {
    pub fn open_about(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.incoming.is_some() {
            return;
        }
        self.restore_focus = window.focused(cx);
        self.logs = None;
        self.preview = None;
        self.history_expanded = false;
        self.about_open = true;
        self.modal_focus.focus(window, cx);
        cx.notify();
    }
    pub fn close_about(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.about_open = false;
        self.restore(window, cx);
        cx.notify();
    }
    pub fn about_dialog(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        dialog(&self.modal_focus, cx)
            .popup(
                dialog_popup(cx)
                    .w(px(380.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(super::motion::logo(40., theme.accent))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap_1()
                                    .child(dialog_title("Omasend", cx))
                                    .child(
                                        div()
                                            .text_color(theme.secondary)
                                            .child(concat!("v", env!("CARGO_PKG_VERSION"))),
                                    ),
                            )
                            .child(
                                button("close-about", "", ButtonVariant::Secondary, cx)
                                    .accessibility_label(t!("action.close"))
                                    .p_1()
                                    .self_start()
                                    .child(icon(IconName::Close).size(rems(0.875)))
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.close_about(window, cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().child(t!("about.protocol", name = PROTOCOL_VERSION_V2)))
                            .child(div().text_color(theme.secondary).child(t!("about.credit")))
                            .child(
                                button(
                                    "about-localsend",
                                    "localsend.org",
                                    ButtonVariant::Outline,
                                    cx,
                                )
                                .self_start()
                                .on_click(cx.listener(|_, _, _, cx| cx.open_url(LOCALSEND_URL))),
                            ),
                    ),
            )
            .on_close(cx.listener(|view, _, window, cx| {
                cx.stop_propagation();
                view.close_about(window, cx);
            }))
            .into_any_element()
    }
}
