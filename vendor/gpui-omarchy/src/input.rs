//! Native text editing, selection, clipboard and IME supplied by gpui-base.
use crate::ActiveTheme;
use gpui::{
    App, ElementId, Entity, Focusable, InteractiveElement, MouseButton, ParentElement, Styled,
    Window, px,
};
use gpui_base::{
    Input, InputBase, Textarea,
    input::{InputState, TextareaState},
};

fn frame(id: impl Into<ElementId>, focused: bool, cx: &App) -> InputBase {
    let t = cx.omarchy();
    InputBase::new(id)
        .focused(focused)
        .w_full()
        .min_w_0()
        .px(px(10.))
        .border_1()
        .rounded(px(0.))
        .border_color(t.control_border())
        .bg(t.normal_fill())
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .styles(|s| s.focused(|s| s.bg(t.hover_fill()).border_color(t.focus_border())))
}

pub fn input(
    id: impl Into<ElementId>,
    state: &Entity<InputState>,
    window: &Window,
    cx: &mut App,
) -> InputBase {
    let style = cx.omarchy().input_style();
    state.update(cx, |state, _| state.set_editor_style(style));
    let focused = state.read(cx).focus_handle(cx).is_focused(window);
    let target = state.clone();
    frame(id, focused, cx)
        .py(px(7.))
        .flex()
        .items_center()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            target.update(cx, |state, cx| state.focus(window, cx));
        })
        .child(Input::new(state))
}

pub fn textarea(
    id: impl Into<ElementId>,
    state: &Entity<TextareaState>,
    window: &Window,
    cx: &mut App,
) -> InputBase {
    let style = cx.omarchy().input_style();
    state.update(cx, |state, _| state.set_editor_style(style));
    let focused = state.read(cx).focus_handle(cx).is_focused(window);
    let target = state.clone();
    frame(id, focused, cx)
        .min_h(px(96.))
        .py(px(7.))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            target.update(cx, |state, cx| state.focus(window, cx));
        })
        .child(Textarea::new(state))
}

/// Numeric editing with native arrow-key stepping and paired step buttons.
pub fn number_input(state: &Entity<InputState>, cx: &mut App) -> gpui_base::NumberInput {
    use gpui::ParentElement as _;
    let t = cx.omarchy().clone();
    state.update(cx, |state, cx| {
        state.set_editor_style(t.input_style());
        state.set_text_align(gpui::TextAlign::Center, cx);
    });
    let minus = t.clone();
    gpui_base::NumberInput::new(state)
        .w(px(120.))
        .h(px(28.))
        .flex()
        .items_center()
        .border_1()
        .rounded(px(0.))
        .border_color(t.control_border())
        .bg(t.normal_fill())
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .input(
            gpui::div()
                .flex_1()
                .min_w_0()
                .px(px(10.))
                .child(Input::new(state)),
        )
        .decrement_button(move |button| {
            button
                .w(px(28.))
                .h_full()
                .text_color(minus.foreground)
                .bg(minus.surface)
                .hover(|s| s.bg(minus.selection))
                .child(crate::icon(crate::IconName::Minus).size(px(12.)))
        })
        .increment_button(move |button| {
            button
                .w(px(28.))
                .h_full()
                .text_color(t.foreground)
                .bg(t.surface)
                .hover(|s| s.bg(t.selection))
                .child(crate::icon(crate::IconName::Plus).size(px(12.)))
        })
}
