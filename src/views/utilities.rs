//! Auxiliary surfaces share one modal, so menus never stack over previews.
use super::{
    appearance::{self, Controller, Mode, theme},
    control, editor, plain_control,
    session::Session,
};
use nsh_core::ui::{desktop::title_bar, prelude::*};
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

pub(super) fn overlay(session: Session, appearance: Controller) -> impl IntoElement {
    let surface = session.surface;
    let popup = menu_popup(session.clone(), appearance.clone());
    let maker = session;
    view().child(popup).child(If::new(
        move || surface.get().is_some_and(|s| s != Surface::Menu),
        move || {
            let session = maker.clone();
            let close = session.clone();
            let title_language = session.language;
            let sections = session.clone();
            modal()
                .transparent(true)
                .animation(if reduce_motion() {
                    ModalAnimation::None
                } else {
                    ModalAnimation::Fade
                })
                .on_request_close(move |_: &()| surface.set(None))
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
                                .text_size(move || px(theme().font_size))
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
                                        .max_h(move || px((window().height - 120.0).max(100.0)))
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Submenu {
    Language,
    Theme,
}

fn menu_popup(session: Session, appearance: Controller) -> impl IntoElement {
    let surface = session.surface;
    If::new(
        move || surface.get() == Some(Surface::Menu),
        move || {
            let selected = signal(None::<Submenu>);
            let session = session.clone();
            let appearance = appearance.clone();
            modal()
                .transparent(true)
                .animation(if reduce_motion() {
                    ModalAnimation::None
                } else {
                    ModalAnimation::Fade
                })
                .on_request_close(move |_: &()| surface.set(None))
                .child(
                    view()
                        .flex_1()
                        .relative()
                        .font_family(move || SharedString::from(appearance::metrics().family))
                        .text_size(move || px(theme().font_size))
                        .text_color(move || theme().foreground)
                        .child(
                            view()
                                .absolute()
                                .inset(px(0.0))
                                .label(session.translated("action.close"))
                                .on_press(move |_: &()| surface.set(None)),
                        )
                        .child(
                            view()
                                .absolute()
                                .top(move || px(appearance::metrics().space(42.0)))
                                .right(move || px(16.0 + title_bar().right))
                                .child(menu(session.clone(), selected))
                                .child(If::new(
                                    move || selected.get().is_some(),
                                    move || {
                                        let session = session.clone();
                                        let appearance = appearance.clone();
                                        on_key("left", move |_| selected.set(None));
                                        view()
                                            .absolute()
                                            .test_id("app-submenu-popup")
                                            .right(px(248.0))
                                            .top(move || {
                                                px(if selected.get() == Some(Submenu::Language) {
                                                    62.0
                                                } else {
                                                    92.0
                                                })
                                            })
                                            .w(px(188.0))
                                            .p_1()
                                            .border_1()
                                            .border_color(move || theme().foreground.opacity(0.4))
                                            .bg(appearance::panel_background)
                                            .child(dynamic(move || match selected.get() {
                                                Some(Submenu::Theme) => {
                                                    theme_menu(session.clone(), appearance.clone())
                                                        .into_element()
                                                }
                                                _ => language_menu(session.clone()).into_element(),
                                            }))
                                    },
                                )),
                        ),
                )
        },
    )
}

#[derive(Clone)]
struct MenuKeys {
    cursor: Signal<Option<usize>>,
    entries: std::rc::Rc<std::cell::RefCell<Vec<MenuEntry>>>,
}
struct MenuEntry {
    disabled: Reactive<bool>,
    action: std::rc::Rc<dyn Fn()>,
    element: ElementRef,
}
impl MenuKeys {
    fn install() {
        let navigation = Self {
            cursor: signal(None),
            entries: Default::default(),
        };
        provide_context(navigation.clone());
        for (key, delta) in [("down", 1_isize), ("up", -1)] {
            let navigation = navigation.clone();
            on_key(key, move |_| {
                let entries = navigation.entries.borrow();
                if entries.is_empty() {
                    return;
                }
                let count = entries.len() as isize;
                let start = navigation
                    .cursor
                    .get()
                    .map(|i| i as isize)
                    .unwrap_or(if delta > 0 { -1 } else { 0 });
                for step in 1..=count {
                    let index = (start + delta * step).rem_euclid(count) as usize;
                    if !entries[index].disabled.get() {
                        navigation.cursor.set(Some(index));
                        entries[index].element.set_accessibility_focus();
                        break;
                    }
                }
            });
        }
        for key in ["enter", "space"] {
            let navigation = navigation.clone();
            on_key(key, move |_| {
                let action = {
                    let entries = navigation.entries.borrow();
                    navigation
                        .cursor
                        .get()
                        .and_then(|i| entries.get(i))
                        .filter(|entry| !entry.disabled.get())
                        .map(|entry| entry.action.clone())
                };
                if let Some(action) = action {
                    action();
                }
            });
        }
    }
}

fn menu_row(
    title: impl IntoReactive<SharedString>,
    disabled: impl IntoReactive<bool>,
    run: impl Fn() + 'static,
) -> nsh_core::ui::View {
    let disabled = disabled.into_reactive();
    let run: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(run);
    let callback = run.clone();
    let row = plain_control(title, disabled.clone(), move || callback())
        .justify_start()
        .h(px(28.0))
        .px(px(8.0))
        .py(px(4.0));
    if let Some(navigation) = use_context::<MenuKeys>() {
        let element = element_ref();
        let index = navigation.entries.borrow().len();
        navigation.entries.borrow_mut().push(MenuEntry {
            disabled,
            action: run,
            element: element.clone(),
        });
        row.element_ref(&element).bg(move || {
            if navigation.cursor.get() == Some(index) {
                theme().foreground.opacity(0.08)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
    } else {
        row
    }
}

fn divider() -> impl IntoElement {
    view().h(px(1.0)).my(px(3.0)).bg(appearance::divider)
}

fn menu(session: Session, selected: Signal<Option<Submenu>>) -> impl IntoElement {
    MenuKeys::install();
    let paste = session.clone();
    let files = session.clone();
    let check = session.clone();
    let about = session.clone();
    let logs = session.clone();
    let surface = session.surface;
    let language = session.language;
    let busy = session.busy;
    let status = session.update;
    let mut menu = view()
        .test_id("app-menu-popup")
        .w(px(248.0))
        .p_1()
        .border_1()
        .border_color(move || theme().foreground.opacity(0.4))
        .bg(appearance::panel_background)
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
                text("ctrl+v")
                    .flex_1()
                    .text_right()
                    .text_color(move || theme().muted_foreground),
            ),
        )
        .child(
            menu_row(
                session.translated("action.add_files_ellipsis"),
                move || busy.get(),
                move || {
                    surface.set(None);
                    files.pick(false);
                },
            )
            .child(
                text("ctrl+o")
                    .flex_1()
                    .text_right()
                    .text_color(move || theme().muted_foreground),
            ),
        )
        .child(divider())
        .child(
            menu_row(session.translated("action.language"), false, move || {
                selected.set(Some(Submenu::Language))
            })
            .child(text("›").flex_1().text_right()),
        )
        .child(
            menu_row(
                move || {
                    language.get().text(if appearance::is_omarchy() {
                        "theme.reload"
                    } else {
                        "action.theme"
                    })
                },
                false,
                move || {
                    if appearance::is_omarchy() {
                        appearance::reload_system_theme();
                    } else {
                        selected.set(Some(Submenu::Theme));
                    }
                },
            )
            .child(
                text(|| if appearance::is_omarchy() { "" } else { "›" })
                    .flex_1()
                    .text_right(),
            ),
        )
        .child(divider())
        .child(menu_row(
            session.translated("action.check_update"),
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
        ))
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
        .child(divider())
        .child(menu_row(
            session.translated("action.exit"),
            false,
            move || session.exit(),
        ))
}

fn theme_menu(session: Session, appearance: Controller) -> impl IntoElement {
    MenuKeys::install();
    let mut modes = view();
    for mode in [Mode::System, Mode::Light, Mode::Dark] {
        let chooser = appearance.clone();
        let selected = appearance.mode;
        let errors = session.clone();
        let language = session.language;
        modes = modes.child(
            menu_row(
                move || -> SharedString {
                    format!(
                        "{}{}",
                        if selected.get() == mode { "✓ " } else { "" },
                        language.get().text(mode.label())
                    )
                    .into()
                },
                false,
                move || {
                    if let Err(error) = chooser.select(mode) {
                        errors.fail(error);
                    }
                    errors.surface.set(None);
                },
            )
            .accessibility_state(move || AccessibilityState {
                selected: selected.get() == mode,
                ..Default::default()
            }),
        );
    }
    modes.child(divider()).child(menu_row(
        session.translated("theme.reload"),
        false,
        move || appearance.reload(),
    ))
}

fn language_menu(session: Session) -> impl IntoElement {
    MenuKeys::install();
    let mut menu = view();
    for (title, language) in [("English", Language::En), ("简体中文", Language::ZhCn)] {
        let chooser = session.clone();
        let selected = session.language;
        menu = menu.child(
            menu_row(
                move || -> SharedString {
                    format!(
                        "{}{}",
                        if selected.get() == language {
                            "✓ "
                        } else {
                            ""
                        },
                        title
                    )
                    .into()
                },
                false,
                move || chooser.set_language(language),
            )
            .accessibility_state(move || AccessibilityState {
                selected: selected.get() == language,
                ..Default::default()
            }),
        );
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
                .editable(false)
                .multiline(true)
                .h(move || px((window().height - 240.0).clamp(100.0, 420.0)))
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
            .text_color(move || theme().success),
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
