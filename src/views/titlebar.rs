//! The application's title bar shares NSH's native drag and window controls.
use super::{appearance, appearance::theme, motion, plain_control, session::Session, utilities};
use nsh_core::{
    desktop::TitleBarStyle,
    ui::{
        desktop::{set_title_bar, title_bar},
        prelude::*,
    },
};

pub(super) fn install() {
    effect(|| {
        set_title_bar(
            TitleBarStyle::Hidden,
            Some(appearance::metrics().space(42.0)),
        );
    });
}

pub(super) fn bar(session: Session) -> impl IntoElement {
    let menu = session.clone();
    view()
        .test_id("app-header")
        .window_drag(true)
        .flex_row()
        .items_center()
        .gap_2()
        .pl(move || px(appearance::metrics().space(16.0) + title_bar().left))
        .pr(move || px(appearance::metrics().space(16.0) + title_bar().right))
        .h(move || px(appearance::metrics().space(42.0)))
        .flex_shrink_0()
        .bg(move || {
            if appearance::is_omarchy() {
                theme().background
            } else {
                theme().group_box
            }
        })
        .border_b_1()
        .border_color(appearance::divider)
        .child(motion::emblem(session.clone(), 24.0))
        .child(text("Omasend").test_id("app-title").flex_1())
        .child(
            plain_control("☰", false, move || {
                menu.open_surface(utilities::Surface::Menu)
            })
            .window_drag(false)
            .test_id("app-menu")
            .label(session.translated("action.menu")),
        )
}
