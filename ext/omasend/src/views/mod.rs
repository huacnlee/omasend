//! Native Shell Rust UI. The protocol and transfer model remain platform independent.
pub(crate) mod appearance;
mod diagnostics;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod inputs;
#[cfg(target_os = "ios")]
#[path = "ios.rs"]
mod mobile;
#[cfg(target_os = "android")]
#[path = "android.rs"]
mod mobile;
mod motion;
mod panels;
mod preferences;
mod preview;
mod services;
mod session;
mod titlebar;
mod utilities;

use nsh_core::{keymap::When, prelude::*};
use session::Session;

pub(crate) fn control(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
) -> nsh_omarchy::ButtonElement {
    control_variant(title, disabled, run, nsh_omarchy::ButtonVariant::Default)
}

fn plain_control(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
) -> nsh_omarchy::ButtonElement {
    control_variant(title, disabled, run, nsh_omarchy::ButtonVariant::Ghost)
}

/// Every Omasend action is an Omarchy button; `variant` picks outline or ghost.
fn control_variant(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
    variant: nsh_omarchy::ButtonVariant,
) -> nsh_omarchy::ButtonElement {
    let title = title.into_reactive();
    let disabled = disabled.into_reactive();
    nsh_omarchy::button(move || title.get())
        .variant(variant)
        .disabled(move || disabled.get())
        .on_press(move |_: &()| run())
}

pub(super) fn editor() -> nsh_core::element::Input {
    nsh_core::element::input()
        .text_color(move || nsh_omarchy::tokens::foreground())
        .text_size(move || px(appearance::font_size()))
        .font_family(move || SharedString::from(appearance::metrics().family))
        .placeholder_color(move || color(nsh_omarchy::tokens::secondary()))
        .selection_color(move || color(nsh_omarchy::tokens::accent()))
}

/// Omasend's window: nearby devices, the Outbox and transfers.
#[component]
pub fn App() -> impl IntoElement {
    let appearance = appearance::install();
    let session = Session::persistent();
    session.start();
    session.start_services();
    shell(session, appearance)
}

