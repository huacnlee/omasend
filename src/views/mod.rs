//! Native Shell Rust UI. The protocol and transfer model remain platform independent.
pub(crate) mod appearance;
mod inputs;
mod motion;
mod panels;
mod preferences;
mod preview;
mod services;
mod session;
mod titlebar;
mod utilities;

use appearance::theme;
use nsh_core::{keymap::When, ui::prelude::*};
use session::Session;

pub(crate) fn control(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
) -> nsh_core::ui::View {
    control_variant(title, disabled, run, true)
}

fn plain_control(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
) -> nsh_core::ui::View {
    control_variant(title, disabled, run, false)
}

fn control_variant(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
    outlined: bool,
) -> nsh_core::ui::View {
    let title = title.into_reactive();
    let scale = native_value(1.0);
    let down = scale.clone();
    let up = scale.clone();
    let standalone = !appearance::is_omarchy();
    let button = view()
        .flex_row()
        .items_center()
        .justify_center()
        .px(move || px(appearance::metrics().space(10.0)))
        .py(move || px(appearance::metrics().space(6.0)))
        .border_1()
        .border_color(move || {
            if outlined && standalone {
                theme().border
            } else if outlined {
                theme().foreground.opacity(0.4)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .rounded(px(if standalone { 4.0 } else { 0.0 }))
        .bg(move || {
            if outlined && standalone {
                theme().secondary
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .focus(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .active(|s| s.bg(move || theme().foreground.opacity(0.22)))
        .disabled_style(|s| s.opacity(0.45))
        .disabled(disabled)
        .window_drag(false)
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .bind_scale(&scale)
        .on_press_in(move |_: &()| {
            if !reduce_motion() {
                down.timing(0.98, 70.0, Easing::Linear);
            }
        })
        .on_press_out(move |_: &()| {
            up.timing(
                1.0,
                if reduce_motion() { 0.0 } else { 110.0 },
                Easing::Linear,
            );
        })
        .on_press(move |_: &()| run())
        .child(text(title).text_size(move || px(theme().font_size)));
    if standalone && outlined {
        button.shadow_sm().inset_shadow_sm()
    } else {
        button
    }
}

pub(super) fn editor() -> nsh_core::ui::Input {
    input()
        .text_color(move || theme().foreground)
        .text_size(move || px(theme().font_size))
        .font_family(move || SharedString::from(appearance::metrics().family))
        .placeholder_color(move || color(theme().muted_foreground))
        .selection_color(move || color(theme().caret))
}

#[component]
pub fn App() -> impl IntoElement {
    let appearance = appearance::install();
    let session = Session::persistent();
    session.start();
    session.start_services();
    shell(session, appearance)
}

fn shell(session: Session, appearance: appearance::Controller) -> impl IntoElement {
    titlebar::install();
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
    let files = session.clone();
    let paste = session.clone();
    let sender = session.clone();
    let input_disabled = session.clone();
    let paste_disabled = session.clone();
    let send_disabled = session.clone();
    let target = session.clone();
    let status = session.clone();
    let errors = session.clone();
    let busy = session.busy;
    let notice = session.notice;
    let identity = session.identity;
    let language = session.language;
    let history = session.clone();
    view()
        .flex_1()
        .font_family(move || SharedString::from(appearance::metrics().family))
        .text_size(move || px(theme().font_size))
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(titlebar::bar(session.clone()))
        .child(If::new(
            move || {
                errors.revision.get();
                errors.state.borrow().error.is_some()
            },
            {
                let session = session.clone();
                move || {
                    let labels = session.clone();
                    text(move || {
                        labels.revision.get();
                        labels
                            .state
                            .borrow()
                            .error
                            .as_ref()
                            .map(|e| labels.language.get().error(e))
                            .unwrap_or_default()
                    })
                    .px(px(16.0))
                    .py(px(8.0))
                    .text_color(move || theme().danger)
                }
            },
        ))
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
                            .text_color(move || theme().muted_foreground)
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
                        .gap_3()
                        .child(control(
                            session.translated("action.add_files_ellipsis"),
                            move || input_disabled.input_blocked(),
                            move || files.pick(false),
                        ))
                        .child(plain_control(
                            session.translated("action.paste"),
                            move || paste_disabled.input_blocked(),
                            move || paste.paste(),
                        ))
                        .child(view().flex_1())
                        .child(
                            plain_control("◴", false, move || history.open_history())
                                .label(session.translated("history.title")),
                        )
                        .child(control(
                            session.translated("action.send"),
                            move || send_disabled.cannot_send(),
                            move || sender.send(),
                        )),
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
                .text_color(move || theme().muted_foreground)
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
                    .text_color(move || theme().success),
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
        .child(utilities::overlay(session, appearance))
}

#[cfg(all(test, feature = "ui-tests"))]
mod integration_tests;
