use super::{Home, size_label};
use gpui_omarchy::gpui::{
    AnimationExt, AnyElement, Context, SharedString, Window, div, prelude::*, rems,
};
use gpui_omarchy::{ActiveTheme, ButtonVariant, IconName, button, icon, progress, sheet};
use omasend::model::{Transfer, TransferStatus};

impl Home {
    pub fn history_button(&self, cx: &mut Context<Self>) -> AnyElement {
        button("toggle-transfer-history", "", ButtonVariant::Secondary, cx)
            .accessibility_label(self.language.text("Transfer history"))
            .map(|button| {
                gpui_omarchy::with_tooltip(button, self.language.text("Transfer history"))
            })
            .disabled(self.state.transfers.is_empty())
            .child(icon(IconName::History).size(rems(0.875)))
            .on_click(cx.listener(|view, _, window, cx| view.open_history(window, cx)))
            .into_any_element()
    }

    pub fn open_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.history_expanded = true;
        self.restore_focus = window.focused(cx);
        self.history_scroll.scroll_to_bottom();
        self.modal_focus.focus(window, cx);
        cx.notify();
    }

    pub fn close_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.history_expanded = false;
        self.restore(window, cx);
        cx.notify();
    }

    /// An edge-attached sheet covers the composer; it never participates in its layout.
    /// The single variable-height scroller includes the newest transfer at the bottom.
    pub fn history_overlay(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let mut records = div()
            .id("history-records")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.history_scroll)
            .px_4();
        for transfer in &self.state.transfers {
            records = records.child(self.transfer_row(transfer, false, cx));
        }
        if self.state.transfers.is_empty() {
            records = records.child(
                div()
                    .py_3()
                    .text_color(theme.secondary)
                    .child(self.language.text("No transfers yet")),
            );
        }
        let surface = div()
            .id("history-dock")
            .absolute()
            .bottom_0()
            .left_0()
            .w_full()
            .h(window.viewport_size().height * 0.55)
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
                    .gap_2()
                    .flex_shrink_0()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.divider())
                    .child(div().flex_1().child(self.language.text("Transfer history")))
                    .child(
                        button(
                            "clear-transfer-history",
                            self.language.text("Clear"),
                            ButtonVariant::Danger,
                            cx,
                        )
                        .accessibility_label(self.language.text("Clear transfer history"))
                        .disabled(
                            !self
                                .state
                                .transfers
                                .iter()
                                .any(|transfer| transfer.status != TransferStatus::Active),
                        )
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.state.clear_transfer_history();
                            view.history_scroll.scroll_to_bottom();
                            view.modal_focus.focus(window, cx);
                            cx.notify();
                        })),
                    )
                    .child(
                        button("close-transfer-history", "", ButtonVariant::Secondary, cx)
                            .accessibility_label(self.language.text("Close"))
                            .map(|button| {
                                gpui_omarchy::with_tooltip(button, self.language.text("Close"))
                            })
                            .p_1()
                            .child(icon(IconName::Close).size(rems(0.875)))
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.close_history(window, cx);
                            })),
                    ),
            )
            .child(records)
            .with_animation(
                "history-dock-enter",
                super::motion::popup_enter(),
                |surface, phase| surface.bottom(rems(-0.75 * (1. - phase))).opacity(phase),
            );
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
                // Sheet propagates its Cancel action by default. Consume this
                // dismissal so Home's Escape fallback cannot overwrite focus.
                cx.stop_propagation();
                view.update(cx, |view, cx| view.close_history(window, cx));
            })
            .into_any_element()
    }

    pub fn transfer_progress(
        &self,
        transfer: &Transfer,
        key: &str,
        item_range: Option<(u64, u64)>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let theme = cx.omarchy();
        let (transferred, total) = item_range
            .map(|(offset, size)| (transfer.transferred.saturating_sub(offset).min(size), size))
            .unwrap_or((transfer.transferred, transfer.total));
        let speed = if item_range.is_some_and(|(offset, size)| {
            transfer.transferred < offset || transfer.transferred >= offset.saturating_add(size)
        }) {
            0
        } else {
            transfer.bytes_per_second()
        };
        let active = transfer.status == TransferStatus::Active;
        let mut row = div().flex().flex_col().gap_2();
        if active && !transfer.awaiting_acceptance {
            let percentage = if total == 0 {
                0.
            } else {
                transferred as f32 / total as f32 * 100.
            };
            row = row.child(progress(
                SharedString::from(format!("progress-{key}")),
                percentage,
                cx,
            ));
        }
        row = row.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(self.transfer_status(transfer, cx))
                .when(active && !transfer.awaiting_acceptance, |row| {
                    let percentage = if total == 0 {
                        0.
                    } else {
                        transferred as f64 / total as f64 * 100.
                    };
                    row.child(
                        div()
                            .text_size(rems(0.6875))
                            .text_color(theme.secondary)
                            .child(format!(
                                "{} / {} · {:.0}% · {}/s",
                                size_label(transferred),
                                size_label(total),
                                percentage,
                                size_label(speed),
                            )),
                    )
                }),
        );
        row.into_any_element()
    }

    fn transfer_status(&self, transfer: &Transfer, cx: &Context<Self>) -> AnyElement {
        let theme = cx.omarchy();
        let (status, color) = match &transfer.status {
            TransferStatus::Active if transfer.awaiting_acceptance => {
                ("Waiting for receiver", theme.secondary)
            }
            TransferStatus::Active if transfer.sending => ("Sending", theme.accent),
            TransferStatus::Active => ("Receiving", theme.accent),
            TransferStatus::Completed if transfer.sending => ("Sent", theme.success),
            TransferStatus::Completed => ("Received", theme.success),
            TransferStatus::Cancelled => ("Cancelled", theme.secondary),
            TransferStatus::Failed(_) => ("Failed", theme.danger),
        };
        let clock: chrono::DateTime<chrono::Local> = transfer.when.into();
        div()
            .flex()
            .items_center()
            .flex_shrink_0()
            .gap_3()
            .child(div().text_color(color).child(match &transfer.status {
                TransferStatus::Failed(error) => self.language.error(error),
                _ => self.language.text(status).to_owned(),
            }))
            .child(
                div()
                    .text_size(rems(0.6875))
                    .text_color(theme.secondary)
                    .child(clock.format("%H:%M").to_string()),
            )
            .into_any_element()
    }

    fn transfer_row(
        &self,
        transfer: &Transfer,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.omarchy().clone();
        let id = transfer.id.clone();
        let active = transfer.status == TransferStatus::Active;
        let mut row = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .py_3()
            .border_b_1()
            .border_color(theme.divider())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_color(theme.bright)
                            .child(
                                transfer
                                    .files
                                    .iter()
                                    .map(|file| file.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                    )
                    .when(active, |row| {
                        row.child(
                            button(
                                SharedString::from(format!("cancel-{id}")),
                                self.language.text("Cancel"),
                                ButtonVariant::Secondary,
                                cx,
                            )
                            .on_click(cx.listener(
                                move |view, _, _, cx| {
                                    if let Some(node) = &view.node
                                        && let Err(error) = node.handle.cancel(&id)
                                    {
                                        view.state.error = Some(error.to_string());
                                    }
                                    cx.notify();
                                },
                            )),
                        )
                    }),
            )
            .when(!compact, |row| {
                row.child(div().text_color(theme.secondary).child(format!(
                    "{} · {}",
                    self.language.named(
                        if transfer.sending {
                            "To {name}"
                        } else {
                            "From {name}"
                        },
                        &transfer.peer
                    ),
                    size_label(transfer.total),
                )))
            });
        row = row.child(self.transfer_progress(transfer, &transfer.id, None, cx));
        for (index, path) in transfer.paths.iter().enumerate() {
            let open_path = path.clone();
            let reveal_path = path.clone();
            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(
                        transfer.paths.len() > 1
                            || transfer.files.first().is_some_and(|file| {
                                path.file_name().unwrap_or_default().to_string_lossy() != file.name
                            }),
                        |row| {
                            row.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_ellipsis()
                                    .overflow_hidden()
                                    .child(
                                        path.file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .into_owned(),
                                    ),
                            )
                        },
                    )
                    .child(
                        button(
                            SharedString::from(format!("open-{}-{index}", transfer.id)),
                            self.language.text("Open"),
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .on_click(move |_, _, cx| cx.open_with_system(&open_path)),
                    )
                    .child(
                        button(
                            SharedString::from(format!("reveal-{}-{index}", transfer.id)),
                            self.language.text("Show in Files"),
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .on_click(move |_, _, cx| cx.reveal_path(&reveal_path)),
                    ),
            );
        }
        row.with_animation(
            SharedString::from(format!("transfer-enter-{}", transfer.id)),
            super::motion::content_enter(),
            |row, phase| row.opacity(phase),
        )
        .into_any_element()
    }
}
