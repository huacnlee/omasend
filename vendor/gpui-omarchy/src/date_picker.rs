use crate::{ButtonVariant, IconName, button, calendar, icon};
use gpui_kit::base::{CalendarEvent, CalendarState, DatePicker, Popup};
use gpui_kit::{
    App, Context, ElementId, Entity, FocusHandle, MouseButton, Window, div, prelude::*, px,
};

pub struct DatePickerState {
    pub calendar: Entity<CalendarState>,
    focus: FocusHandle,
    open: bool,
}

impl DatePickerState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let calendar = cx.new(|cx| CalendarState::new(window, cx));
        cx.subscribe_in(
            &calendar,
            window,
            |this, _, event: &CalendarEvent, window, cx| {
                let CalendarEvent::Selected(date) = event;
                if date.is_complete() {
                    this.set_open(false, window, cx);
                }
            },
        )
        .detach();
        cx.observe(&calendar, |_, _, cx| cx.notify()).detach();
        Self {
            calendar,
            focus: cx.focus_handle(),
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open = open;
        if open {
            self.calendar
                .read(cx)
                .focus_handle
                .clone()
                .focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
        cx.notify();
    }
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        gpui_kit::KeyBinding::new(
            "enter",
            gpui_kit::base::actions::Confirm { secondary: false },
            Some("OmarchyDatePicker"),
        ),
        gpui_kit::KeyBinding::new(
            "space",
            gpui_kit::base::actions::Confirm { secondary: false },
            Some("OmarchyDatePicker"),
        ),
        gpui_kit::KeyBinding::new(
            "escape",
            gpui_kit::base::actions::Cancel,
            Some("OmarchyDatePicker"),
        ),
    ]);
}

/// Controlled base date picker with a styled calendar popup and focus return.
pub fn date_picker(
    id: impl Into<ElementId>,
    state: &Entity<DatePickerState>,
    cx: &App,
) -> DatePicker {
    let id = id.into();
    let current = state.read(cx);
    let open = current.open;
    let focus = current.focus.clone();
    let label = current
        .calendar
        .read(cx)
        .date()
        .format("%b %e, %Y")
        .unwrap_or_else(|| "Choose a date".into());
    let trigger = button("date-trigger", "", ButtonVariant::Outline, cx)
        // The outer picker owns focus; registering this handle on the trigger
        // as well reports two focused accessibility nodes in the same frame.
        .focusable(false)
        .accessibility_label("Choose a date")
        .debug_selector(|| "date-picker-trigger".into())
        .w_full()
        .justify_start()
        .child(div().flex_1().child(label))
        .child(icon(IconName::Calendar).size(px(14.)))
        .on_click(move |_, window, cx| {
            // Route pointer activation through the base root so builder refinements
            // such as `.disabled(true)` govern both keyboard and mouse behavior.
            if open {
                window.dispatch_action(Box::new(gpui_kit::base::actions::Cancel), cx);
            } else {
                window.dispatch_action(
                    Box::new(gpui_kit::base::actions::Confirm { secondary: false }),
                    cx,
                );
            }
        });
    let mut popup = Popup::new((id.clone(), "popup"), trigger);
    if open {
        let target = state.clone();
        popup = popup.content(
            div()
                .id("date-calendar-popup")
                .key_context("OmarchyPopoverContent")
                .occlude()
                .mt(px(4.))
                .on_mouse_down_out(move |_, window, cx| {
                    target.update(cx, |state, cx| state.set_open(false, window, cx))
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(calendar("date-calendar", &current.calendar, cx)),
        );
    }
    let target = state.clone();
    DatePicker::new(id, &focus)
        .open(open)
        .w(px(248.))
        .key_context("OmarchyDatePicker")
        .on_open_change(move |open, window, cx| {
            target.update(cx, |state, cx| state.set_open(open, window, cx))
        })
        .child(popup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::gpui;
    use gpui_kit::{Render, TestAppContext};

    struct Harness {
        state: Entity<DatePickerState>,
        disabled: bool,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            crate::focus_scope("date-picker-test")
                .size_full()
                .child(date_picker("date", &self.state, cx).disabled(self.disabled))
        }
    }

    #[gpui::test]
    fn disabled_picker_rejects_pointer_and_keyboard(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| Harness {
            state: cx.new(|cx| DatePickerState::new(window, cx)),
            disabled: true,
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let trigger = cx.debug_bounds("date-picker-trigger").unwrap().center();
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(!view.read(cx).state.read(cx).is_open());
        });
        cx.simulate_keystrokes("enter space");
        cx.update(|_, cx| assert!(!view.read(cx).state.read(cx).is_open()));
        view.update(cx, |this, cx| {
            this.disabled = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(view.read(cx).state.read(cx).is_open());
        });
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| assert!(!view.read(cx).state.read(cx).is_open()));
    }
    #[gpui::test]
    fn pointer_selection_reopen_and_outside_dismissal(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| DatePickerState::new(window, cx));
            cx.observe(&state, |_, _, cx| cx.notify()).detach();
            Harness {
                state,
                disabled: false,
            }
        });
        for _ in 0..3 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let trigger = cx.debug_bounds("date-picker-trigger").unwrap().center();
            cx.simulate_click(trigger, Default::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let next = cx.debug_bounds("calendar-next").unwrap().center();
            cx.simulate_click(next, Default::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            // Header center + header half-height + section gap + weekday row
            // + one complete week + day half-height: select the second Saturday.
            let day = next + gpui_kit::point(px(0.), px(14. + 8. + 28. + 28. + 14.));
            cx.simulate_click(day, Default::default());
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let state = view.read(cx).state.read(cx);
                assert!(!state.is_open(), "selecting a day closes the popup");
                assert!(state.calendar.read(cx).date().is_some());
                assert!(state.focus.is_focused(window));
            });
            cx.simulate_click(trigger, Default::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.simulate_click(gpui_kit::point(px(500.), px(400.)), Default::default());
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(!view.read(cx).state.read(cx).is_open());
            });
        }
    }
}
