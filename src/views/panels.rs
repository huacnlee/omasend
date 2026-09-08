use super::{Home, size_label};
use gpui_omarchy::gpui::{
    AnimationExt, AnyElement, Context, ObjectFit, SharedString, div, img, prelude::*, px, rems,
};
use gpui_omarchy::{
    ActiveTheme, ButtonVariant, alert_dialog, button, dialog, dialog_popup, dialog_title, keycap,
    progress,
};
use omasend::model::{SendItem, TransferStatus};

impl Home {
    pub fn nearby(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let mut list = div()
            .id("nearby-list")
            .overflow_x_scroll()
            .flex()
            .gap_2()
            .min_w_0();
        if self.state.devices.is_empty() {
            list = list.child(
                div().py_3().text_color(theme.secondary).child(
                    self.language
                        .text("Looking for devices… Open LocalSend on the same Wi-Fi."),
                ),
            );
        }
        for device in &self.state.devices {
            let id = device.fingerprint.clone();
            let selected = self.state.selected.as_ref() == Some(&id);
            list = list.child(
                button(
                    gpui_omarchy::gpui::SharedString::from(format!("device-{id}")),
                    "",
                    ButtonVariant::Secondary,
                    cx,
                )
                .selected(selected)
                .w(rems(13.))
                .flex_shrink_0()
                .items_start()
                .gap_2()
                .p_3()
                .accessibility_label(self.language.named("Select {name}", &device.alias))
                .child(
                    div()
                        .text_color(if selected {
                            theme.accent
                        } else {
                            theme.secondary
                        })
                        .child(if selected { ">" } else { " " }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .gap_1()
                        .child(
                            div()
                                .text_ellipsis()
                                .overflow_hidden()
                                .child(device.alias.clone()),
                        )
                        .child(
                            div()
                                .text_size(rems(0.6875))
                                .text_color(theme.secondary)
                                .child(device.model.clone()),
                        ),
                )
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.state.selected = Some(id.clone());
                    view.focus.focus(window, cx);
                    cx.notify();
                })),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(theme.divider())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui_omarchy::gpui::FontWeight::BOLD)
                            .child(format!(
                                "{} ({})",
                                self.language.text("Nearby"),
                                self.state.devices.len()
                            )),
                    )
                    .child(
                        div()
                            .text_size(rems(0.6875))
                            .text_color(theme.secondary)
                            .child(
                                self.node
                                    .as_ref()
                                    .map(|node| {
                                        format!(
                                            "{}: {}",
                                            self.language.text("You"),
                                            node.device.alias
                                        )
                                    })
                                    .unwrap_or_else(|| self.language.text("Starting…").into()),
                            ),
                    ),
            )
            .child(list)
            .into_any_element()
    }

    pub fn composer(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let mut list = div()
            .id("composer-list")
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .flex_1()
            .min_h(rems(8.))
            .gap_3();
        if self.state.composer.is_empty() {
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .justify_center()
                    .items_center()
                    .gap_3()
                    .bg(theme.inset)
                    .p_4()
                    .child(
                        div()
                            .text_size(rems(1.75))
                            .text_color(theme.accent)
                            .child("↓"),
                    )
                    .child(
                        div()
                            .text_color(theme.bright)
                            .font_weight(gpui_omarchy::gpui::FontWeight::BOLD)
                            .child(self.language.text("Drop something here")),
                    )
                    .child(
                        div()
                            .text_color(theme.secondary)
                            .child(self.language.text("Files, folders, images or a few words.")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(keycap("ctrl+v", cx))
                            .child(self.language.text("Paste"))
                            .child(div().w(rems(0.5)))
                            .child(keycap("ctrl+o", cx))
                            .child(self.language.text("Add files")),
                    ),
            );
        }
        for item in &self.state.composer {
            let id = item.id.clone();
            let mut row = div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .border_1()
                .border_color(theme.border);
            if item.is_image() {
                if let Some(path) = item.item.path() {
                    row = row.child(
                        img(path.to_path_buf())
                            .w_full()
                            .h(rems(10.))
                            .object_fit(ObjectFit::Contain),
                    );
                }
            } else if item.is_video() {
                row = row.child(
                    div()
                        .text_size(rems(1.5))
                        .text_color(theme.secondary)
                        .child("▶"),
                );
            } else if let SendItem::Text(text) = &item.item {
                row = row.child(
                    div()
                        .text_color(theme.secondary)
                        .child(text.chars().take(240).collect::<String>()),
                );
            }
            let preview_id = id.clone();
            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(div().overflow_hidden().text_ellipsis().child(item.name()))
                            .child(div().text_color(theme.secondary).child(format!(
                                "{} · {}",
                                size_label(item.size()),
                                self.language.count(
                                    item.uploads.len(),
                                    "{count} file",
                                    "{count} files"
                                )
                            ))),
                    )
                    .when(item.is_image(), |row| {
                        row.child(
                            button(
                                gpui_omarchy::gpui::SharedString::from(format!("preview-{id}")),
                                self.language.text("Preview…"),
                                ButtonVariant::Secondary,
                                cx,
                            )
                            .on_click(cx.listener(
                                move |view, _, window, cx| {
                                    view.preview = Some(preview_id.clone());
                                    view.restore_focus = window.focused(cx);
                                    view.modal_focus.focus(window, cx);
                                    cx.notify();
                                },
                            )),
                        )
                    })
                    .child(
                        button(
                            gpui_omarchy::gpui::SharedString::from(format!("remove-{id}")),
                            self.language.text("Remove"),
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .on_click(cx.listener(
                            move |view, _, window, cx| {
                                view.state.composer.retain(|item| item.id != id);
                                view.focus.focus(window, cx);
                                cx.notify();
                            },
                        )),
                    ),
            );
            list = list.child(row.with_animation(
                SharedString::from(format!("content-enter-{}", item.id)),
                super::motion::content_enter(),
                |row, phase| row.opacity(phase),
            ));
        }
        list.into_any_element()
    }

    pub fn transfers(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let mut list = div()
            .id("transfers-list")
            .overflow_y_scroll()
            .max_h(rems(13.))
            .flex()
            .flex_col();
        if self.state.transfers.is_empty() {
            list = list.child(
                div()
                    .text_color(theme.secondary)
                    .child(self.language.text("No transfers yet")),
            );
        }
        for transfer in self.state.transfers.iter().rev() {
            let id = transfer.id.clone();
            let active = transfer.status == TransferStatus::Active;
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
            let mut row =
                div()
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
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_color(color)
                                    .child(self.language.text(status)),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(rems(0.6875))
                                    .text_color(theme.secondary)
                                    .child(clock.format("%H:%M").to_string()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(div().flex_1().min_w_0().text_color(theme.secondary).child(
                                format!(
                                    "{} · {}",
                                    self.language.named(
                                        if transfer.sending {
                                            "To {name}"
                                        } else {
                                            "From {name}"
                                        },
                                        &transfer.peer
                                    ),
                                    size_label(transfer.total)
                                ),
                            ))
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
                    );
            if active && !transfer.awaiting_acceptance {
                let percentage = if transfer.total == 0 {
                    0.
                } else {
                    transfer.transferred as f32 / transfer.total as f32 * 100.
                };
                row = row
                    .child(progress(
                        SharedString::from(format!("progress-{}", transfer.id)),
                        percentage,
                        cx,
                    ))
                    .child(
                        div()
                            .text_size(rems(0.6875))
                            .text_color(theme.secondary)
                            .child(format!(
                                "{} / {} · {:.0}%",
                                size_label(transfer.transferred),
                                size_label(transfer.total),
                                percentage
                            )),
                    );
            }
            if let TransferStatus::Failed(error) = &transfer.status {
                row = row.child(
                    div()
                        .text_color(theme.danger)
                        .child(self.language.error(error)),
                );
            }
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
                                    path.file_name().unwrap_or_default().to_string_lossy()
                                        != file.name
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
            list = list.child(row.with_animation(
                SharedString::from(format!("transfer-enter-{}", transfer.id)),
                super::motion::content_enter(),
                |row, phase| row.opacity(phase),
            ));
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(theme.divider())
            .child(
                div()
                    .font_weight(gpui_omarchy::gpui::FontWeight::BOLD)
                    .child(self.language.text("Transfer history")),
            )
            .child(list)
            .into_any_element()
    }

    pub fn receive_dialog(&self, cx: &mut Context<Self>) -> AnyElement {
        let request = self.state.incoming.as_ref().unwrap();
        let mut files = div()
            .id("incoming-files")
            .overflow_y_scroll()
            .max_h(rems(16.))
            .flex()
            .flex_col()
            .gap_2();
        for file in &request.files {
            files = files.child(
                div()
                    .flex()
                    .gap_3()
                    .child(div().flex_1().child(file.name.clone()))
                    .child(size_label(file.size)),
            );
        }
        let popup = dialog_popup(cx)
            .child(dialog_title(
                self.language
                    .named("{name} wants to send", &request.peer.alias),
                cx,
            ))
            .child(files)
            .child(
                div()
                    .text_color(cx.omarchy().secondary)
                    .child(self.language.text("Save to your Downloads folder")),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        button(
                            "decline",
                            self.language.text("Decline"),
                            ButtonVariant::Outline,
                            cx,
                        )
                        .on_click(
                            cx.listener(|view, _, window, cx| view.decide(false, window, cx)),
                        ),
                    )
                    .child(
                        button(
                            "accept",
                            self.language.text("Accept"),
                            ButtonVariant::Primary,
                            cx,
                        )
                        .on_click(cx.listener(|view, _, window, cx| view.decide(true, window, cx))),
                    ),
            );
        let ok = cx.listener(|view: &mut Self, _, window, cx| view.decide(true, window, cx));
        alert_dialog(&self.modal_focus, cx)
            .popup(popup.with_animation(
                "popup-enter",
                super::motion::popup_enter(),
                |popup, phase| popup.opacity(phase).top(px(4. * (1. - phase))),
            ))
            .on_ok(move |event, window, cx| {
                ok(event, window, cx);
                false
            })
            .on_close(cx.listener(|view, _, window, cx| view.decide(false, window, cx)))
            .into_any_element()
    }

    pub fn preview_dialog(&self, cx: &mut Context<Self>) -> AnyElement {
        let item = self
            .state
            .composer
            .iter()
            .find(|item| Some(&item.id) == self.preview.as_ref());
        let mut popup = dialog_popup(cx).w(rems(40.)).child(dialog_title(
            item.map(|item| item.name())
                .unwrap_or_else(|| self.language.text("Preview").into()),
            cx,
        ));
        if let Some(path) = item.and_then(|item| item.item.path()) {
            popup = popup.child(
                img(path.to_path_buf())
                    .w_full()
                    .h(rems(24.))
                    .object_fit(ObjectFit::Contain),
            );
        }
        popup = popup.child(
            button(
                "close-preview",
                self.language.text("Close"),
                ButtonVariant::Outline,
                cx,
            )
            .on_click(cx.listener(|view, _, window, cx| {
                view.preview = None;
                view.restore(window, cx);
                cx.notify();
            })),
        );
        dialog(&self.modal_focus, cx)
            .popup(popup.with_animation(
                "popup-enter",
                super::motion::popup_enter(),
                |popup, phase| popup.opacity(phase).top(px(4. * (1. - phase))),
            ))
            .on_close(cx.listener(|view, _, window, cx| {
                view.preview = None;
                view.restore(window, cx);
                cx.notify();
            }))
            .into_any_element()
    }
}
