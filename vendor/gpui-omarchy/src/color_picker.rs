//! Color editing backed by base's hex validation and synchronized HSLA state.
use crate::{ActiveTheme, ButtonVariant, button, input, popover_surface, slider};
use gpui_kit::base::{ColorPicker, ColorPickerState, Popup};
use gpui_kit::{App, ElementId, Entity, Focusable, Window, div, prelude::*, px};

pub fn color_picker(
    id: impl Into<ElementId>,
    state: &Entity<ColorPickerState>,
    window: &mut Window,
    cx: &mut App,
) -> ColorPicker {
    let id = id.into();
    state.update(cx, |state, cx| state.sync_pending_value(window, cx));
    let current = state.read(cx);
    let open = current.is_open();
    let color = current.displayed_color();
    let focus = current.focus_handle(cx);
    let hex = current.hex_input().clone();
    let channels = current.sliders().clone();
    let hex_text = hex.read(cx).value().to_string();
    let digits = hex_text.strip_prefix('#').unwrap_or(&hex_text);
    let valid_hex = matches!(digits.len(), 3 | 4 | 6 | 8)
        && digits.bytes().all(|byte| byte.is_ascii_hexdigit());
    // A successful Hex commit is owned by base and closes its state directly.
    // Return focus before removing the text field from the render tree.
    if !open && hex.read(cx).focus_handle(cx).is_focused(window) {
        focus.focus(window, cx);
    }
    let t = cx.omarchy().clone();
    let trigger = button("color-trigger", "Choose color", ButtonVariant::Outline, cx)
        .debug_selector(|| "color-picker-trigger".into())
        // The outer picker owns focus; registering this handle on the trigger
        // as well reports two focused accessibility nodes in the same frame.
        .focusable(false)
        .child(
            div()
                .size(px(16.))
                .border_1()
                .border_color(t.border)
                .bg(color.unwrap_or(t.background)),
        )
        .on_click(|_, window, cx| {
            window.dispatch_action(
                Box::new(gpui_kit::base::actions::Confirm { secondary: false }),
                cx,
            );
        });
    let mut popup = Popup::new((id.clone(), "popup"), trigger);
    if open {
        let target = state.clone();
        let return_focus = focus.clone();
        let mut body = popover_surface(cx)
            .id("color-editor")
            .debug_selector(|| "color-picker-popup".into())
            .key_context("OmarchyPopoverContent")
            .w(px(280.))
            .mt(px(4.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .on_mouse_down_out(move |_, window, cx| {
                target.update(cx, |state, cx| {
                    restore_committed_color(state, window, cx);
                    state.set_open(false, cx);
                });
                return_focus.focus(window, cx);
            })
            .child("Hex color")
            .child(input("color-hex", &hex, window, cx))
            .when(!hex_text.is_empty() && !valid_hex, |body| {
                body.child(
                    div()
                        .debug_selector(|| "color-hex-error".into())
                        .text_color(t.danger)
                        .child("Use 3, 4, 6 or 8 hexadecimal digits."),
                )
            });
        for (name, channel) in [
            ("Hue", channels.hue()),
            ("Saturation", channels.saturation()),
            ("Lightness", channels.lightness()),
            ("Opacity", channels.alpha()),
        ] {
            body = body.child(
                div().flex().flex_col().gap(px(4.)).child(name).child(
                    div()
                        .debug_selector(move || format!("color-channel-{name}"))
                        .child(slider(channel, false, window, cx).px(px(0.))),
                ),
            );
        }
        let target = state.clone();
        let return_focus = focus.clone();
        body = body.child(
            button("color-apply", "Apply color", ButtonVariant::Outline, cx)
                .debug_selector(|| "color-apply".into())
                .disabled(!valid_hex)
                .on_click(move |_, window, cx| {
                    let committed = target.update(cx, |state, cx| {
                        let text = state.hex_input().read(cx).value().to_string();
                        state.commit_hex(&text, window, cx).is_some()
                    });
                    if committed {
                        return_focus.focus(window, cx);
                    }
                }),
        );
        popup = popup.content(body);
    }
    let target = state.clone();
    let return_focus = focus.clone();
    ColorPicker::new(id)
        .open(open)
        .track_focus(&focus)
        .accessibility_label("Choose color")
        .on_open_change(move |open, window, cx| {
            target.update(cx, |state, cx| {
                if !open {
                    restore_committed_color(state, window, cx);
                }
                state.set_open(open, cx);
            });
            return_focus.focus(window, cx);
        })
        .child(popup)
}

// Invalid Hex input leaves base's preview equal to its committed value, so
// clear_preview alone can retain an invalid draft. Re-sync all editing fields.
fn restore_committed_color(
    state: &mut ColorPickerState,
    window: &mut Window,
    cx: &mut gpui_kit::Context<ColorPickerState>,
) {
    if let Some(value) = state.value() {
        state.set_value(value, window, cx);
    } else {
        state.clear_value(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::gpui;
    use gpui_kit::{Context, Render, TestAppContext};
    struct Harness {
        state: Entity<ColorPickerState>,
        disabled: bool,
    }
    impl Render for Harness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            crate::focus_scope("color-test")
                .size_full()
                .child(color_picker("test", &self.state, window, cx).disabled(self.disabled))
        }
    }
    #[gpui::test]
    fn disabled_and_invalid_drafts_do_not_commit(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| {
                ColorPickerState::new(window, cx).default_value(gpui_kit::hsla(0., 1., 0.5, 1.))
            });
            cx.observe(&state, |_, _, cx| cx.notify()).detach();
            Harness {
                state,
                disabled: true,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let trigger = cx.debug_bounds("color-picker-trigger").unwrap().center();
        cx.simulate_click(trigger, Default::default());
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| assert!(!view.read(cx).state.read(cx).is_open()));
        view.update(cx, |this, cx| {
            this.disabled = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let input = view.read(cx).state.read(cx).hex_input().clone();
            input.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.focus(window, cx);
            });
        });
        cx.simulate_input("#12");
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(view.read(cx).state.read(cx).is_open());
        });
        assert!(cx.debug_bounds("color-hex-error").is_some());
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let state = view.read(cx).state.read(cx);
            assert!(!state.is_open());
            assert_eq!(state.hex_input().read(cx).value().as_ref(), "#FF0000");
        });
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let opacity = cx.debug_bounds("color-channel-Opacity").unwrap();
        cx.simulate_click(
            gpui_kit::point(opacity.left() + opacity.size.width / 2., opacity.center().y),
            Default::default(),
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let state = view.read(cx).state.read(cx);
            assert!(state.is_open());
            assert!((state.value().unwrap().a - 0.5).abs() < 0.1);
        });
        cx.update(|window, cx| {
            let hex = view.read(cx).state.read(cx).hex_input().clone();
            hex.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.focus(window, cx);
            });
        });
        cx.simulate_input("#00FF00");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let apply = cx.debug_bounds("color-apply").unwrap().center();
        cx.simulate_click(apply, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let state = view.read(cx).state.read(cx);
            assert!(!state.is_open());
            assert_eq!(state.value(), Some(gpui_kit::hsla(1. / 3., 1., 0.5, 1.)));
            assert!(state.focus_handle(cx).is_focused(window));
        });
    }
}
