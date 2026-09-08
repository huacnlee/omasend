//! Exclusive settings and page navigation with one keyboard stop per group.
use crate::{ActiveTheme, ChoiceItem};
use gpui::{App, Context, ElementId, Entity, FocusHandle, KeyBinding, Window, prelude::*, px};
use gpui_base::{
    Radio, RadioGroup, Tabs,
    actions::{Confirm, SelectLeft, SelectRight},
};
use std::rc::Rc;

type Change = Rc<dyn Fn(usize, &mut Window, &mut App)>;
struct Cursor {
    focus: FocusHandle,
    index: Option<usize>,
    was_focused: bool,
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some("OmarchyOptionGroup")),
        KeyBinding::new("h", SelectLeft, Some("OmarchyOptionGroup")),
        KeyBinding::new("right", SelectRight, Some("OmarchyOptionGroup")),
        KeyBinding::new("l", SelectRight, Some("OmarchyOptionGroup")),
        KeyBinding::new(
            "enter",
            Confirm { secondary: false },
            Some("OmarchyOptionGroup"),
        ),
        KeyBinding::new(
            "space",
            Confirm { secondary: false },
            Some("OmarchyOptionGroup"),
        ),
    ]);
}

/// A single-choice setting rendered as Omarchy's bordered ButtonGroup.
/// The application owns `selected` and updates it in `on_change`.
/// Left/Right (or h/l) move the cursor; Return/Space commit the choice.
pub fn button_group(
    id: impl Into<ElementId>,
    items: Vec<ChoiceItem>,
    selected: Option<usize>,
    on_change: impl Fn(usize, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) -> RadioGroup {
    let id = id.into();
    let cursor = cursor(&id, &items, selected, window, cx);
    let on_change: Change = Rc::new(move |index, window, cx| {
        if selected != Some(index) {
            on_change(index, window, cx);
        }
    });
    let t = cx.omarchy().clone();
    let count = items.len();
    let rows = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let change = on_change.clone();
            let focus = cursor.read(cx).focus.clone();
            Radio::new(index)
                .checked(selected == Some(index))
                .disabled(item.disabled)
                .accessibility_label(item.label.clone())
                .set_position(index + 1, count)
                .tab_stop(false)
                .flex()
                .items_center()
                .px(px(10.))
                .py(px(6.))
                .border_1()
                .rounded(px(0.))
                .border_color(t.control_border())
                .font_family(t.font.clone())
                .text_size(px(12.))
                .text_color(t.foreground)
                .bg(if selected == Some(index) {
                    t.selected_fill()
                } else {
                    t.foreground.opacity(0.)
                })
                .when(
                    focus.is_focused(window) && cursor.read(cx).index == Some(index),
                    |row| row.bg(t.hover_fill()).border_color(t.focus_border()),
                )
                .hover(|row| row.bg(t.hover_fill()).border_color(t.focus_border()))
                .active(|row| row.bg(t.pressed_fill()))
                .styles(|styles| styles.disabled(|row| row.opacity(0.45)))
                .child(item.label.clone())
                .on_change(move |_, _, window, cx| {
                    focus.focus(window, cx);
                    change(index, window, cx);
                })
                .into_any_element()
        })
        .collect::<Vec<_>>();
    navigate(
        RadioGroup::new(id)
            .axis(gpui::Axis::Horizontal)
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .children(rows),
        cursor,
        items,
        on_change,
        cx,
    )
}

/// A page tab list with the same restrained visual language and independent
/// tab-list semantics. Use the selected value to render the associated panel.
pub fn tab_list(
    id: impl Into<ElementId>,
    items: Vec<ChoiceItem>,
    selected: Option<usize>,
    on_change: impl Fn(usize, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) -> Tabs {
    let id = id.into();
    let cursor = cursor(&id, &items, selected, window, cx);
    let on_change: Change = Rc::new(move |index, window, cx| {
        if selected != Some(index) {
            on_change(index, window, cx);
        }
    });
    let t = cx.omarchy().clone();
    let count = items.len();
    let rows = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let change = on_change.clone();
            let focus = cursor.read(cx).focus.clone();
            crate::tab(index, item.label.clone(), selected == Some(index), cx)
                .debug_selector(move || format!("omarchy-tab-{index}"))
                .disabled(item.disabled)
                .set_position(index + 1, count)
                .when(
                    focus.is_focused(window) && cursor.read(cx).index == Some(index),
                    |tab| tab.bg(t.hover_fill()).border_color(t.focus_border()),
                )
                .on_click(move |_, window, cx| {
                    focus.focus(window, cx);
                    change(index, window, cx);
                })
                .into_any_element()
        })
        .collect::<Vec<_>>();
    navigate(
        crate::tabs(id, cx).children(rows),
        cursor,
        items,
        on_change,
        cx,
    )
}

