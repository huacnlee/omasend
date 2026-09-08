use super::Home;
use gpui_omarchy::gpui::{
    AnyElement, ClipboardItem, Context, ScrollHandle, Task, Window, div, prelude::*, rems,
};
use gpui_omarchy::{ActiveTheme, ButtonVariant, IconName, button, icon, sheet};

pub struct LogsPanel {
    pub(super) text: String,
    scroll: ScrollHandle,
    copied: bool,
    pub(super) refresh: Option<Task<()>>,
}

impl Home {
    pub fn open_logs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.logs.is_some() || self.state.incoming.is_some() {
            return;
        }
        if !self.history_expanded && self.preview.is_none() {
            self.restore_focus = window.focused(cx);
        }
        self.history_expanded = false;
        self.preview = None;
        let scroll = ScrollHandle::new();
        scroll.scroll_to_bottom();
        self.logs = Some(LogsPanel {
            text: omasend::diagnostics::snapshot(),
            scroll,
            copied: false,
            refresh: None,
        });
        self.modal_focus.focus(window, cx);
        let task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                let keep_polling = this.update(cx, |view, cx| {
                    let Some(logs) = &mut view.logs else {
                        return false;
                    };
                    let text = omasend::diagnostics::snapshot();
                    if text != logs.text {
                        let following = (logs.scroll.offset().y + logs.scroll.max_offset().y).abs()
                            <= gpui_omarchy::gpui::px(1.);
                        logs.text = text;
                        if following {
                            logs.scroll.scroll_to_bottom();
                        }
                        logs.copied = false;
                        cx.notify();
                    }
                    true
                });
                if !matches!(keep_polling, Ok(true)) {
                    break;
                }
            }
        });
        self.logs.as_mut().unwrap().refresh = Some(task);
        cx.notify();
    }

    pub fn close_logs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.logs = None;
        self.restore(window, cx);
        cx.notify();
    }

    pub fn logs_overlay(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let logs = self.logs.as_ref().expect("logs panel is open");
        let mut lines = div()
            .id("log-lines")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .overflow_x_scroll()
            .track_scroll(&logs.scroll)
            .px_4()
            .py_2()
            .flex()
            .flex_col()
            .text_size(rems(0.6875));
        if logs.text.is_empty() {
            lines = lines.child(
                div()
                    .text_color(theme.secondary)
                    .child(self.language.text("No logs yet")),
            );
        } else {
            for line in logs.text.lines() {
                lines = lines.child(
                    div()
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .child(line.to_owned()),
                );
            }
        }
        let surface = div()
            .absolute()
            .bottom_0()
            .left_0()
            .w_full()
            .h(window.viewport_size().height * 0.6)
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.background)
            .text_color(theme.foreground)
            .occlude()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_shrink_0()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.divider())
                    .child(self.language.text("Logs"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                button(
                                    "copy-logs",
                                    self.language
                                        .text(if logs.copied { "Copied" } else { "Copy" }),
                                    ButtonVariant::Outline,
                                    cx,
                                )
                                .disabled(logs.text.is_empty())
                                .on_click(cx.listener(
                                    |view, _, _, cx| {
                                        if let Some(logs) = &mut view.logs {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                logs.text.clone(),
                                            ));
                                            logs.copied = true;
                                            cx.notify();
                                        }
                                    },
                                )),
                            )
                            .child(
                                button(
                                    "clear-logs",
                                    self.language.text("Clear"),
                                    ButtonVariant::Outline,
                                    cx,
                                )
                                .text_color(theme.danger)
                                .border_color(theme.danger)
                                .hover(|style| {
                                    style
                                        .bg(theme.danger.opacity(0.08))
                                        .border_color(theme.danger)
                                })
                                .focus_visible(|style| {
                                    style
                                        .bg(theme.danger.opacity(0.08))
                                        .border_color(theme.danger)
                                })
                                .active(|style| style.bg(theme.danger.opacity(0.14)))
                                .disabled(logs.text.is_empty())
                                .on_click(cx.listener(
                                    |view, _, _, cx| {
                                        omasend::diagnostics::clear();
                                        if let Some(logs) = &mut view.logs {
                                            logs.text.clear();
                                            logs.copied = false;
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                button("close-logs", "", ButtonVariant::Secondary, cx)
                                    .accessibility_label(self.language.text("Close"))
                                    .map(|button| {
                                        gpui_omarchy::with_tooltip(
                                            button,
                                            self.language.text("Close"),
                                        )
                                    })
                                    .p_1()
                                    .child(icon(IconName::Close).size(rems(0.875)))
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.close_logs(window, cx)
                                    })),
                            ),
                    ),
            )
            .child(lines);
        let view = cx.entity();
        sheet(&self.modal_focus, cx)
            .overlay(
                div()
                    .absolute()
                    .inset_0()
                    .bg(theme.background.opacity(0.18)),
            )
            .surface(surface)
            .request_close(move |window, cx| {
                cx.stop_propagation();
                view.update(cx, |view, cx| view.close_logs(window, cx));
            })
            .into_any_element()
    }
}
