//! Contextual controls in an anchored, non-modal surface.
use crate::ActiveTheme;
use gpui::{App, Context, Div, ElementId, Window, div, prelude::*, px};
use gpui_base::{Popover, PopoverState};

pub(crate) fn init(cx: &mut App) {
    // Suppress the outer Popover toggle bindings inside its content. Deeper
    // control contexts (Button, Input, Select) still retain their own bindings.
    cx.bind_keys([
        gpui::KeyBinding::new("enter", gpui::NoAction, Some("OmarchyPopoverContent")),
        gpui::KeyBinding::new("space", gpui::NoAction, Some("OmarchyPopoverContent")),
    ]);
}

/// An anchored control surface with base-owned open state, outside dismissal,
/// and focus return. The content builder runs with the current theme.
///
/// To customize the surface itself, use the returned Popover's `content` builder
/// and compose `popover_surface(cx)` with your own dimensions and children.
pub fn popover<E: IntoElement>(
    id: impl Into<ElementId>,
    trigger: impl gpui_base::Selectable + IntoElement + 'static,
    content: impl FnOnce(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
) -> Popover {
    Popover::new(id)
        .trigger(trigger)
        .content(move |state, window, cx| {
            let body = content(state, window, cx);
            div()
                .key_context("OmarchyPopoverContent")
                .child(popover_surface(cx).child(body))
        })
}

/// Omarchy popup chrome. Content remains free to use forms, rows, and actions.
pub fn popover_surface(cx: &App) -> Div {
    let t = cx.omarchy();
    div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .w(px(280.))
        .max_w(gpui::relative(1.))
        .p(px(14.))
        .border_1()
        .rounded(px(0.))
        .border_color(t.border)
        .bg(t.background)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{FocusHandle, Render, TestAppContext};
    struct Harness {
        trigger: FocusHandle,
        checked: bool,
        checkbox_focus: FocusHandle,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let target = cx.entity();
            crate::focus_scope("root").size_full().child(popover(
                "options",
                crate::button(
                    "trigger",
                    "Display options",
                    crate::ButtonVariant::Secondary,
                    cx,
                )
                .track_focus(&self.trigger),
                move |_, _, cx| {
                    let checked = target.read(cx).checked;
                    let checkbox_focus = target.read(cx).checkbox_focus.clone();
                    div()
                        .debug_selector(|| "test-popover-content".into())
                        .child(
                            crate::checkbox(
                                "hidden",
                                "Show hidden files",
                                if checked {
                                    gpui_base::CheckboxState::Checked
                                } else {
                                    gpui_base::CheckboxState::Unchecked
                                },
                                cx,
                            )
                            .track_focus(&checkbox_focus)
                            .on_change(move |value, _, _, cx| {
                                target.update(cx, |state, cx| {
                                    state.checked = value == gpui_base::CheckboxState::Checked;
                                    cx.notify();
                                })
                            }),
                        )
                },
            ))
        }
    }
    #[gpui::test]
    fn keyboard_edits_popup_without_closing_then_escape_restores_focus(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|_, cx| Harness {
            trigger: cx.focus_handle(),
            checked: false,
            checkbox_focus: cx.focus_handle(),
        });
        cx.update(|window, cx| {
            view.read(cx).trigger.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("test-popover-content").is_some());
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.update(|window, cx| {
            assert!(
                view.read(cx).checkbox_focus.is_focused(window),
                "Tab should focus checkbox"
            )
        });
        let keystroke = gpui::Keystroke::parse("space").unwrap();
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(view.read(cx).checked);
        });
        assert!(
            cx.debug_bounds("test-popover-content").is_some(),
            "editing does not dismiss the popup"
        );
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(view.read(cx).trigger.is_focused(window));
        });
        assert!(cx.debug_bounds("test-popover-content").is_none());
    }
}
