//! Auxiliary surfaces share one modal, so menus never stack over previews.
use super::{
    appearance::{self, Controller, Mode, theme},
    control, editor,
    session::Session,
};
use nsh_core::prelude::*;
use omasend::{i18n::Language, updates::UpdateState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Surface {
    Menu,
    Logs,
    About,
}
impl Surface {
    fn title(self) -> &'static str {
        match self {
            Self::Menu => "action.menu",
            Self::Logs => "logs.title",
            Self::About => "action.about",
        }
    }
}

pub(super) fn overlay(session: Session, _appearance: Controller) -> impl IntoElement {
    let surface = session.surface;
    let maker = session;
    view().child(when(
        move || surface.get().is_some_and(|s| s != Surface::Menu),
        move || {
            let session = maker.clone();
            let close = session.clone();
            let title_language = session.language;
            let sections = session.clone();
            nsh_base::dialog()
                .animation(if reduce_motion() {
                    ModalAnimation::None
                } else {
                    ModalAnimation::Fade
                })
                .on_cancel(move |_: &()| surface.set(None))
                .child(
                    view()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .p_4()
                        .bg(AdaptiveColor::from(rgb(0)).opacity(0.6))
                        .child(
                            view()
                                .w_full()
                                .max_w(move || {
                                    px(if surface.get() == Some(Surface::Logs) {
                                        720.0
                                    } else {
                                        420.0
                                    })
                                })
                                .p_4()
                                .gap_4()
                                .border_1()
                                .font_family(move || {
                                    SharedString::from(appearance::metrics().family)
                                })
                                .text_size(move || px(theme().number("font.size").unwrap()))
                                .bg(appearance::panel_background)
                                .text_color(move || theme().foreground)
                                .border_color(move || theme().border)
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            text(move || {
                                                title_language.get().text(
                                                    surface.get().unwrap_or(Surface::Menu).title(),
                                                )
                                            })
                                            .flex_1()
                                            .font_weight(FontWeight::BOLD),
                                        )
                                        .child(control(
                                            session.translated("action.close"),
                                            false,
                                            move || close.surface.set(None),
                                        )),
                                )
                                .child(
                                    scroll()
                                        .max_h(move || px((window().height.0 - 120.0).max(100.0)))
                                        .child(dynamic(move || {
                                            match surface.get().unwrap_or(Surface::Menu) {
                                                Surface::Menu => view().into_element(),
                                                Surface::Logs => {
                                                    logs(sections.clone()).into_element()
                                                }
                                                Surface::About => {
                                                    about(sections.clone()).into_element()
                                                }
                                            }
                                        })),
                                ),
                        ),
                )
        },
    ))
}

#[derive(Clone, Copy)]
enum ToolbarIcon {
    Menu,
    History,
    Copy,
    Close,
}

