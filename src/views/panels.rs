//! Keyed lists keep focus and animation nodes alive across network progress.
use super::appearance::{self, theme};
use super::{control, plain_control, session::Session};
use nsh_core::prelude::*;
use omasend::model::TransferStatus;

#[cfg(all(test, feature = "ui-tests"))]
pub(crate) fn content(session: Session) -> impl IntoElement {
    view()
        .child(nearby(session.clone()))
        .child(outbox(session.clone()))
        .child(activity(session))
}

pub(super) fn error_notice(session: Session) -> impl IntoElement {
    let status = session.clone();
    when(
        move || {
            status.revision.get();
            status.state.borrow().error.is_some()
        },
        move || {
            let details = session.clone();
            let message = computed(move || {
                details.revision.get();
                let language = details.language.get();
                let raw = details.state.borrow().error.clone().unwrap_or_default();
                if raw.contains("Cannot listen for LocalSend transfers") {
                    (
                        language.text("error.notice_unavailable"),
                        language.text("error.notice_port"),
                    )
                } else {
                    (language.text("error.notice_title"), language.error(&raw))
                }
            });
            let dismiss = session.clone();
            view()
                .test_id("error-notice")
                .role(AccessibilityRole::Alert)
                .mx_4()
                .mt_3()
                .p_3()
                .flex_row()
                .items_center()
                .gap_3()
                .border_1()
                .border_color(appearance::divider)
                .bg(move || theme().foreground.opacity(0.03))
                .rounded_none()
                .child(
                    view()
                        .size(px(20.0))
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(move || theme().color("destructive").unwrap().opacity(0.4))
                        .child(text("!").text_color(move || theme().color("destructive").unwrap())),
                )
                .child(
                    view()
                        .flex_1()
                        .gap_1()
                        .child(text(move || message.get().0).font_weight(FontWeight::BOLD))
                        .child(
                            text(move || message.get().1)
                                .text_size(move || px(if is_desktop() { 11.0 } else { 13.0 }))
                                .text_color(move || theme().muted_foreground),
                        ),
                )
                .child(super::utilities::copy_error_button(session.clone()))
                .child(super::utilities::close_button(
                    session.translated("action.dismiss_error"),
                    move || {
                        dismiss.state.borrow_mut().error = None;
                        dismiss.changed();
                    },
                ))
        },
    )
}

