//! Dense, square surfaces and non-interactive information components.
use crate::ActiveTheme;
use gpui::{
    App, Div, ElementId, FontWeight, ParentElement, SharedString, Styled, div, px, relative,
};
use gpui_base::{Progress, ProgressIndicator, ProgressTrack};

pub fn panel(title: impl Into<SharedString>, cx: &App) -> Div {
    let t = cx.omarchy();
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(14.))
        .p(px(18.))
        .border_1()
        .border_color(t.border)
        .bg(t.background)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::BOLD)
                .child(title.into()),
        )
}

pub fn separator(cx: &App) -> Div {
    div()
        .w_full()
        .h(px(1.))
        .flex_shrink_0()
        .bg(cx.omarchy().divider())
}

/// A vertical rule for toolbars. Override height to fit a different row size.
pub fn vertical_separator(cx: &App) -> Div {
    div()
        .w(px(1.))
        .h(px(20.))
        .flex_shrink_0()
        .bg(cx.omarchy().divider())
}

pub fn keycap(key: impl Into<SharedString>, cx: &App) -> Div {
    let t = cx.omarchy();
    div()
        .px(px(4.))
        .py(px(2.))
        .border_1()
        .border_color(t.border)
        .bg(t.inset)
        .font_family(t.font.clone())
        .text_size(px(11.))
        .font_weight(FontWeight::BOLD)
        .text_color(t.foreground)
        .child(key.into())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Neutral,
    Success,
    Warning,
    Error,
}

/// Inline feedback with square borders and semantic theme colors.
pub fn alert(message: impl Into<SharedString>, status: Status, cx: &App) -> Div {
    let theme = cx.omarchy();
    let color = match status {
        Status::Neutral => theme.secondary,
        Status::Success => theme.success,
        Status::Warning => theme.warning,
        Status::Error => theme.danger,
    };
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .p(px(10.))
        .border_1()
        .border_color(color.opacity(0.35))
        .bg(color.opacity(0.06))
        .text_color(color)
        .child(
            crate::icon(match status {
                Status::Success => crate::IconName::Check,
                Status::Neutral => crate::IconName::Minus,
                Status::Warning | Status::Error => crate::IconName::TriangleAlert,
            })
            .size(px(14.))
            .flex_shrink_0()
            .text_color(color),
        )
        .child(div().flex_1().min_w_0().child(message.into()))
}

pub fn badge(label: impl Into<SharedString>, status: Status, cx: &App) -> Div {
    let t = cx.omarchy();
    let color = match status {
        Status::Neutral => t.secondary,
        Status::Success => t.success,
        Status::Warning => t.warning,
        Status::Error => t.danger,
    };
    div()
        .px(px(6.))
        .py(px(2.))
        .border_1()
        .border_color(color)
        .text_color(color)
        .font_family(t.font.clone())
        .text_size(px(11.))
        .child(label.into())
}

pub fn empty_state(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    cx: &App,
) -> Div {
    let t = cx.omarchy();
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(18.))
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
        .child(div().font_weight(FontWeight::BOLD).child(title.into()))
        .child(div().text_color(t.secondary).child(description.into()))
}

/// Determinate progress, clamped to 0..=100; non-finite values become zero.
pub fn progress(id: impl Into<ElementId>, value: f32, cx: &App) -> Progress {
    let t = cx.omarchy();
    let value = normalized_progress(value);
    Progress::new(id).value(value).w_full().child(
        ProgressTrack::new().w_full().h(px(6.)).bg(t.border).child(
            ProgressIndicator::new()
                .h_full()
                .w(relative(value / 100.))
                .bg(t.accent),
        ),
    )
}

fn normalized_progress(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0., 100.)
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_handles_invalid_and_out_of_range_values() {
        for (input, expected) in [
            (-1., 0.),
            (42., 42.),
            (150., 100.),
            (f32::NAN, 0.),
            (f32::INFINITY, 0.),
        ] {
            assert_eq!(normalized_progress(input), expected);
        }
    }
}

/// Notification surface; compose actions and use gpui-base's ToastManager for
/// application-owned stacking and timeout policy.
pub fn toast(id: impl Into<gpui::ElementId>, cx: &App) -> gpui_base::Toast {
    let t = cx.omarchy();
    gpui_base::Toast::new(id)
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(14.))
        .w_full()
        .border_1()
        .rounded(px(0.))
        .border_color(t.border)
        .bg(t.background)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
}

/// A square identity marker. Supply initials or an icon in the fallback slot;
/// callers can replace it with `avatar_image` through the base image builder.
pub fn avatar(initials: impl Into<SharedString>, cx: &App) -> gpui_base::Avatar {
    let t = cx.omarchy();
    gpui_base::Avatar::new()
        .size(px(32.))
        .flex_shrink_0()
        .overflow_hidden()
        .rounded(px(0.))
        .border_1()
        .border_color(t.control_border())
        .bg(t.hover_fill())
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
        .fallback(
            gpui_base::AvatarFallback::new()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(initials.into()),
        )
}

/// An image slot sized to its avatar. Image loading and failure policy remain
/// with the application, matching gpui-base's explicit slot API.
pub fn avatar_image(source: impl Into<gpui::ImageSource>) -> gpui_base::AvatarImage {
    gpui_base::AvatarImage::new(source).size_full()
}