fn icon_button(
    label: impl IntoReactive<SharedString>,
    icon: ToolbarIcon,
    test_id: Option<&str>,
    run: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    let label = label.into_reactive();
    let (target, glyph) = if is_desktop() {
        (28.0, 12.0)
    } else {
        (44.0, 16.0)
    };
    let path = match icon {
        ToolbarIcon::Menu => "M0.75 3H17.25 M0.75 9H17.25 M0.75 15H17.25",
        ToolbarIcon::History => {
            "M9 1.5 A7.5 7.5 0 1 1 1.5 9 A7.5 7.5 0 0 1 9 1.5 M9 4.5 L9 9 L12.5 10.5"
        }
        ToolbarIcon::Copy => "M5.5 5.5H16.5V16.5H5.5Z M12.5 1.5H1.5V12.5",
        ToolbarIcon::Close => "M1.5 1.5L16.5 16.5 M16.5 1.5L1.5 16.5",
    };
    let mut content = view()
        .size(px(target))
        .window_drag(false)
        .items_center()
        .justify_center()
        .child(
            canvas(move || {
                Painter::new()
                    .scale(glyph / 18.0, glyph / 18.0)
                    .stroke_color(color(theme().foreground))
                    .line_width(1.5)
                    .stroke_svg(path)
            })
            .size(px(glyph))
            .flex_shrink_0(),
        );
    if let Some(id) = test_id {
        content = content.test_id(id);
    }
    nsh_base::button()
        .accessibility_label(move || Some(label.get()))
        .on_press(run)
        .size(px(target))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded_none()
        .hover(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .focus(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .active(|s| s.bg(move || theme().foreground.opacity(0.22)))
        .child(content)
}

pub(super) fn close_button(
    label: impl IntoReactive<SharedString>,
    run: impl Fn() + 'static,
) -> impl IntoElement {
    icon_button(label, ToolbarIcon::Close, None, run)
}

pub(super) fn copy_error_button(session: Session) -> impl IntoElement {
    icon_button(
        session.translated("action.copy_error"),
        ToolbarIcon::Copy,
        Some("copy-error"),
        move || {
            if let Some(error) = session.state.borrow().error.clone() {
                set_clipboard(error);
                session.notify("status.copied");
            }
        },
    )
}

pub(super) fn history_button(session: Session) -> impl IntoElement {
    icon_button(
        session.translated("history.title"),
        ToolbarIcon::History,
        Some("history-button"),
        move || session.open_history(),
    )
}

pub(super) fn mobile_menu_trigger(session: Session) -> impl IntoElement {
    let surface = session.surface;
    icon_button(
        session.translated("action.menu"),
        ToolbarIcon::Menu,
        Some("app-menu"),
        move || {
            if surface.get().is_none() {
                surface.set(Some(Surface::Menu));
            } else if surface.get() == Some(Surface::Menu) {
                surface.set(None);
            }
        },
    )
    .selected(move || surface.get() == Some(Surface::Menu))
    .selected_style(|s| s.bg(move || theme().foreground.opacity(0.08)))
}

fn desktop_menu_trigger(session: Session) -> nsh_base::ButtonElement {
    let surface = session.surface;
    icon_button(
        session.translated("action.menu"),
        ToolbarIcon::Menu,
        Some("app-menu"),
        move || {
            if surface.get() == Some(Surface::Menu) {
                surface.set(None);
            } else if surface.get().is_none()
                && session.state.borrow().incoming.is_none()
                && session.preview.get().is_none()
                && !session.history_visible.get()
            {
                surface.set(Some(Surface::Menu));
            }
        },
    )
    .selected(move || surface.get() == Some(Surface::Menu))
    .selected_style(|s| s.bg(move || theme().foreground.opacity(0.08)))
}

pub(super) fn menu_button(session: Session) -> impl IntoElement {
    let appearance = use_context::<Controller>().expect("appearance controller");
    let surface = session.surface;
    let guard = session.clone();
    let content = session.clone();
    let trigger = if is_desktop() {
        desktop_menu_trigger(session).into_element()
    } else {
        view().hidden().into_element()
    };
    nsh_base::menu()
        .open(move || Some(surface.get() == Some(Surface::Menu)))
        .on_open_change(move |event: &nsh_base::MenuEvent| {
            let nsh_base::MenuEvent::OpenChange(open, _) = event else {
                return;
            };
            if *open {
                if guard.state.borrow().incoming.is_none()
                    && guard.preview.get().is_none()
                    && !guard.history_visible.get()
                    && surface.get().is_none()
                {
                    surface.set(Some(Surface::Menu));
                }
            } else if surface.get() == Some(Surface::Menu) {
                surface.set(None);
            }
        })
        .placement(Placement::Bottom)
        .align(Align::End)
        .offset(px(6.0))
        .margin(px(8.0))
        .popup_style(|s| {
            s.p_1()
                .pb(move || {
                    px(if is_desktop() {
                        4.0
                    } else {
                        safe_area().bottom.0 + 8.0
                    })
                })
                .border_1()
                .border_color(appearance::divider)
                .bg(appearance::panel_background)
                .text_color(move || theme().foreground)
                .font_family(move || SharedString::from(appearance::metrics().family))
                .text_size(move || px(theme().number("font.size").unwrap()))
                .rounded_none()
        })
        .backdrop_style(|s| s.bg(AdaptiveColor::from(rgb(0)).opacity(0.6)))
        .trigger(trigger)
        .child(menu_contents(content, appearance))
}

pub(super) fn add_files_button(session: Session) -> impl IntoElement {
    let disabled = session.clone();
    let picker = session.clone();
    if is_desktop() {
        return control(
            session.translated("action.add_files_ellipsis"),
            move || disabled.input_blocked(),
            move || picker.pick(false),
        )
        .into_element();
    }
    let open = signal(false);
    let pressed = signal(false);
    nsh_base::menu()
        .open(move || Some(open.get()))
        .on_open_change(move |event: &nsh_base::MenuEvent| {
            if let nsh_base::MenuEvent::OpenChange(next, _) = event {
                open.set(*next);
            }
        })
        .popup_style(|s| {
            s.p_2()
                .pb(move || px(safe_area().bottom.0 + 8.0))
                .border_1()
                .border_color(appearance::divider)
                .bg(appearance::panel_background)
                .text_color(move || theme().foreground)
                .font_family(move || SharedString::from(appearance::metrics().family))
                .text_size(move || px(theme().number("font.size").unwrap()))
        })
        .backdrop_style(|s| s.bg(AdaptiveColor::from(rgb(0)).opacity(0.6)))
        .trigger(
            control(
                session.translated("action.add_files_ellipsis"),
                move || disabled.input_blocked(),
                move || open.set(!open.get()),
            )
            .test_id("add-files-button")
            .on_press_down(move |_: &()| pressed.set(true))
            .on_press_up(move |_: &()| pressed.set(false))
            .bg(move || {
                theme().foreground.opacity(if pressed.get() {
                    0.22
                } else if open.get() {
                    0.18
                } else {
                    0.0
                })
            }),
        )
        .child(media_sources(session))
        .into_element()
}

fn media_sources(session: Session) -> impl IntoElement {
    let photos = session.clone();
    let files = session.clone();
    view()
        .test_id("add-files-menu")
        .child(menu_row(
            session.translated("action.photo_library"),
            false,
            move || photos.pick_photos(),
        ))
        .child(menu_row(
            session.translated("action.browse_files"),
            false,
            move || files.pick(false),
        ))
}

fn menu_row(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
) -> nsh_base::MenuItemElement {
    let title = title.into_reactive();
    let label = title.clone();
    nsh_base::menu_item()
        .accessibility_label(move || Some(label.get()))
        .disabled(disabled.into_reactive())
        .on_select(run)
        .flex_row()
        .items_center()
        .gap_2()
        .justify_start()
        .h(move || px(if is_desktop() { 28.0 } else { 44.0 }))
        .px(px(8.0))
        .py(px(4.0))
        .hover(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .focus(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .active(|s| s.bg(move || theme().foreground.opacity(0.22)))
        .child(text(title))
}

pub(super) fn check_icon() -> impl IntoElement {
    canvas(move || {
        Painter::new()
            .stroke_color(color(theme().foreground))
            .line_width(1.5)
            .stroke_svg("M2 6L5 9L11 3")
    })
    .size(px(12.0))
}

fn checked_menu_row(
    title: impl IntoReactive<SharedString>,
    checked: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
) -> nsh_base::MenuItemElement {
    let title = title.into_reactive();
    let label = title.clone();
    let checked = checked.into_reactive();
    let state = checked.clone();
    menu_row(title, false, run)
        .accessibility_label(move || {
            Some(if state.get() {
                SharedString::from(format!("{} ✓", label.get()))
            } else {
                label.get()
            })
        })
        .child(view().flex_1().items_end().child(when(
            move || checked.get(),
            || view().test_id("menu-check-icon").child(check_icon()),
        )))
}

fn submenu_trigger(title: impl IntoReactive<SharedString>) -> nsh_core::element::View {
    let title = title.into_reactive();
    view()
        .label(title.clone())
        .flex_row()
        .items_center()
        .gap_2()
        .h(move || px(if is_desktop() { 28.0 } else { 44.0 }))
        .px(px(8.0))
        .py(px(4.0))
        .hover(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .focus(|s| s.bg(move || theme().foreground.opacity(0.08)))
        .active(|s| s.bg(move || theme().foreground.opacity(0.22)))
        .child(text(title))
        .child(text("›").flex_1().text_right())
}

fn menu_back(session: Session) -> impl IntoElement {
    view()
        .test_id("menu-back")
        .text_color(move || color(theme().foreground))
        .h(px(44.0))
        .px(px(8.0))
        .justify_center()
        .label(session.translated("action.back"))
        .child(text(move || {
            format!("‹ {}", session.language.get().text("action.back"))
        }))
}

fn submenu_contents(content: impl IntoElement) -> impl IntoElement {
    view().child(content)
}

fn divider() -> impl IntoElement {
    nsh_base::menu_separator()
        .h(px(1.0))
        .my(px(3.0))
        .bg(appearance::divider)
}

fn menu_contents(session: Session, appearance: Controller) -> impl IntoElement {
    let paste = session.clone();
    let files = session.clone();
    let check = session.clone();
    let about = session.clone();
    let logs = session.clone();
    let history = session.clone();
    let surface = session.surface;
    let language = session.language;
    let busy = session.busy;
    let status = session.update;
    let mut menu = view()
        .test_id("app-menu-popup")
        .when(is_desktop(), |v| v.w(px(248.0)))
        .child(
            menu_row(
                session.translated("action.paste"),
                move || busy.get(),
                move || {
                    surface.set(None);
                    paste.paste();
                },
            )
            .child(
                text(move || if is_desktop() { "ctrl+v" } else { "" })
                    .flex_1()
                    .text_right()
                    .text_color(move || theme().muted_foreground),
            ),
        )
        .child(if is_desktop() {
            menu_row(
                session.translated("action.add_files_ellipsis"),
                move || busy.get(),
                move || {
                    surface.set(None);
                    files.pick(false);
                },
            )
            .child(
                text(move || if is_desktop() { "ctrl+o" } else { "" })
                    .flex_1()
                    .text_right()
                    .text_color(move || theme().muted_foreground),
            )
            .into_element()
        } else {
            nsh_base::submenu()
                .when(!is_desktop(), |v| v.flex_col_reverse())
                .trigger(submenu_trigger(
                    session.translated("action.add_files_ellipsis"),
                ))
                .back(menu_back(session.clone()))
                .child(submenu_contents(media_sources(session.clone())))
                .into_element()
        })
        .child(when(
            || !is_desktop(),
            move || {
                let history = history.clone();
                menu_row(language.get().text("history.title"), false, move || {
                    history.open_history()
                })
            },
        ))
        .child(divider())
        .child(
            nsh_base::submenu()
                .when(!is_desktop(), |v| v.flex_col_reverse())
                .trigger(submenu_trigger(session.translated("action.language")))
                .back(menu_back(session.clone()))
                .child(submenu_contents(language_menu(session.clone()))),
        )
        .child(if appearance::is_omarchy() {
            let chooser = appearance.clone();
            menu_row(session.translated("theme.reload"), false, move || {
                chooser.reload()
            })
            .into_element()
        } else {
            nsh_base::submenu()
                .when(!is_desktop(), |v| v.flex_col_reverse())
                .trigger(
                    submenu_trigger(session.translated("action.theme"))
                        .test_id("menu-theme-trigger"),
                )
                .back(menu_back(session.clone()))
                .child(submenu_contents(theme_menu(
                    session.clone(),
                    appearance.clone(),
                )))
                .into_element()
        })
        .child(when(is_desktop, divider))
        .child(when(is_desktop, move || {
            let check = check.clone();
            menu_row(
                language.get().text("action.check_update"),
                move || {
                    matches!(
                        status.get(),
                        UpdateState::Checking | UpdateState::Installing { .. }
                    )
                },
                move || {
                    surface.set(None);
                    check.check_updates();
                },
            )
        }))
        .child(divider())
        .child(menu_row(
            session.translated("action.about"),
            false,
            move || about.open_surface(Surface::About),
        ));
    for (title, url) in [
        ("Omasend…", "https://huacnlee.github.io/omasend"),
        ("GitHub…", "https://github.com/huacnlee/omasend"),
        ("LocalSend…", "https://localsend.org"),
    ] {
        let opener = session.clone();
        menu = menu.child(menu_row(title, false, move || {
            surface.set(None);
            opener.open_url(url.into());
        }));
    }
    menu.child(divider())
        .child(menu_row(
            move || language.get().text("action.logs"),
            false,
            move || logs.open_surface(Surface::Logs),
        ))
        .child(when(is_desktop, move || {
            let session = session.clone();
            view().child(divider()).child(menu_row(
                session.translated("action.exit"),
                false,
                move || session.exit(),
            ))
        }))
}

fn theme_menu(session: Session, appearance: Controller) -> impl IntoElement {
    let mut modes = view().test_id("app-submenu-popup");
    for mode in [Mode::System, Mode::Light, Mode::Dark] {
        let chooser = appearance.clone();
        let selected = appearance.mode;
        let errors = session.clone();
        let language = session.language;
        modes = modes.child(checked_menu_row(
            move || language.get().text(mode.label()),
            move || selected.get() == mode,
            move || {
                if let Err(error) = chooser.select(mode) {
                    errors.fail(error);
                }
                errors.surface.set(None);
            },
        ));
    }
    if is_desktop() {
        modes.child(divider()).child(menu_row(
            session.translated("theme.reload"),
            false,
            move || appearance.reload(),
        ))
    } else {
        modes
    }
}

fn language_menu(session: Session) -> impl IntoElement {
    let mut menu = view().test_id("app-submenu-popup");
    for (title, language) in [("English", Language::En), ("简体中文", Language::ZhCn)] {
        let chooser = session.clone();
        let selected = session.language;
        menu = menu.child(checked_menu_row(
            title,
            move || selected.get() == language,
            move || chooser.set_language(language),
        ));
    }
    menu
}

fn logs(session: Session) -> impl IntoElement {
    let snapshot = session.logs;
    let language = session.language;
    let notice = session.notice;
    let refresh = session.clone();
    let copy = session.clone();
    let clear = session.clone();
    view()
        .gap_3()
        .child(
            editor()
                .value(move || snapshot.get())
                .placeholder(move || language.get().text("logs.empty"))
                .readonly(true)
                .multiline(true)
                .h(move || px((window().height.0 - 240.0).clamp(100.0, 420.0)))
                .label(session.translated("logs.title")),
        )
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(control(
                    session.translated("action.refresh"),
                    false,
                    move || refresh.refresh_logs(),
                ))
                .child(control(
                    session.translated("action.copy"),
                    move || snapshot.get().is_empty(),
                    move || {
                        set_clipboard(copy.logs.get());
                        copy.notify("logs.copied");
                    },
                ))
                .child(control(
                    session.translated("action.clear"),
                    move || snapshot.get().is_empty(),
                    move || {
                        omasend::diagnostics::clear();
                        clear.refresh_logs();
                    },
                )),
        )
        .child(
            text(move || {
                notice
                    .get()
                    .map(|key| language.get().text(key))
                    .unwrap_or_default()
            })
            .text_color(move || theme().color("success.background").unwrap()),
        )
}

fn about(session: Session) -> impl IntoElement {
    let language = session.language;
    let source = session.clone();
    let localsend = session.clone();
    view()
        .gap_4()
        .child(
            text(concat!("Omasend · v", env!("CARGO_PKG_VERSION"))).font_weight(FontWeight::BOLD),
        )
        .child(text(move || {
            language.get().named(
                "about.protocol",
                localsend::model::discovery::PROTOCOL_VERSION_V2,
            )
        }))
        .child(text(session.translated("about.credit")))
        .child(
            text(session.translated("about.native_shell"))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(control("localsend.org", false, move || {
                    localsend.open_url("https://localsend.org".into())
                }))
                .child(control(
                    session.translated("about.source"),
                    false,
                    move || source.open_url("https://github.com/huacnlee/omasend".into()),
                )),
        )
}