pub(super) fn nearby(session: Session) -> impl IntoElement {
    let count = session.clone();
    let empty = session.clone();
    let devices = session.clone();
    let rows = session.clone();
    let expanded = signal(false);
    let overflow = session.clone();
    let language = session.language;
    view()
        .test_id("nearby-panel")
        .gap_2()
        .px(px(16.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(appearance::divider)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(text(session.translated("nearby.title")))
                .child(
                    text(move || {
                        count.revision.get();
                        count.state.borrow().devices.len().to_string()
                    })
                    .text_color(move || theme().muted_foreground),
                )
                .child(view().flex_1())
                .child(when(
                    move || {
                        overflow.revision.get();
                        overflow.state.borrow().devices.len() > nearby_columns(window().width.0)
                    },
                    move || {
                        plain_control(
                            move || {
                                language.get().text(if expanded.get() {
                                    "action.collapse"
                                } else {
                                    "action.expand"
                                })
                            },
                            false,
                            move || {
                                if !reduce_motion() {
                                    animate_layout(nsh_core::LayoutAnimation::all(
                                        nsh_core::LayoutTransition::new(
                                            nsh_core::LayoutCurve::EaseOut,
                                            160.0,
                                        ),
                                    ));
                                }
                                expanded.set(!expanded.get());
                            },
                        )
                        .test_id("toggle-nearby")
                    },
                )),
        )
        .child(when(
            move || {
                empty.revision.get();
                empty.state.borrow().devices.is_empty()
            },
            {
                let session = session.clone();
                move || {
                    text(session.translated("nearby.looking"))
                        .text_color(move || theme().muted_foreground)
                }
            },
        ))
        .child(
            scroll()
                .max_h(move || px((window().height.0 * 0.35).min(190.0)))
                .child(view().flex_row().flex_wrap().gap_2().child(each(
                    move || {
                        devices.revision.get();
                        let state = devices.state.borrow();
                        let selected = state.devices.iter().position(|device| {
                            state.selected.as_ref() == Some(&device.fingerprint)
                        });
                        nearby_indices(
                            state.devices.len(),
                            nearby_columns(window().width.0),
                            expanded.get(),
                            selected,
                        )
                        .into_iter()
                        .map(|index| state.devices[index].fingerprint.clone())
                        .collect::<Vec<_>>()
                    },
                    Clone::clone,
                    move |id, _| device_card(rows.clone(), id.get_untracked()),
                ))),
        )
}

fn nearby_columns(width: f32) -> usize {
    (((width - 32.0 + 8.0) / 248.0).floor() as usize).max(1)
}

fn nearby_indices(
    total: usize,
    columns: usize,
    expanded: bool,
    selected: Option<usize>,
) -> Vec<usize> {
    if expanded {
        return (0..total).collect();
    }
    let mut visible: Vec<_> = (0..total.min(columns)).collect();
    if let Some(selected) = selected.filter(|&index| index < total && index >= columns)
        && let Some(last) = visible.last_mut()
    {
        *last = selected;
    }
    visible
}

fn device_card(session: Session, id: String) -> impl IntoElement {
    let selected_session = session.clone();
    let selected_id = id.clone();
    let selected = computed(move || {
        selected_session.revision.get();
        selected_session.state.borrow().selected.as_ref() == Some(&selected_id)
    });
    let labels = session.clone();
    let label_id = id.clone();
    let device = computed(move || {
        labels.revision.get();
        labels
            .state
            .borrow()
            .devices
            .iter()
            .find(|d| d.fingerprint == label_id)
            .map(|d| (d.alias.clone(), d.model.clone()))
    });
    let width = move || {
        let available = window().width.0 - 32.0;
        let columns = nearby_columns(window().width.0) as f32;
        px((available - 8.0 * (columns - 1.0)) / columns)
    };
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .p_3()
        .w(width)
        .h(px(58.0))
        .rounded_none()
        .border_1()
        .border_color(move || {
            if selected.get() {
                theme().primary
            } else {
                theme().border
            }
        })
        .bg(move || {
            theme()
                .foreground
                .opacity(if selected.get() { 0.04 } else { 0.0 })
        })
        .hover(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .focus(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .role(AccessibilityRole::Button)
        .label(move || device.get().map(|d| d.0).unwrap_or_default())
        .on_press(move |_: &()| session.select(id.clone()))
        .child(
            view()
                .size(px(32.0))
                .items_center()
                .justify_center()
                .border_1()
                .border_color(move || theme().color("success.background").unwrap().opacity(0.45))
                .bg(move || theme().color("success.background").unwrap().opacity(0.12))
                .child(
                    text(move || {
                        device
                            .get()
                            .map(|d| device_initials(&d.0))
                            .unwrap_or_default()
                    })
                    .text_color(move || theme().color("success.background").unwrap()),
                ),
        )
        .child(
            view()
                .flex_1()
                .child(
                    text(move || device.get().map(|d| d.0).unwrap_or_default()).number_of_lines(1),
                )
                .child(
                    text(move || {
                        device
                            .get()
                            .map(|d| {
                                if d.1.is_empty() {
                                    "LocalSend".into()
                                } else {
                                    d.1
                                }
                            })
                            .unwrap_or_default()
                    })
                    .number_of_lines(1)
                    .text_size(px(11.0))
                    .text_color(move || theme().muted_foreground),
                ),
        )
        .child(view().size(px(12.0)).flex_shrink_0().child(when(
            move || selected.get(),
            || {
                view()
                    .test_id("nearby-check-icon")
                    .child(super::utilities::check_icon())
            },
        )))
}

fn device_initials(name: &str) -> String {
    let mut words = name.split_whitespace();
    let first = words.next().unwrap_or("?");
    match words.next() {
        Some(second) => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect::<String>(),
        None => first.chars().take(2).collect(),
    }
    .to_uppercase()
    .chars()
    .take(2)
    .collect()
}

fn keycap(label: &'static str) -> impl IntoElement {
    view()
        .px(px(4.0))
        .py(px(3.0))
        .border_1()
        .border_color(move || theme().foreground.opacity(0.25))
        .child(text(label).text_size(px(11.0)))
}

#[derive(Clone, Copy)]
struct ReceivedPresentation {
    height: Signal<f32>,
    dismissed: Signal<Option<String>>,
}

pub(super) fn outbox(session: Session) -> impl IntoElement {
    let items = session.clone();
    let item_rows = session.clone();
    let empty = session.clone();
    let full = session.clone();
    let dismissed_received = signal(None::<String>);
    let content_height = signal(0.0);
    let presentation = ReceivedPresentation {
        height: content_height,
        dismissed: dismissed_received,
    };
    let received = session.clone();
    let latest_received = computed(move || {
        received.revision.get();
        let state = received.state.borrow();
        if !state.composer.is_empty() {
            return None;
        }
        state
            .transfers
            .iter()
            .rev()
            .find(|transfer| !transfer.sending && transfer.status == TransferStatus::Completed)
            .map(|transfer| transfer.id.clone())
            .filter(|id| dismissed_received.get().as_ref() != Some(id))
    });
    let guard = session.clone();
    let enabled = computed(move || !guard.input_blocked());
    let hovering = signal(false);
    let highlight = animated_value(0.0);
    let changing = highlight;
    effect(move || {
        let accepts_files = enabled.get();
        if !accepts_files {
            hovering.set(false);
        }
        let active = accepts_files && hovering.get();
        changing.timing(
            if active { 1.0 } else { 0.0 },
            if reduce_motion() { 0.0 } else { 110.0 },
            Easing::Linear,
        );
    });
    let receiver = session.clone();
    view()
        .test_id("composer-area")
        .on_layout(move |bounds: &Bounds<Points>| content_height.set(bounds.size.height.0))
        .disabled(move || !enabled.get())
        .on_file_drag_enter(move |_: &()| hovering.set(true))
        .on_file_drag_leave(move |_: &()| hovering.set(false))
        .on_file_drop(move |event: &FileDropEvent| {
            hovering.set(false);
            receiver.drop_files(event.uris.clone());
        })
        .bg(appearance::panel_background)
        .rounded_none()
        .flex_1()
        .border_1()
        .border_color(appearance::divider)
        .child(
            view()
                .test_id("composer-drop-highlight")
                .absolute()
                .inset_0()
                .pointer_events(PointerEvents::None)
                .border_2()
                .border_color(move || theme().primary)
                .rounded_none()
                .bind_opacity(&highlight),
        )
        .child(when(
            move || {
                empty.revision.get();
                empty.state.borrow().composer.is_empty() && latest_received.get().is_none()
            },
            {
                let session = session.clone();
                move || {
                    view()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .child(super::motion::emblem(64.0))
                        .child(text(session.translated(if is_desktop() {
                            "composer.drop_hint"
                        } else {
                            "composer.mobile_hint"
                        })))
                        .child(
                            text(session.translated("composer.drop_kinds"))
                                .text_color(move || theme().muted_foreground)
                                .max_w(move || px((window().width.0 - 64.0).max(0.0)))
                                .text_center()
                                .number_of_lines(2),
                        )
                        .child(when(is_desktop, {
                            let session = session.clone();
                            move || {
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(keycap("ctrl+v"))
                                    .child(text(session.translated("action.paste")))
                                    .child(view().w(px(8.0)))
                                    .child(keycap("ctrl+o"))
                                    .child(text(session.translated("action.add_files")))
                            }
                        }))
                }
            },
        ))
        .child(when(move || latest_received.get().is_some(), {
            let session = session.clone();
            move || {
                let session = session.clone();
                scroll().flex_1().child(view().p_3().child(each(
                    move || latest_received.get().into_iter().collect::<Vec<_>>(),
                    Clone::clone,
                    move |id, _| {
                        let id = id.get_untracked();
                        transfer_row(session.clone(), id, Some(presentation))
                    },
                )))
            }
        }))
        .child(when(
            move || {
                full.revision.get();
                !full.state.borrow().composer.is_empty()
            },
            move || {
                let items = items.clone();
                let item_rows = item_rows.clone();
                scroll().flex_1().child(view().p_3().gap_2().child(each(
                    move || {
                        items.revision.get();
                        items
                            .state
                            .borrow()
                            .composer
                            .iter()
                            .map(|i| i.id.clone())
                            .collect::<Vec<_>>()
                    },
                    Clone::clone,
                    move |id, _| {
                        let id = id.get_untracked();
                        let labels = item_rows.clone();
                        let remover = item_rows.clone();
                        let progress_session = item_rows.clone();
                        let progress_id = id.clone();
                        let previewer = item_rows.clone();
                        let preview_id = id.clone();
                        let previewable = item_rows.clone();
                        let check_id = id.clone();
                        let thumbnail_session = item_rows.clone();
                        let thumbnail_id = id.clone();
                        let label_id = id.clone();
                        let summary_session = item_rows.clone();
                        let summary_id = id.clone();
                        let action_session = item_rows.clone();
                        let action_id = id.clone();
                        let active_transfer =
                            computed(move || action_session.item_transfer(&action_id));
                        let action_language = item_rows.language;
                        let preview_language = item_rows.language;
                        let summary = computed(move || {
                            summary_session.revision.get();
                            summary_session
                                .state
                                .borrow()
                                .composer
                                .iter()
                                .find(|item| item.id == summary_id)
                                .and_then(|item| {
                                    if item.is_video() {
                                        Some((true, "▶".to_owned()))
                                    } else if let omasend::model::SendItem::Text(text) = &item.item
                                    {
                                        Some((false, text.chars().take(240).collect()))
                                    } else {
                                        None
                                    }
                                })
                        });
                        view()
                            .test_id("composer-card")
                            .p_3()
                            .gap_2()
                            .border_1()
                            .border_color(appearance::divider)
                            .rounded_none()
                            .child(when(
                                move || summary.get().is_some(),
                                move || {
                                    text(move || summary.get().map(|s| s.1).unwrap_or_default())
                                        .test_id("composer-content-summary")
                                        .number_of_lines(3)
                                        .text_size(move || {
                                            px(if summary.get().is_some_and(|s| s.0) {
                                                24.0
                                            } else {
                                                12.0
                                            })
                                        })
                                        .text_color(move || theme().muted_foreground)
                                },
                            ))
                            .child(
                                view()
                                    .flex_row()
                                    .gap_2()
                                    .items_center()
                                    .child(thumbnail(thumbnail_session, thumbnail_id))
                                    .child(
                                        text(move || {
                                            labels.revision.get();
                                            labels
                                                .state
                                                .borrow()
                                                .composer
                                                .iter()
                                                .find(|i| i.id == label_id)
                                                .map(|i| {
                                                    if matches!(
                                                        i.item,
                                                        omasend::model::SendItem::Text(_)
                                                    ) {
                                                        size_label(i.size())
                                                    } else {
                                                        format!(
                                                            "{} · {} · {}",
                                                            i.name(),
                                                            size_label(i.size()),
                                                            labels.language.get().count(
                                                                i.uploads.len(),
                                                                "history.file_one",
                                                                "history.file_other",
                                                            )
                                                        )
                                                    }
                                                })
                                                .unwrap_or_default()
                                        })
                                        .number_of_lines(1)
                                        .flex_1(),
                                    )
                                    .child(when(
                                        move || {
                                            previewable.revision.get();
                                            previewable.state.borrow().composer.iter().any(|item| {
                                                item.id == check_id
                                                    && (item.is_image()
                                                        || matches!(
                                                            item.item,
                                                            omasend::model::SendItem::Text(_)
                                                        ))
                                            })
                                        },
                                        move || {
                                            let previewer = previewer.clone();
                                            let preview_id = preview_id.clone();
                                            control(
                                                move || {
                                                    preview_language.get().text("action.preview")
                                                },
                                                false,
                                                move || previewer.preview_item(&preview_id),
                                            )
                                        },
                                    ))
                                    .child(control(
                                        move || {
                                            action_language.get().text(
                                                if active_transfer.get().is_some() {
                                                    "action.cancel"
                                                } else {
                                                    "action.remove"
                                                },
                                            )
                                        },
                                        false,
                                        move || {
                                            if let Some(transfer) = remover.item_transfer(&id) {
                                                remover.cancel(&transfer);
                                            } else {
                                                remover.remove(&id);
                                            }
                                        },
                                    )),
                            )
                            .child(composer_progress(progress_session, progress_id))
                    },
                )))
            },
        ))
}

fn composer_progress(session: Session, id: String) -> impl IntoElement {
    let language = session.language;
    let details = computed(move || {
        session.revision.get();
        let state = session.state.borrow();
        let transfer = state
            .transfers
            .iter()
            .rev()
            .find(|transfer| transfer.composer_ids.contains(&id))?;
        let item = state.composer.iter().find(|item| item.id == id)?;
        let offset: u64 = state
            .composer
            .iter()
            .take_while(|item| item.id != id)
            .filter(|item| transfer.composer_ids.contains(&item.id))
            .map(|item| item.size())
            .sum();
        let size = item.size();
        let bytes = transfer.transferred.saturating_sub(offset).min(size);
        Some((
            bytes,
            size,
            transfer.awaiting_acceptance,
            transfer.status.clone(),
            if transfer.transferred >= offset && transfer.transferred < offset.saturating_add(size)
            {
                transfer.bytes_per_second()
            } else {
                0
            },
        ))
    });
    let progress = animated_value(0.0);
    let driver = progress;
    effect(move || {
        let ratio = details
            .get()
            .map(|(bytes, size, ..)| {
                if size == 0 {
                    0.0
                } else {
                    bytes as f32 / size as f32
                }
            })
            .unwrap_or(0.0);
        if reduce_motion() {
            driver.set(ratio);
        } else {
            driver.timing(ratio, 160.0, Easing::Linear);
        }
    });
    when(
        move || details.get().is_some(),
        move || {
            view()
                .gap_2()
                .child(
                    text(move || {
                        let Some((bytes, size, waiting, status, speed)) = details.get() else {
                            return String::new();
                        };
                        match status {
                            TransferStatus::Active if waiting => {
                                language.get().text("status.waiting").to_string()
                            }
                            TransferStatus::Active => format!(
                                "{} / {} · {:.0}% · {}/s",
                                size_label(bytes),
                                size_label(size),
                                if size == 0 {
                                    0.0
                                } else {
                                    bytes as f64 / size as f64 * 100.0
                                },
                                size_label(speed),
                            ),
                            TransferStatus::Completed => {
                                language.get().text("status.sent").to_string()
                            }
                            TransferStatus::Cancelled => {
                                language.get().text("status.cancelled").to_string()
                            }
                            TransferStatus::Failed(error) => language.get().transfer_error(&error),
                        }
                    })
                    .text_size(px(11.0))
                    .text_color(move || theme().muted_foreground),
                )
                .child(when(
                    move || {
                        details
                            .get()
                            .is_some_and(|d| !d.2 && d.3 == TransferStatus::Active)
                    },
                    move || {
                        view()
                            .test_id("composer-transfer-progress")
                            .h(px(3.0))
                            .bg(move || theme().border)
                            .child(
                                view()
                                    .h(px(3.0))
                                    .w_full()
                                    .bg(move || theme().color("progress.bar.background").unwrap())
                                    .transform_origin(nsh_core::TransformOrigin {
                                        x: nsh_core::geometry::DefiniteLength::Fraction(
                                            nsh_core::geometry::Percentage(0.0),
                                        ),
                                        ..Default::default()
                                    })
                                    .bind_transform(nsh_core::TransformKind::ScaleX, &progress),
                            )
                    },
                ))
        },
    )
}

#[cfg(all(test, feature = "ui-tests"))]
pub(super) fn activity(session: Session) -> impl IntoElement {
    activity_with(session, true)
}

pub(super) fn active(session: Session) -> impl IntoElement {
    activity_with(session, false)
}

pub(super) fn history_overlay(session: Session) -> impl IntoElement {
    let visible = session.history_visible;
    let clear = session.clone();
    let empty = session.clone();
    let no_records = session.clone();
    nsh_base::sheet()
        .open(move || Some(visible.get()))
        .side(Placement::Bottom)
        .on_open_change(move |event: &nsh_base::SheetEvent| {
            if let nsh_base::SheetEvent::OpenChange(next, _) = event {
                visible.set(*next);
            }
        })
        .popup_style(|s| {
            s.h(move || px(window().height.0 * 0.55))
                .bg(appearance::panel_background)
                .text_color(move || theme().foreground)
                .font_family(move || SharedString::from(appearance::metrics().family))
                .text_size(move || px(theme().number("font.size").unwrap()))
                .pb(move || safe_area().bottom)
                .border_t_1()
                .border_color(appearance::divider)
        })
        .backdrop_style(|s| s.bg(AdaptiveColor::from(rgb(0)).opacity(0.45)))
        .child(
            view()
                .test_id("history-sheet")
                .w_full()
                .flex_1()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_4()
                        .py_2()
                        .flex_shrink_0()
                        .border_b_1()
                        .border_color(appearance::divider)
                        .child(text(session.translated("history.title")).flex_1())
                        .child(
                            plain_control(
                                session.translated("action.clear"),
                                move || {
                                    empty.revision.get();
                                    !empty
                                        .state
                                        .borrow()
                                        .transfers
                                        .iter()
                                        .any(|t| t.status != TransferStatus::Active)
                                },
                                move || clear.clear_history(),
                            )
                            .label(session.translated("action.clear_history"))
                            .text_color(move || theme().color("destructive").unwrap()),
                        )
                        .child(super::utilities::close_button(
                            session.translated("action.close"),
                            move || visible.set(false),
                        )),
                )
                .child(
                    scroll().flex_1().child(
                        view()
                            .px_4()
                            .child(when(
                                move || {
                                    no_records.revision.get();
                                    no_records.state.borrow().transfers.is_empty()
                                },
                                {
                                    let session = session.clone();
                                    move || {
                                        text(session.translated("history.empty"))
                                            .py_3()
                                            .text_color(move || theme().muted_foreground)
                                    }
                                },
                            ))
                            .child(activity_with(session, true)),
                    ),
                ),
        )
}

fn activity_with(session: Session, include_finished: bool) -> impl IntoElement {
    let requests = session.clone();
    let request_rows = session.clone();
    let transfers = session.clone();
    let transfer_rows = session;
    view()
        .gap_2()
        .child(each(
            move || {
                requests.revision.get();
                requests
                    .state
                    .borrow()
                    .incoming
                    .as_ref()
                    .map(|r| vec![r.id.clone()])
                    .unwrap_or_default()
            },
            Clone::clone,
            move |id, _| {
                let id = id.get_untracked();
                let labels = request_rows.clone();
                let acceptor = request_rows.clone();
                let decliner = request_rows.clone();
                let decline_id = id.clone();
                let files = request_rows
                    .state
                    .borrow()
                    .incoming
                    .as_ref()
                    .map(|request| {
                        request
                            .files
                            .iter()
                            .map(|file| format!("{} · {}", file.name, size_label(file.size)))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                nsh_base::dialog()
                    .animation(if reduce_motion() {
                        ModalAnimation::None
                    } else {
                        ModalAnimation::Fade
                    })
                    .on_cancel({
                        let decider = request_rows.clone();
                        let id = decline_id.clone();
                        move |_: &()| decider.decide(&id, false)
                    })
                    .child(
                        view()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .p_6()
                            .bg(AdaptiveColor::from(rgb(0)).opacity(0.6))
                            .child(
                                view()
                                    .w_full()
                                    .max_w(px(420.0))
                                    .gap_3()
                                    .bg(appearance::panel_background)
                                    .border_1()
                                    .p_4()
                                    .border_color(move || theme().border)
                                    .child(text(move || {
                                        labels.revision.get();
                                        labels
                                            .state
                                            .borrow()
                                            .incoming
                                            .as_ref()
                                            .map(|r| {
                                                labels
                                                    .language
                                                    .get()
                                                    .named("receive.title", &r.peer.alias)
                                            })
                                            .unwrap_or_default()
                                    }))
                                    .child(scroll().max_h(px(220.0)).child(
                                        view().gap_2().children(files.into_iter().map(text)),
                                    ))
                                    .child(text(request_rows.translated("receive.destination")))
                                    .child(
                                        view()
                                            .flex_row()
                                            .gap_2()
                                            .child(control(
                                                request_rows.translated("action.accept"),
                                                false,
                                                move || acceptor.decide(&id, true),
                                            ))
                                            .child(control(
                                                request_rows.translated("action.decline"),
                                                false,
                                                move || decliner.decide(&decline_id, false),
                                            )),
                                    ),
                            ),
                    )
            },
        ))
        .child(each(
            move || {
                if include_finished {
                    transfers.visible_transfer_ids()
                } else {
                    transfers.revision.get();
                    let state = transfers.state.borrow();
                    state
                        .transfers
                        .iter()
                        .filter(|t| t.status == TransferStatus::Active)
                        .map(|t| t.id.clone())
                        .collect()
                }
            },
            Clone::clone,
            move |id, _| transfer_row(transfer_rows.clone(), id.get_untracked(), None),
        ))
}

fn transfer_row(
    session: Session,
    id: String,
    presentation: Option<ReceivedPresentation>,
) -> impl IntoElement {
    let dismiss_language = session.language;
    let dismiss_id = id.clone();
    let labels = session.clone();
    let label_id = id.clone();
    let details = computed(move || {
        labels.revision.get();
        labels
            .state
            .borrow()
            .transfers
            .iter()
            .find(|t| t.id == label_id)
            .map(|t| {
                let status = match &t.status {
                    TransferStatus::Active if t.awaiting_acceptance => {
                        labels.language.get().text("status.waiting")
                    }
                    TransferStatus::Active => labels.language.get().text(if t.sending {
                        "status.sending"
                    } else {
                        "status.receiving"
                    }),
                    TransferStatus::Completed => labels.language.get().text(if t.sending {
                        "status.sent"
                    } else {
                        "status.received"
                    }),
                    TransferStatus::Cancelled => labels.language.get().text("status.cancelled"),
                    TransferStatus::Failed(error) => labels.language.get().transfer_error(error),
                };
                let when: chrono::DateTime<chrono::Local> = t.when.into();
                let size = if t.status == TransferStatus::Active {
                    format!("{} / {}", size_label(t.transferred), size_label(t.total))
                } else {
                    size_label(t.total)
                };
                (
                    t.peer.clone(),
                    status,
                    format!("{} · {}", size, when.format("%m-%d %H:%M")),
                    t.status.clone(),
                )
            })
    });
    let active = computed(move || details.get().is_some_and(|d| d.3 == TransferStatus::Active));
    let updates = session.clone();
    let update_id = id.clone();
    let cancel = session.clone();
    let cancel_id = id.clone();
    let progress = animated_value(0.0);
    let driver = progress;
    effect(move || {
        updates.revision.get();
        let ratio = updates
            .state
            .borrow()
            .transfers
            .iter()
            .find(|t| t.id == update_id)
            .map(|t| {
                if t.total > 0 {
                    (t.transferred as f32 / t.total as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);
        if reduce_motion() {
            driver.set(ratio);
        } else {
            driver.timing(ratio, 160.0, Easing::Linear);
        }
    });
    view()
        .test_id("transfer-record")
        .gap(px(6.0))
        .py_3()
        .border_b_1()
        .border_color(appearance::divider)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .child(
                    text(move || details.get().map(|d| d.0).unwrap_or_default())
                        .number_of_lines(1)
                        .flex_1(),
                )
                .child(
                    text(move || details.get().map(|d| d.1).unwrap_or_default())
                        .number_of_lines(1)
                        .text_size(px(11.0))
                        .text_color(move || match details.get().map(|d| d.3) {
                            Some(TransferStatus::Completed) => {
                                theme().color("success.background").unwrap()
                            }
                            Some(TransferStatus::Failed(_)) => {
                                theme().color("destructive").unwrap()
                            }
                            _ => theme().muted_foreground,
                        }),
                )
                .child(when(
                    move || presentation.is_some(),
                    move || {
                        let dismiss_id = dismiss_id.clone();
                        control(
                            move || dismiss_language.get().text("action.dismiss"),
                            false,
                            move || {
                                if let Some(presentation) = presentation {
                                    presentation.dismissed.set(Some(dismiss_id.clone()));
                                }
                            },
                        )
                        .test_id("dismiss-received")
                    },
                )),
        )
        .child(
            text(move || details.get().map(|d| d.2).unwrap_or_default())
                .text_size(px(11.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(when(
            move || active.get(),
            move || {
                let cancel = cancel.clone();
                let id = cancel_id.clone();
                view()
                    .gap_2()
                    .child(
                        view()
                            .test_id("transfer-progress")
                            .h(px(3.0))
                            .bg(move || theme().border)
                            .child(
                                view()
                                    .h(px(3.0))
                                    .w_full()
                                    .bg(move || theme().color("progress.bar.background").unwrap())
                                    .transform_origin(nsh_core::TransformOrigin {
                                        x: nsh_core::geometry::DefiniteLength::Fraction(
                                            nsh_core::geometry::Percentage(0.0),
                                        ),
                                        ..Default::default()
                                    })
                                    .bind_transform(nsh_core::TransformKind::ScaleX, &progress),
                            ),
                    )
                    .child(view().flex_row().child(plain_control(
                        cancel.translated("action.cancel"),
                        false,
                        move || cancel.cancel(&id),
                    )))
            },
        ))
        .child(received_actions(session, id, presentation))
}

fn received_actions(
    session: Session,
    id: String,
    presentation: Option<ReceivedPresentation>,
) -> impl IntoElement {
    let paths = session.clone();
    let path_id = id.clone();
    let path_rows = session.clone();
    let language = session.language;
    let has_text = session.clone();
    let text_id = id.clone();
    let copy = session.clone();
    let show_text = session.clone();
    let show_id = id.clone();
    view()
        .gap_2()
        .child(each(
            move || {
                paths.revision.get();
                paths
                    .state
                    .borrow()
                    .transfers
                    .iter()
                    .find(|t| t.id == path_id)
                    .map(|t| t.paths.clone())
                    .unwrap_or_default()
            },
            Clone::clone,
            move |path, _| {
                let path = path.get_untracked();
                let opener = path_rows.clone();
                let revealer = path_rows.clone();
                let reveal_path = path.clone();
                let previewer = path_rows.clone();
                let preview_path = path.clone();
                let is_image = mime_guess::from_path(&path)
                    .first_raw()
                    .is_some_and(|mime| mime.starts_with("image/"));
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .child(when(move || is_image, {
                        let path = path.clone();
                        move || {
                            let uri = url::Url::from_file_path(&path)
                                .map(|uri| uri.to_string())
                                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
                            image(uri)
                                .test_id("received-image-thumbnail")
                                .size(px(48.0))
                                .when_some(presentation, |image, presentation| {
                                    image.w_full().h(move || {
                                        let chrome = if is_desktop() { 180.0 } else { 240.0 };
                                        px((presentation.height.get() - chrome).max(48.0))
                                    })
                                })
                                .flex_shrink_0()
                                .resize_mode(nsh_core::ResizeMode::Contain)
                        }
                    }))
                    .child(
                        text(
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                        )
                        .flex_1(),
                    )
                    .child(when(
                        move || is_image,
                        move || {
                            let previewer = previewer.clone();
                            let path = preview_path.clone();
                            control(previewer.translated("action.preview"), false, move || {
                                previewer.preview.set(Some(super::preview::Preview::Image(
                                    omasend::model::SendItem::File(path.clone()),
                                )));
                            })
                        },
                    ))
                    .child(control(
                        path_rows.translated(if cfg!(target_os = "ios") {
                            "action.share"
                        } else {
                            "action.open"
                        }),
                        false,
                        move || opener.open_path(path.clone(), false),
                    ))
                    .child(when(is_desktop, move || {
                        let revealer = revealer.clone();
                        let reveal_path = reveal_path.clone();
                        control(
                            language.get().text("action.show_in_files"),
                            false,
                            move || revealer.open_path(reveal_path.clone(), true),
                        )
                    }))
            },
        ))
        .child(when(
            move || {
                has_text.revision.get();
                has_text
                    .state
                    .borrow()
                    .transfers
                    .iter()
                    .any(|t| t.id == text_id && t.received_text.is_some())
            },
            move || {
                let copy = copy.clone();
                let id = id.clone();
                let show_text = show_text.clone();
                let show_id = show_id.clone();
                let expanded = signal(false);
                let language = session.language;
                view()
                    .gap_2()
                    .child(when(move || !expanded.get(), {
                        let show_text = show_text.clone();
                        let show_id = show_id.clone();
                        move || {
                            let show_text = show_text.clone();
                            let show_id = show_id.clone();
                            text(move || {
                                show_text.revision.get();
                                show_text
                                    .state
                                    .borrow()
                                    .transfers
                                    .iter()
                                    .find(|t| t.id == show_id)
                                    .and_then(|t| t.received_text.clone())
                                    .unwrap_or_default()
                            })
                            .number_of_lines(2)
                            .text_size(px(11.0))
                            .text_color(move || theme().muted_foreground)
                        }
                    }))
                    .child(
                        view()
                            .flex_row()
                            .gap_2()
                            .child(plain_control(
                                move || {
                                    language.get().text(if expanded.get() {
                                        "action.collapse"
                                    } else {
                                        "action.expand"
                                    })
                                },
                                false,
                                move || {
                                    if !reduce_motion() {
                                        animate_layout(nsh_core::LayoutAnimation::all(
                                            nsh_core::LayoutTransition::new(
                                                nsh_core::LayoutCurve::EaseOut,
                                                140.0,
                                            ),
                                        ));
                                    }
                                    expanded.set(!expanded.get());
                                },
                            ))
                            .child(plain_control(
                                session.translated("action.copy"),
                                false,
                                move || copy.copy_received(&id),
                            )),
                    )
                    .child(when(move || expanded.get(), {
                        let show_text = show_text.clone();
                        let show_id = show_id.clone();
                        let title_language = session.language;
                        move || {
                            let show_text = show_text.clone();
                            let show_id = show_id.clone();
                            super::editor()
                                .multiline(true)
                                .readonly(true)
                                .value(move || {
                                    show_text.revision.get();
                                    show_text
                                        .state
                                        .borrow()
                                        .transfers
                                        .iter()
                                        .find(|t| t.id == show_id)
                                        .and_then(|t| t.received_text.clone())
                                        .unwrap_or_default()
                                })
                                .h(move || px((window().height.0 * 0.22).clamp(72.0, 140.0)))
                                .label(move || title_language.get().text("message.title"))
                        }
                    }))
            },
        ))
}

fn size_label(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut index = 0;
    while value >= 1024.0 && index < units.len() - 1 {
        value /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", units[index])
    }
}

fn thumbnail(session: Session, id: String) -> impl IntoElement {
    let visible = session.clone();
    let check_id = id.clone();
    when(
        move || {
            visible.revision.get();
            visible
                .state
                .borrow()
                .composer
                .iter()
                .any(|item| item.id == check_id && item.is_image())
        },
        move || {
            let state = session.state.borrow();
            let path = state
                .composer
                .iter()
                .find(|item| item.id == id)
                .and_then(|item| item.item.path())
                .expect("image item has a path");
            let uri = url::Url::from_file_path(path)
                .map(|uri| uri.to_string())
                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
            image(uri)
                .size(px(48.0))
                .resize_mode(nsh_core::ResizeMode::Contain)
        },
    )
}
