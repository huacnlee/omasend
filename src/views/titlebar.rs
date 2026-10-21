//! The application's title bar shares NSH's native drag and window controls.
use super::{appearance, appearance::theme, motion, session::Session, utilities};
use nsh_core::{
    desktop::TitleBarStyle,
    ui::{
        desktop::{set_title_bar, title_bar},
        prelude::*,
    },
};

pub(super) fn install() {
    if !is_desktop() {
        return;
    }
    effect(|| {
        set_title_bar(
            TitleBarStyle::Hidden,
            Some(nsh_core::Points(appearance::metrics().space(42.0))),
        );
    });
}

pub(super) fn bar(session: Session) -> impl IntoElement {
    view()
        .test_id("app-header")
        .window_drag(true)
        .flex_row()
        .items_center()
        .gap_2()
        .pl(move || px(appearance::metrics().space(16.0) + title_bar().left.0))
        .pr(move || px(appearance::metrics().space(16.0) + title_bar().right.0))
        .h(move || px(appearance::metrics().space(42.0)))
        .flex_shrink_0()
        .bg(move || {
            if appearance::is_omarchy_style() {
                theme().background()
            } else {
                theme().color("group_box.background").unwrap()
            }
        })
        .border_b_1()
        .border_color(appearance::divider)
        .child(motion::emblem(session.clone(), 24.0))
        .child(text("Omasend").test_id("app-title").flex_1())
        .child(utilities::menu_button(session))
}

/// UIKit owns header layout, safe-area changes and the native navigation layer.
pub(super) fn mobile_page(session: Session, content: impl IntoElement) -> impl IntoElement {
    nsh_base::nav_stack().flex_1().child(
        nsh_base::nav_page()
            .key("home")
            .title("Omasend")
            .header_large_title(false)
            .header_style_with(|s| {
                s.bg(move || color(theme().background()))
                    .text_color(move || color(theme().foreground()))
                    .tint_color(move || color(theme().foreground()))
            })
            .header_title(
                view()
                    .test_id("app-header")
                    .w(move || px((window().width.0 - 32.0).max(0.0)))
                    .h(px(44.0))
                    .flex_row()
                    .items_center()
                    .child(
                        view()
                            .w(px(44.0))
                            .items_center()
                            .justify_center()
                            .child(motion::emblem(session.clone(), 22.0)),
                    )
                    .child(
                        text("Omasend")
                            .test_id("app-title")
                            .flex_1()
                            .text_center()
                            .font_family("Menlo")
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || color(theme().foreground())),
                    )
                    .child(utilities::mobile_menu_trigger(session)),
            )
            .child(content),
    )
}