fn cursor(
    id: &ElementId,
    items: &[ChoiceItem],
    selected: Option<usize>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Cursor> {
    let initial = selected
        .filter(|&i| items.get(i).is_some_and(|item| !item.disabled))
        .or_else(|| items.iter().position(|item| !item.disabled));
    let cursor = window.use_keyed_state((id.clone(), "cursor"), cx, |_, cx| Cursor {
        focus: cx.focus_handle(),
        index: initial,
        was_focused: false,
    });
    cursor.update(cx, |state, _| {
        let focused = state.focus.is_focused(window);
        if (focused && !state.was_focused)
            || !state
                .index
                .is_some_and(|i| items.get(i).is_some_and(|item| !item.disabled))
        {
            state.index = initial;
        }
        state.was_focused = focused;
    });
    cursor
}

fn navigate<T: StatefulInteractiveElement + FluentBuilder>(
    root: T,
    cursor: Entity<Cursor>,
    items: Vec<ChoiceItem>,
    change: Change,
    cx: &App,
) -> T {
    let focus = cursor.read(cx).focus.clone();
    let enabled: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| (!item.disabled).then_some(i))
        .collect();
    let mut root = root
        .key_context("OmarchyOptionGroup")
        .when(!enabled.is_empty(), |root| {
            root.track_focus(&focus.clone().tab_stop(true))
        });
    if !enabled.is_empty() {
        let pointer_focus = focus.clone();
        root = root.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
            pointer_focus.focus(window, cx)
        });
    }
    let left = cursor.clone();
    let left_items = enabled.clone();
    root = root.on_action(move |_: &SelectLeft, window, cx| {
        left.update(cx, |state, cx| step(state, &left_items, false, cx));
        window.refresh();
    });
    let right = cursor.clone();
    let right_items = enabled;
    root = root.on_action(move |_: &SelectRight, window, cx| {
        right.update(cx, |state, cx| step(state, &right_items, true, cx));
        window.refresh();
    });
    root.on_action(move |_: &Confirm, window, cx| {
        if let Some(index) = cursor
            .read(cx)
            .index
            .filter(|&i| items.get(i).is_some_and(|item| !item.disabled))
        {
            change(index, window, cx);
        }
    })
}
fn step(cursor: &mut Cursor, enabled: &[usize], forward: bool, cx: &mut Context<Cursor>) {
    if enabled.is_empty() {
        return;
    }
    let current = cursor
        .index
        .and_then(|i| enabled.iter().position(|&v| v == i))
        .unwrap_or(0);
    cursor.index = Some(
        enabled[if forward {
            (current + 1).min(enabled.len() - 1)
        } else {
            current.saturating_sub(1)
        }],
    );
    cx.notify();
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Render, TestAppContext};
    struct Harness {
        tab_list: bool,
        selected: usize,
        before: FocusHandle,
        after: FocusHandle,
    }
    impl Render for Harness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec![
                ChoiceItem::new("first", "First"),
                ChoiceItem::new("disabled", "Disabled").disabled(true),
                ChoiceItem::new("last", "Last"),
            ];
            let target = cx.entity();
            let change = move |index, _: &mut Window, cx: &mut App| {
                target.update(cx, |this, cx| {
                    this.selected = index;
                    cx.notify();
                })
            };
            let group = if self.tab_list {
                tab_list("options", items, Some(self.selected), change, window, cx)
                    .into_any_element()
            } else {
                button_group("options", items, Some(self.selected), change, window, cx)
                    .into_any_element()
            };
            crate::focus_scope("root")
                .size_full()
                .flex()
                .flex_col()
                .items_start()
                .child(
                    crate::button("before", "Before", crate::ButtonVariant::Secondary, cx)
                        .track_focus(&self.before),
                )
                .child(group)
                .child(
                    crate::button("after", "After", crate::ButtonVariant::Secondary, cx)
                        .track_focus(&self.after),
                )
        }
    }
    #[gpui::test]
    fn groups_have_one_tab_stop_and_skip_disabled_choices(cx: &mut TestAppContext) {
        cx.update(crate::init);
        for tab_list in [false, true] {
            let (view, cx) = cx.add_window_view(move |_, cx| Harness {
                tab_list,
                selected: 0,
                before: cx.focus_handle(),
                after: cx.focus_handle(),
            });
            cx.update(|window, cx| {
                view.read(cx).before.clone().focus(window, cx);
                window.draw(cx).clear(cx);
            });
            cx.simulate_keystrokes("tab");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
            });
            cx.simulate_keystrokes("right");
            cx.update(|_, cx| {
                assert_eq!(
                    view.read(cx).selected,
                    0,
                    "arrows move the cursor without committing"
                )
            });
            cx.simulate_keystrokes("enter");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(view.read(cx).selected, 2, "disabled option is skipped");
            });
            cx.simulate_keystrokes("tab");
            cx.update(|window, cx| {
                assert!(
                    view.read(cx).after.is_focused(window),
                    "one tab stop per group"
                )
            });
            cx.simulate_keystrokes("shift-tab");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
            });
            cx.simulate_keystrokes("left space");
            cx.update(|_, cx| assert_eq!(view.read(cx).selected, 0));
        }
    }
}
