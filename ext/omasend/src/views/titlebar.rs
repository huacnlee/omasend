//! The application's title bar shares NSH's native drag and window controls.
use super::{motion, session::Session, utilities};
use nsh_core::prelude::*;

/// The desktop title bar is nsh-omarchy's: it hides the system bar, drags
/// the window and keeps clear of the window controls. The emblem stays.
pub(super) fn bar(session: Session) -> impl IntoElement {
    nsh_omarchy::title_bar()
        .test_id("app-header")
        .child(motion::static_emblem(20.0))
        .child(nsh_omarchy::title("Omasend").test_id("app-title").flex_1())
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
                s.bg(move || color(nsh_omarchy::tokens::background()))
                    .text_color(move || color(nsh_omarchy::tokens::foreground()))
                    .tint_color(move || color(nsh_omarchy::tokens::foreground()))
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
                            .child(motion::static_emblem(22.0)),
                    )
                    .child(
                        text("Omasend")
                            .test_id("app-title")
                            .flex_1()
                            .text_center()
                            .font_family("Menlo")
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || color(nsh_omarchy::tokens::foreground())),
                    )
                    .child(utilities::mobile_menu_trigger(session)),
            )
            .child(content),
    )
}