fn shell(session: Session, appearance: appearance::Controller) -> impl IntoElement {
    let picker = session.clone();
    on_key("mod+o", move |_| picker.pick(false));
    let paste = session.clone();
    on_key_with(
        "mod+v",
        KeyOptions {
            when: When::NotEditing,
            ..Default::default()
        },
        move |_| paste.paste(),
    );
    #[cfg(target_os = "macos")]
    {
        let picker = session.clone();
        on_key("ctrl+o", move |_| picker.pick(false));
        let paste = session.clone();
        on_key_with(
            "ctrl+v",
            KeyOptions {
                when: When::NotEditing,
                ..Default::default()
            },
            move |_| paste.paste(),
        );
    }
    for (key, delta) in [("left", -1), ("right", 1)] {
        let navigation = session.clone();
        on_key_with(
            key,
            KeyOptions {
                when: When::NotEditing,
                ..Default::default()
            },
            move |_| navigation.navigate(delta),
        );
    }
    let send = session.clone();
    on_key_with(
        "enter",
        KeyOptions {
            when: When::NotEditing,
            ..Default::default()
        },
        move |_| send.send(),
    );
    let paste = session.clone();
    let sender = session.clone();
    let paste_disabled = session.clone();
    let send_disabled = session.clone();
    let target = session.clone();
    let status = session.clone();
    let busy = session.busy;
    let notice = session.notice;
    let notice_revision = session.notice_revision;
    let notice_opacity = animated_value(1.0);
    let confirmation = notice_opacity;
    effect(move || {
        notice_revision.get();
        if notice.get().is_some() && !reduce_motion() {
            confirmation.set(0.0);
            confirmation.timing(1.0, 140.0, Easing::Linear);
        } else {
            confirmation.set(1.0);
        }
    });
    let identity = session.identity;
    let language = session.language;
    let header_session = session.clone();
    let mobile_menu = session.clone();
    let content = view()
        .p_safe_area()
        .flex_1()
        .font_family(move || SharedString::from(appearance::metrics().family))
        .text_size(move || px(appearance::font_size()))
        .bg(move || nsh_omarchy::tokens::background())
        .text_color(move || nsh_omarchy::tokens::foreground())
        .child(when(is_desktop, {
            let session = session.clone();
            move || titlebar::bar(session.clone())
        }))
        .child(panels::error_notice(session.clone()))
        .child(panels::nearby(session.clone()))
        .child(
            view()
                .flex_1()
                .test_id("outbox-panel")
                .px(px(16.0))
                .pt(px(16.0))
                .pb(px(12.0))
                .gap_3()
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .items_center()
                        .child(text(session.translated("composer.outbox")))
                        .child(
                            text(move || {
                                target.revision.get();
                                let state = target.state.borrow();
                                state
                                    .selected_device()
                                    .map(|d| language.get().named("history.to_lower", &d.alias))
                                    .unwrap_or_else(|| {
                                        language.get().text("composer.choose_device")
                                    })
                            })
                            .text_color(move || nsh_omarchy::tokens::secondary())
                            .flex_1(),
                        )
                        .child(text(move || {
                            if busy.get() {
                                language.get().text("status.preparing")
                            } else {
                                String::new()
                            }
                        })),
                )
                .child(panels::outbox(session.clone()))
                .child(
                    view()
                        .test_id("outbox-actions")
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(utilities::add_files_button(session.clone()))
                        .child(plain_control(
                            session.translated("action.paste"),
                            move || paste_disabled.input_blocked(),
                            move || paste.paste(),
                        ))
                        .child(view().flex_1())
                        .child(when(is_desktop, {
                            let history = session.clone();
                            move || utilities::history_button(history.clone())
                        }))
                        .child(
                            control(
                                session.translated("action.send"),
                                move || send_disabled.cannot_send(),
                                move || sender.send(),
                            )
                            .when(!is_desktop(), |v| {
                                v.border_color(move || nsh_omarchy::tokens::accent())
                                    .text_color(move || nsh_omarchy::tokens::accent())
                            }),
                        ),
                )
                .child(panels::active(session.clone())),
        )
        .child(
            view()
                .test_id("status-bar")
                .h(px(28.0))
                .px(px(16.0))
                .flex_shrink_0()
                .flex_row()
                .items_center()
                .gap_3()
                .border_t_1()
                .border_color(appearance::divider)
                .text_size(px(11.0))
                .text_color(move || nsh_omarchy::tokens::secondary())
                .child(
                    text(move || {
                        identity.get().map(|d| d.alias).unwrap_or_else(|| {
                            gethostname::gethostname().to_string_lossy().into_owned()
                        })
                    })
                    .flex_1(),
                )
                .child(
                    text(move || {
                        notice
                            .get()
                            .map(|key| language.get().text(key))
                            .unwrap_or_default()
                    })
                    .text_color(move || nsh_omarchy::tokens::success())
                    .bind_opacity(&notice_opacity),
                )
                .child(services::update_control(session.clone()))
                .child(text(move || {
                    status.revision.get();
                    language.get().text(if identity.get().is_none() {
                        "status.connecting"
                    } else if status.state.borrow().devices.is_empty() {
                        "status.searching"
                    } else {
                        "status.connected"
                    })
                })),
        )
        .child(panels::history_overlay(session.clone()))
        .child(preview::overlay(session.clone()))
        .child(utilities::overlay(session, appearance));
    if is_desktop() {
        content.into_element()
    } else {
        view()
            .flex_1()
            .child(titlebar::mobile_page(header_session, content))
            .child(utilities::menu_button(mobile_menu))
            .into_element()
    }
}

#[cfg(all(test, feature = "ui-tests"))]
mod integration_tests;
