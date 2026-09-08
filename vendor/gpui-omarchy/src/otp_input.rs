use crate::ActiveTheme;
use gpui_kit::base::{OtpState, StyledExt as _};
use gpui_kit::{App, Entity, Focusable, MouseButton, Window, div, prelude::*, px};

/// Numeric code entry using base state, with clipboard support and inert disabled cells.
#[derive(IntoElement)]
pub struct OtpInput {
    state: Entity<OtpState>,
    disabled: bool,
    style: gpui_kit::StyleRefinement,
    children: Vec<gpui_kit::AnyElement>,
}
impl OtpInput {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
impl Styled for OtpInput {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for OtpInput {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui_kit::AnyElement>) {
        self.children.extend(elements);
    }
}

pub fn otp_input(state: &Entity<OtpState>, _: &Window, _: &App) -> OtpInput {
    OtpInput {
        state: state.clone(),
        disabled: false,
        style: Default::default(),
        children: vec![],
    }
}

fn code_digits(text: &str, length: usize) -> String {
    text.chars()
        .filter_map(|ch| {
            if ch.is_ascii_digit() {
                Some(ch)
            } else {
                (ch as u32)
                    .checked_sub('０' as u32)
                    .and_then(|n| char::from_digit(n, 10))
            }
        })
        .take(length)
        .collect()
}

impl RenderOnce for OtpInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state;
        let t = cx.omarchy();
        let value = state.read(cx);
        let digits: Vec<char> = value.value().chars().collect();
        let focused = !self.disabled && value.focus_handle(cx).is_focused(window);
        let mut cells = Vec::new();
        for index in 0..value.len() {
            let target = state.clone();
            let current = focused && index == digits.len().min(value.len().saturating_sub(1));
            let character = digits
                .get(index)
                .map(|ch| if value.is_masked() { '•' } else { *ch });
            cells.push(
                div()
                    .w(px(32.))
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .rounded(px(0.))
                    .border_color(if current {
                        t.accent
                    } else {
                        t.control_border()
                    })
                    .bg(t.normal_fill())
                    .font_family(t.font.clone())
                    .text_size(px(16.))
                    .text_color(t.foreground)
                    .when(!self.disabled, |cell| {
                        cell.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            target.update(cx, |state, cx| state.focus(window, cx))
                        })
                    })
                    .child(character.map(|ch| ch.to_string()).unwrap_or_default())
                    .into_any_element(),
            );
        }
        cells.extend(self.children);
        if self.disabled {
            return div()
                .flex()
                .gap(px(6.))
                .children(cells)
                .refine_style(&self.style)
                .opacity(0.45)
                .into_any_element();
        }
        let paste = state.clone();
        div()
            .id(("omarchy-otp", state.entity_id()))
            .capture_key_down(move |event, window, cx| {
                let mods = event.keystroke.modifiers;
                let paste_key = event.keystroke.key == "v"
                    && !mods.alt
                    && !mods.shift
                    && if cfg!(target_os = "macos") {
                        mods.platform && !mods.control
                    } else {
                        mods.control && !mods.platform
                    };
                if paste_key {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        paste.update(cx, |state, cx| {
                            let next = code_digits(&text, state.len());
                            if !next.is_empty() && next != state.value().as_ref() {
                                let complete = next.len() == state.len();
                                state.set_value(next, window, cx);
                                cx.emit(gpui_kit::base::OtpEvent::Change);
                                if complete {
                                    cx.emit(gpui_kit::base::OtpEvent::Complete);
                                }
                            }
                        });
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                } else if (mods.control || mods.platform || mods.alt)
                    && (event.keystroke.key == "backspace"
                        || !code_digits(&event.keystroke.key, 1).is_empty()
                        || event
                            .keystroke
                            .key_char
                            .as_deref()
                            .is_some_and(|text| !code_digits(text, 1).is_empty()))
                {
                    // Modified digits are shortcuts, never code entry.
                    cx.stop_propagation();
                } else if !mods.control
                    && !mods.platform
                    && !mods.alt
                    && event.keystroke.key != "backspace"
                    && let Some(text) = event.keystroke.key_char.as_deref()
                {
                    // The produced character owns text entry. The physical key
                    // can name a different digit (or a symbol) on another layout.
                    let digit = code_digits(text, 1);
                    if !digit.is_empty() {
                        paste.update(cx, |state, cx| {
                            let mut next = state.value().to_string();
                            if next.chars().count() < state.len() {
                                next.push_str(&digit);
                                let complete = next.chars().count() == state.len();
                                state.set_value(next, window, cx);
                                cx.emit(gpui_kit::base::OtpEvent::Change);
                                if complete {
                                    cx.emit(gpui_kit::base::OtpEvent::Complete);
                                }
                            }
                        });
                        window.prevent_default();
                    }
                    cx.stop_propagation();
                }
            })
            .child(
                gpui_kit::base::OtpInput::new(&state)
                    .flex()
                    .gap(px(6.))
                    .children(cells)
                    .refine_style(&self.style),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::gpui;
    use gpui_kit::{Context, Render, TestAppContext};
    struct Harness {
        state: Entity<OtpState>,
        disabled: bool,
    }
    impl Render for Harness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            crate::focus_scope("otp-test")
                .size_full()
                .child(otp_input(&self.state, window, cx).disabled(self.disabled))
        }
    }
    #[gpui::test]
    fn otp_filters_digits_limits_length_and_backspaces(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| OtpState::new(6, window, cx)),
            disabled: false,
        });
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.focus(window, cx));
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("1 a 2 3 4 5 6 7");
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value().as_ref(), "123456"));
        cx.simulate_keystrokes("backspace 9");
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value().as_ref(), "123459"));
    }
    #[gpui::test]
    fn typed_characters_take_precedence_over_physical_digit_keys(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| OtpState::new(6, window, cx)),
            disabled: false,
        });
        cx.update(|window, cx| {
            view.read(cx)
                .state
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
            window.draw(cx).clear(cx);
        });
        for (key, character, expected) in [
            ("shift-1", "!", ""),
            ("shift-&", "1", "1"),
            ("2", "３", "13"),
        ] {
            cx.update(|window, cx| {
                let mut stroke = gpui_kit::Keystroke::parse(key).unwrap();
                stroke.key_char = Some(character.into());
                window.dispatch_keystroke(stroke, cx);
                assert_eq!(view.read(cx).state.read(cx).value().as_ref(), expected);
                window.draw(cx).clear(cx);
            });
        }
    }

    #[gpui::test]
    fn paste_filters_code_and_disabled_input_ignores_events(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| OtpState::new(6, window, cx)),
            disabled: false,
        });
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = events.clone();
        view.update(cx, |this, cx| {
            cx.subscribe(
                &this.state,
                move |_, _, event: &gpui_kit::base::OtpEvent, _| seen.borrow_mut().push(*event),
            )
            .detach();
        });
        cx.update(|window, cx| {
            view.read(cx)
                .state
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("１２3-4567".into()));
            window.draw(cx).clear(cx);
        });
        let paste = if cfg!(target_os = "macos") {
            "cmd-v"
        } else {
            "ctrl-v"
        };
        cx.simulate_keystrokes(paste);
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value().as_ref(), "123456"));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == gpui_kit::base::OtpEvent::Complete)
                .count(),
            1
        );
        cx.simulate_keystrokes(paste);
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == gpui_kit::base::OtpEvent::Complete)
                .count(),
            1
        );
        cx.simulate_keystrokes("backspace ctrl-8 alt-9");
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value().as_ref(), "12345"));
        view.update(cx, |this, cx| {
            this.disabled = true;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(gpui_kit::point(px(10.), px(10.)), Default::default());
        cx.simulate_keystrokes("backspace 9");
        cx.simulate_keystrokes(paste);
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value().as_ref(), "12345"));
        cx.update(|window, cx| {
            window.blur(cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_click(gpui_kit::point(px(10.), px(10.)), Default::default());
        cx.update(|window, cx| {
            assert!(
                !view
                    .read(cx)
                    .state
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window)
            )
        });
    }
}
