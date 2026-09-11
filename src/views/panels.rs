use super::home::Preview;
use super::{Home, size_label};
use gpui_kit::{
    AnimationExt, AnyElement, Context, ObjectFit, SharedString, div, img, prelude::*, px, rems,
};
use gpui_omarchy::{
    ActiveTheme, ButtonVariant, IconName, alert_dialog, button, dialog, dialog_popup, dialog_title,
    icon, keycap,
};
use omasend::model::{SendItem, TransferStatus};
use rust_i18n::t;

impl Home {
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
        // A receiver has no composer items, so an incoming transfer would run
        // with nothing on screen. Give it the place the outbox occupies while
        // it is in flight.
        if let Some(transfer) = self
            .state
            .transfers
            .iter()
            .rev()
            .find(|transfer| !transfer.sending && transfer.status == TransferStatus::Active)
        {
            let transfer_id = transfer.id.clone();
            return list
                .child(
                    div()
                        .id("active-receive")
                        .debug_selector(|| "active-receive".into())
                        .flex()
                        .flex_col()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .bg(theme.background)
                        .border_1()
                        .border_color(theme.divider())
                        .p_4()
                        .child(
                            div()
                                .w(rems(28.))
                                .max_w_full()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .justify_center()
                                        .text_color(theme.accent)
                                        .child(icon(IconName::ArrowDown).size(rems(2.))),
                                )
                                .child(
                                    div().w_full().text_center().text_color(theme.bright).child(
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
                                        .w_full()
                                        .text_center()
                                        .text_color(theme.secondary)
                                        .child(format!(
                                            "{} · {}",
                                            self.language.named("history.from", &transfer.peer),
                                            size_label(transfer.total),
                                        )),
                                )
                                .child(self.transfer_progress(transfer, "active-receive", None, cx))
                                .child(
                                    div().flex().justify_center().child(
                                        button(
                                            "cancel-active-receive",
                                            t!("action.cancel"),
                                            ButtonVariant::Outline,
                                            cx,
                                        )
                                        .on_click(
                                            cx.listener(move |view, _, _, cx| {
                                                if let Some(node) = &view.node
                                                    && let Err(error) =
                                                        node.handle.cancel(&transfer_id)
                                                {
                                                    view.state.error = Some(error.to_string());
                                                }
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                                ),
                        ),
                )
                .into_any_element();
        }
        if self.state.composer.is_empty()
            && let Some(transfer) = self
                .state
                .transfers
                .iter()
                .filter(|transfer| transfer.status == TransferStatus::Completed)
                .max_by_key(|transfer| transfer.when)
            && self.dismissed_success.as_ref() != Some(&transfer.id)
        {
            let transfer_id = transfer.id.clone();
            return list
                .child(
                    div()
                        .id("last-completed-transfer")
                        .debug_selector(|| "last-completed-transfer".into())
                        .relative()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .bg(theme.background)
                        .border_1()
                        .border_color(theme.divider())
                        .p_4()
                        .child(
                            div()
                                .w(rems(28.))
                                .max_w_full()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .justify_center()
                                        .text_color(theme.success)
                                        .child(icon(IconName::Check).size(rems(2.))),
                                )
                                .child(self.transfer_summary(transfer, cx))
                                .child(
                                    div()
                                        .flex()
                                        .justify_center()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .id("dismiss-transfer-success")
                                                .debug_selector(|| {
                                                    "dismiss-transfer-success".into()
                                                })
                                                .child(
                                                    button(
                                                        "confirm-transfer-success",
                                                        t!("status.done"),
                                                        ButtonVariant::Outline,
                                                        cx,
                                                    )
                                                    .on_click(cx.listener(
                                                        move |view, _, window, cx| {
                                                            view.dismissed_success =
                                                                Some(transfer_id.clone());
                                                            view.focus.focus(window, cx);
                                                            cx.notify();
                                                        },
                                                    )),
                                                ),
                                        )
                                        .when_some(
                                            transfer.received_text.clone(),
                                            |actions, text| {
                                                actions.child(
                                                    button(
                                                        "show-completed-text",
                                                        t!("action.show_text"),
                                                        ButtonVariant::Outline,
                                                        cx,
                                                    )
                                                    .debug_selector(|| "show-completed-text".into())
                                                    .on_click(cx.listener(
                                                        move |view, _, window, cx| {
                                                            view.open_received_text(
                                                                text.clone(),
                                                                window,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                                )
                                            },
                                        )
                                        .when(transfer.received_text.is_none(), |actions| {
                                            actions.when_some(
                                                transfer.paths.first().cloned(),
                                                |actions, path| {
                                                    actions.child(
                                                        button(
                                                            "reveal-completed-transfer",
                                                            t!("action.show_in_files"),
                                                            ButtonVariant::Outline,
                                                            cx,
                                                        )
                                                        .debug_selector(|| {
                                                            "reveal-completed-transfer".into()
                                                        })
                                                        .on_click(move |_, _, cx| {
                                                            cx.reveal_path(&path)
                                                        }),
                                                    )
                                                },
                                            )
                                        }),
                                ),
                        ),
                )
                .into_any_element();
        }
        if self.state.composer.is_empty() {
            list = list.child(
                div()
                    .id("composer-welcome")
                    .debug_selector(|| "composer-welcome".into())
                    .flex()
                    .flex_col()
                    .flex_1()
                    .justify_center()
                    .items_center()
                    .gap_3()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.divider())
                    .p_4()
                    .child(super::motion::discovery_logo(
                        theme.accent,
                        self.state.discovering && !cx.reduce_motion(),
                        self.state.discovery_revision,
                        64.,
                    ))
                    .child(
                        div()
                            .text_color(theme.bright)
                            .font_weight(gpui_kit::FontWeight::BOLD)
                            .child(t!("composer.drop_hint")),
                    )
                    .child(
                        div()
                            .text_color(theme.secondary)
                            .child(t!("composer.drop_kinds")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(keycap("ctrl+v", cx))
                            .child(t!("action.paste"))
                            .child(div().w(rems(0.5)))
                            .child(keycap("ctrl+o", cx))
                            .child(t!("action.add_files")),
                    ),
            );
        }
        for item in &self.state.composer {
            let id = item.id.clone();
            let transfer = self
                .state
                .transfers
                .iter()
                .rev()
                .find(|transfer| transfer.composer_ids.contains(&item.id));
            let active = transfer.is_some_and(|transfer| transfer.status == TransferStatus::Active);

            let mut row = div()
                .flex()
                .flex_col()
                .gap_2()
                .flex_shrink_0()
                .p_3()
                .border_1()
                .border_color(theme.border);
            if item.is_video() {
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
                    .when(item.is_image(), |header| {
                        header.when_some(item.item.path(), |header, path| {
                            header.child(
                                div()
                                    .flex_shrink_0()
                                    .size(px(40.))
                                    .mr_1()
                                    .overflow_hidden()
                                    .child(
                                        img(path.to_path_buf())
                                            .size_full()
                                            .object_fit(ObjectFit::Contain),
                                    ),
                            )
                        })
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .when(!matches!(item.item, SendItem::Text(_)), |details| {
                                details.child(
                                    div().overflow_hidden().text_ellipsis().child(item.name()),
                                )
                            })
                            .child(div().text_color(theme.secondary).child(
                                if matches!(item.item, SendItem::Text(_)) {
                                    size_label(item.size())
                                } else {
                                    format!(
                                        "{} · {}",
                                        size_label(item.size()),
                                        self.language.count(
                                            item.uploads.len(),
                                            "history.file_one",
                                            "history.file_other"
                                        )
                                    )
                                },
                            )),
                    )
                    .when(item.is_image(), |row| {
                        row.child(
                            button(
                                gpui_kit::SharedString::from(format!("preview-{id}")),
                                t!("action.preview_ellipsis"),
                                ButtonVariant::Secondary,
                                cx,
                            )
                            .on_click(cx.listener(
                                move |view, _, window, cx| {
                                    view.preview = Some(Preview::Composer(preview_id.clone()));
                                    view.restore_focus = window.focused(cx);
                                    view.modal_focus.focus(window, cx);
                                    cx.notify();
                                },
                            )),
                        )
                    })
                    .when(!active, |row| {
                        row.child(
                            button(
                                gpui_kit::SharedString::from(format!("remove-{id}")),
                                t!("action.remove"),
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
                        )
                    })
                    .when(active, |row| {
                        let transfer_id = transfer.unwrap().id.clone();
                        row.child(
                            button(
                                SharedString::from(format!("cancel-item-{}", item.id)),
                                t!("action.cancel"),
                                ButtonVariant::Secondary,
                                cx,
                            )
                            .on_click(cx.listener(
                                move |view, _, _, cx| {
                                    if let Some(node) = &view.node
                                        && let Err(error) = node.handle.cancel(&transfer_id)
                                    {
                                        view.state.error = Some(error.to_string());
                                    }
                                    cx.notify();
                                },
                            )),
                        )
                    }),
            );
            if let Some(transfer) = transfer {
                let offset = self
                    .state
                    .composer
                    .iter()
                    .take_while(|previous| previous.id != item.id)
                    .filter(|previous| transfer.composer_ids.contains(&previous.id))
                    .map(|previous| previous.size())
                    .sum();
                row = row.child(self.transfer_progress(
                    transfer,
                    &item.id,
                    Some((offset, item.size())),
                    cx,
                ));
            }

            list = list.child(row.with_animation(
                SharedString::from(format!("content-enter-{}", item.id)),
                super::motion::content_enter(),
                |row, phase| row.opacity(phase),
            ));
        }
        list.into_any_element()
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
                self.language.named("receive.title", &request.peer.alias),
                cx,
            ))
            .child(files)
            .child(
                div()
                    .text_color(cx.omarchy().secondary)
                    .child(t!("receive.destination")),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        button("decline", t!("action.decline"), ButtonVariant::Outline, cx)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.decide(false, window, cx)),
                            ),
                    )
                    .child(
                        button("accept", t!("action.accept"), ButtonVariant::Primary, cx).on_click(
                            cx.listener(|view, _, window, cx| view.decide(true, window, cx)),
                        ),
                    ),
            );
        let ok = cx.listener(|view: &mut Self, _, window, cx| view.decide(true, window, cx));
        alert_dialog(&self.modal_focus, cx)
            .popup(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .id("receive-popup")
                            .debug_selector(|| "receive-popup".into())
                            .max_w_full()
                            .child(popup.with_animation(
                                "popup-enter",
                                super::motion::popup_enter(),
                                |popup, phase| popup.opacity(phase).top(px(4. * (1. - phase))),
                            )),
                    ),
            )
            .on_ok(move |event, window, cx| {
                ok(event, window, cx);
                false
            })
            .on_close(cx.listener(|view, _, window, cx| view.decide(false, window, cx)))
            .into_any_element()
    }

    pub fn preview_dialog(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let item =
            self.state.composer.iter().find(
                |item| matches!(&self.preview, Some(Preview::Composer(id)) if id == &item.id),
            );
        let mut popup = dialog_popup(cx).w(rems(40.)).child(dialog_title(
            item.map(|item| item.name())
                .unwrap_or_else(|| t!("action.preview").into()),
            cx,
        ));
        let mut actions = div().flex().justify_end().gap_2();
        if let Some(Preview::ReceivedText { input, copied }) = &self.preview {
            popup = dialog_popup(cx)
                .w(rems(40.))
                .child(dialog_title(t!("dialog.received_text"), cx))
                .child(
                    gpui_omarchy::textarea("received-text", input, window, cx)
                        .h(rems(18.))
                        .overflow_hidden()
                        .debug_selector(|| "received-text".into()),
                );
            actions = actions.child(
                button(
                    "copy-received-text",
                    t!(if *copied {
                        "status.copied"
                    } else {
                        "action.copy"
                    }),
                    ButtonVariant::Outline,
                    cx,
                )
                .debug_selector(|| "copy-received-text".into())
                .on_click(cx.listener(|view, _, _, cx| {
                    if let Some(Preview::ReceivedText { input, copied }) = &mut view.preview {
                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                            input.read(cx).value().to_string(),
                        ));
                        *copied = true;
                        cx.notify();
                    }
                })),
            );
        }
        if let Some(path) = item.and_then(|item| item.item.path()) {
            popup = popup.child(
                img(path.to_path_buf())
                    .w_full()
                    .h(rems(24.))
                    .object_fit(ObjectFit::Contain),
            );
        }
        actions = actions.child(
            button(
                "close-preview",
                t!("action.close"),
                ButtonVariant::Outline,
                cx,
            )
            .on_click(cx.listener(|view, _, window, cx| {
                view.preview = None;
                view.restore(window, cx);
                cx.notify();
            })),
        );
        popup = popup.child(actions);
        dialog(&self.modal_focus, cx)
            .popup(div().max_w_full().occlude().child(popup.with_animation(
                "popup-enter",
                super::motion::popup_enter(),
                |popup, phase| popup.opacity(phase).top(px(4. * (1. - phase))),
            )))
            .on_close(cx.listener(|view, _, window, cx| {
                view.preview = None;
                view.restore(window, cx);
                cx.notify();
            }))
            .into_any_element()
    }
}
