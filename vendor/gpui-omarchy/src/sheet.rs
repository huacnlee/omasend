//! Edge-attached modal details, with dismissal and focus trapping owned by base.
use crate::{ActiveTheme, dialog_backdrop};
use gpui::{App, Div, FocusHandle, div, prelude::*, px, relative};

/// Focus `focus` when opening; restore the trigger in `request_close`.
pub fn sheet(focus: &FocusHandle, cx: &mut App) -> gpui_base::Sheet {
    gpui_base::Sheet::new(cx)
        .focus_handle(focus.clone())
        .overlay(dialog_backdrop())
}

/// Right-edge surface. Override width through the normal base styling API.
pub fn sheet_surface(cx: &App) -> Div {
    let t = cx.omarchy();
    div()
        .absolute()
        .right_0()
        .top_0()
        .h_full()
        .w(px(360.))
        .max_w(relative(1.))
        .flex()
        .flex_col()
        .gap(px(14.))
        .p(px(18.))
        .border_l_1()
        .border_color(t.border)
        .bg(t.background)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
        .occlude()
}
