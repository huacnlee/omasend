//! Select and searchable Combobox presentations over the controlled base roots.
use crate::{ActiveTheme, ButtonVariant, IconName, button, icon, input};
use gpui::{
    App, Context, ElementId, Entity, FocusHandle, Focusable, SharedString, Window, div, prelude::*,
    px,
};
use gpui_base::{Combobox, ElementExt as _, Popup, Select, input::InputState};

#[derive(Clone, Debug)]
pub struct ChoiceItem {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
}
impl ChoiceItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Application-owned choice and editing state. Observe this entity for selection changes.
/// Use one entity per control; Select and Combobox share the same value model.
pub struct ChoiceState {
    items: Vec<ChoiceItem>,
    selected: Option<usize>,
    cursor: Option<usize>,
    open: bool,
    disabled: bool,
    label: SharedString,
    placeholder: SharedString,
    trigger_focus: FocusHandle,
    popup_focus: FocusHandle,
    query: Entity<InputState>,
    searchable: bool,
    scroll: gpui::ScrollHandle,
    popup_width: gpui::Pixels,
}
impl ChoiceState {
    pub fn new(items: Vec<ChoiceItem>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search options…"));
        cx.subscribe(
            &query,
            |this, _, event: &gpui_base::input::InputEvent, cx| {
                if !matches!(event, gpui_base::input::InputEvent::Change) {
                    return;
                }
                this.cursor = if this.query.read(cx).value().trim().is_empty() {
                    this.selected
                        .or_else(|| this.items.iter().position(|item| !item.disabled))
                } else {
                    this.visible(cx)
                        .into_iter()
                        .find(|&i| !this.items[i].disabled)
                };
                this.scroll.set_offset(gpui::point(px(0.), px(0.)));
                cx.notify();
            },
        )
        .detach();
        Self {
            items,
            selected: None,
            cursor: None,
            open: false,
            disabled: false,
            label: "Choose an option".into(),
            placeholder: "Choose an option…".into(),
            trigger_focus: cx.focus_handle(),
            popup_focus: cx.focus_handle(),
            query,
            searchable: false,
            scroll: gpui::ScrollHandle::new(),
            popup_width: px(280.),
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn placeholder(mut self, label: impl Into<SharedString>) -> Self {
        self.placeholder = label.into();
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn default_selected(mut self, index: usize) -> Self {
        self.selected = self
            .items
            .get(index)
            .filter(|item| !item.disabled)
            .map(|_| index);
        self
    }
    pub fn selected(&self) -> Option<&ChoiceItem> {
        self.selected.and_then(|i| self.items.get(i))
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn set_selected(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.selected = index.filter(|&i| self.items.get(i).is_some_and(|item| !item.disabled));
        cx.notify();
    }
    fn visible(&self, cx: &App) -> Vec<usize> {
        let query = if self.searchable {
            self.query.read(cx).value().trim().to_lowercase()
        } else {
            String::new()
        };
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| item.label.to_lowercase().contains(&query).then_some(i))
            .collect()
    }
    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.open == open {
            return;
        }
        self.open = open;
        if open {
            if self.searchable {
                let style = cx.omarchy().input_style();
                self.query.update(cx, |query, cx| {
                    // Opening happens before the search input's first render.
                    // Set its font before edits measure text (system fonts are
                    // unavailable in the browser).
                    query.set_editor_style(style);
                    query.set_value("", window, cx);
                });
            }
            self.cursor = self
                .selected
                .or_else(|| self.items.iter().position(|item| !item.disabled));
            if self.searchable {
                self.query.update(cx, |query, cx| query.focus(window, cx));
            } else {
                self.popup_focus.focus(window, cx);
            }
        } else {
            self.trigger_focus.focus(window, cx);
        }
        cx.notify();
        window.refresh();
    }
    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.cursor.filter(|&i| !self.items[i].disabled) {
            self.selected = Some(index);
            self.set_open(false, window, cx);
        }
    }
    fn handle_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        match key {
            "up" | "down" | "home" | "end" => {
                if !self.open {
                    self.set_open(true, window, cx);
                } else {
                    self.navigate(key, cx);
                }
            }
            "enter" | "space" => {
                if self.open {
                    self.commit(window, cx);
                } else {
                    self.set_open(true, window, cx);
                }
            }
            "escape" => self.set_open(false, window, cx),
            "tab" | "shift-tab" => {
                self.set_open(false, window, cx);
                if key == "tab" {
                    window.focus_next(cx);
                } else {
                    window.focus_prev(cx);
                }
            }
            _ => {}
        }
        window.refresh();
    }
    fn navigate(&mut self, key: &str, cx: &mut Context<Self>) {
        let visible = self.visible(cx);
        let enabled: Vec<_> = visible
            .iter()
            .copied()
            .filter(|&i| !self.items[i].disabled)
            .collect();
        let position = self
            .cursor
            .and_then(|i| enabled.iter().position(|&v| v == i));
        self.cursor = if enabled.is_empty() {
            None
        } else {
            Some(
                enabled[match key {
                    "home" => 0,
                    "end" => enabled.len() - 1,
                    "up" => position.map_or(enabled.len() - 1, |i| {
                        (i + enabled.len() - 1) % enabled.len()
                    }),
                    _ => position.map_or(0, |i| (i + 1) % enabled.len()),
                }],
            )
        };
        if let Some(row) = self
            .cursor
            .and_then(|i| visible.iter().position(|&v| v == i))
        {
            self.scroll.scroll_to_item(row);
        }
        cx.notify();
    }
}
impl Focusable for ChoiceState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.trigger_focus.clone()
    }
}

/// An option picker with j/k and Ctrl-j/k/n/p navigation while open.
pub fn select(
    id: impl Into<ElementId>,
    state: &Entity<ChoiceState>,
    window: &mut Window,
    cx: &mut App,
) -> Select {
    let id = id.into();
    let body = presentation(id.clone(), state, false, window, cx);
    let choice = state.read(cx);
    let target = state.clone();
    Select::new(id)
        .open(choice.open)
        .disabled(choice.disabled)
        .accessibility_label(choice.label.clone())
        .focus_handle(&choice.trigger_focus)
        .on_open_change(move |open, window, cx| {
            target.update(cx, |state, cx| state.set_open(open, window, cx))
        })
        .w_full()
        .child(body)
}

/// A searchable picker with Ctrl-j/k/n/p navigation; plain letters edit the query.
pub fn combobox(
    id: impl Into<ElementId>,
    state: &Entity<ChoiceState>,
    window: &mut Window,
    cx: &mut App,
) -> Combobox {
    let id = id.into();
    let body = presentation(id.clone(), state, true, window, cx);
    let choice = state.read(cx);
    let target = state.clone();
    Combobox::new(id)
        .open(choice.open)
        .disabled(choice.disabled)
        .focus_handle(&choice.trigger_focus)
        .on_open_change(move |open, window, cx| {
            target.update(cx, |state, cx| state.set_open(open, window, cx))
        })
        .w_full()
        .child(body)
}

fn presentation(
    id: ElementId,
    state: &Entity<ChoiceState>,
    searchable: bool,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    state.update(cx, |state, _| state.searchable = searchable);
    let t = cx.omarchy().clone();
    let choice = state.read(cx);
    let label = choice
        .selected()
        .map(|item| item.label.clone())
        .unwrap_or_else(|| choice.placeholder.clone());
    let open = choice.open;
    let disabled = choice.disabled;
    let focus = choice.popup_focus.clone();
    let trigger_focus = choice.trigger_focus.clone();
    let accessible_label = choice.label.clone();
    let query = choice.query.clone();
    let visible = choice.visible(cx);
    let cursor = choice.cursor;
    let selected = choice.selected;
    let items = choice.items.clone();
    let scroll = choice.scroll.clone();
    let popup_width = choice.popup_width;
    let target = state.clone();
    let trigger = button("choice-trigger", "", ButtonVariant::Secondary, cx)
        .focusable(false)
        .disabled(disabled)
        .w_full()
        .justify_start()
        .accessibility_label(accessible_label.clone())
        .border_color(t.control_border())
        .bg(t.normal_fill())
        .child(div().flex_1().min_w_0().text_ellipsis().child(label))
        .child(icon(IconName::ChevronDown).size(px(14.)))
        .on_click(move |_, window, cx| {
            target.update(cx, |state, cx| state.set_open(!state.open, window, cx))
        });
    let measure = state.clone();
    let trigger = div()
        .w_full()
        .child(trigger)
        .on_prepaint(move |bounds, window, cx| {
            measure.update(cx, |state, _| {
                if state.popup_width != bounds.size.width {
                    state.popup_width = bounds.size.width;
                    window.request_animation_frame();
                }
            });
        });
    let mut popup = Popup::new((id, "popup"), trigger).w_full();
    if open {
        let close = state.clone();
        let confirm = state.clone();
        let mut content = div()
            .id("choice-popup")
            .debug_selector(|| "omarchy-choice-popup".into())
            .track_focus(&focus)
            .mt(px(4.))
            .w(popup_width)
            .max_w(gpui::relative(1.))
            .p(px(6.))
            .border_1()
            .border_color(t.control_border())
            .bg(t.background)
            .text_color(t.foreground)
            .font_family(t.font.clone())
            .text_size(px(12.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .on_mouse_down_out(move |_, window, cx| {
                close.update(cx, |state, cx| {
                    state.set_open(false, window, cx);
                })
            })
            .on_action(move |_: &gpui_base::actions::Confirm, window, cx| {
                cx.stop_propagation();
                confirm.update(cx, |state, cx| state.commit(window, cx));
            });
        if searchable {
            content = content.child(
                input("choice-search", &query, window, cx)
                    .h(px(28.))
                    .py(px(0.))
                    .px(px(6.)),
            );
        }
        let mut rows = div()
            .id("choice-options")
            .role(gpui::Role::ListBox)
            .max_h(px(224.))
            .overflow_y_scroll()
            .track_scroll(&scroll)
            .flex()
            .flex_col();
        if visible.is_empty() {
            rows = rows.child(
                div()
                    .p(px(10.))
                    .text_color(t.secondary)
                    .child("No matching options"),
            );
        }
        for index in visible {
            let target = state.clone();
            let hover = state.clone();
            let item = &items[index];
            rows = rows.child(
                button(("choice-option", index), "", ButtonVariant::Secondary, cx)
                    .accessibility_label(item.label.clone())
                    .debug_selector(move || format!("omarchy-choice-option-{index}").into())
                    .role(gpui::Role::ListBoxOption)
                    .aria_selected(selected == Some(index))
                    .when(cursor == Some(index), |row| row.aria_active_descendant())
                    .focusable(false)
                    .disabled(item.disabled)
                    .w_full()
                    .h(px(28.))
                    .py(px(0.))
                    .px(px(6.))
                    .gap(px(6.))
                    .justify_start()
                    .bg(if cursor == Some(index) {
                        t.selected_fill()
                    } else {
                        t.background
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .child(item.label.clone()),
                    )
                    .child(
                        div()
                            .w(px(14.))
                            .flex_shrink_0()
                            .when(selected == Some(index), |mark| {
                                mark.child(icon(IconName::Check).size(px(14.)))
                            }),
                    )
                    .on_hover(move |hovered, window, cx| {
                        if *hovered {
                            hover.update(cx, |state, cx| {
                                if !state.items[index].disabled {
                                    state.cursor = Some(index);
                                    cx.notify();
                                    window.refresh();
                                }
                            });
                        }
                    })
                    .on_click(move |_, window, cx| {
                        target.update(cx, |state, cx| {
                            state.cursor = Some(index);
                            state.commit(window, cx);
                        })
                    }),
            );
        }
        popup = popup.content(content.child(rows));
    }
    let mut root = div()
        .id("choice-keys")
        .aria_label(accessible_label)
        .when(!disabled, |root| {
            root.track_focus(&trigger_focus.tab_stop(true))
        })
        .w_full();
    macro_rules! action {
        ($ty:ty, $key:expr) => {{
            let target = state.clone();
            root = root.capture_action(move |_: &$ty, window, cx| {
                if target.read(cx).disabled {
                    return;
                }
                cx.stop_propagation();
                target.update(cx, |state, cx| state.handle_key($key, window, cx));
            });
        }};
    }
    action!(gpui_base::actions::Confirm, "enter");
    action!(gpui_base::actions::Cancel, "escape");
    action!(gpui_base::actions::SelectUp, "up");
    action!(gpui_base::actions::SelectDown, "down");
    action!(gpui_base::input::Enter, "enter");
    action!(gpui_base::input::Escape, "escape");
    action!(gpui_base::input::MoveUp, "up");
    action!(gpui_base::input::MoveDown, "down");
    action!(gpui_base::input::IndentInline, "tab");
    action!(gpui_base::input::OutdentInline, "shift-tab");
    let target = state.clone();
    root.capture_key_down(move |event, window, cx| {
        if disabled {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let navigation =
            if modifiers.control && !modifiers.alt && !modifiers.platform && !modifiers.shift {
                match key {
                    "j" | "n" => Some("down"),
                    "k" | "p" => Some("up"),
                    _ => None,
                }
            } else if !searchable && !modifiers.modified() {
                match key {
                    "j" => Some("down"),
                    "k" => Some("up"),
                    _ => None,
                }
            } else {
                None
            };
        if let Some(key) = navigation.filter(|_| target.read(cx).open) {
            target.update(cx, |state, cx| state.handle_key(key, window, cx));
        } else if key == "tab" {
            let backward = event.keystroke.modifiers.shift;
            target.update(cx, |state, cx| {
                state.handle_key(if backward { "shift-tab" } else { "tab" }, window, cx)
            });
        } else if !searchable
            && !event.keystroke.modifiers.modified()
            && matches!(key, "home" | "end" | "space")
        {
            target.update(cx, |state, cx| state.handle_key(key, window, cx));
        } else {
            return;
        }
        cx.stop_propagation();
    })
    .child(popup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Render, TestAppContext, VisualTestContext};
    struct Harness {
        state: Entity<ChoiceState>,
        searchable: bool,
    }
    impl Render for Harness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            crate::focus_scope("root")
                .size_full()
                .child(button("before", "Before", ButtonVariant::Secondary, cx))
                .child(div().w(px(280.)).child(if self.searchable {
                    combobox("choice", &self.state, window, cx).into_any_element()
                } else {
                    select("choice", &self.state, window, cx).into_any_element()
                }))
                .child(button("after", "After", ButtonVariant::Secondary, cx))
        }
    }
    fn harness(
        cx: &mut TestAppContext,
        searchable: bool,
        disabled: bool,
    ) -> (Entity<ChoiceState>, &mut VisualTestContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(move |window, cx| {
            let state = cx.new(|cx| {
                ChoiceState::new(
                    vec![
                        ChoiceItem::new("first", "First"),
                        ChoiceItem::new("disabled", "Disabled").disabled(true),
                        ChoiceItem::new("last", "Last"),
                    ],
                    window,
                    cx,
                )
                .default_selected(0)
                .disabled(disabled)
            });
            cx.observe(&state, |_, _, cx| cx.notify()).detach();
            Harness { state, searchable }
        });
        let state = view.read_with(cx, |view, _| view.state.clone());
        cx.update(|window, cx| {
            state.read(cx).trigger_focus.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (state, cx)
    }
    #[gpui::test]
    fn select_skips_disabled_commits_and_restores_focus(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, false, false);
        cx.simulate_keystrokes("down");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(state.read(cx).open);
        });
        cx.simulate_keystrokes("down enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(state.read(cx).selected().unwrap().value.as_ref(), "last");
            assert!(!state.read(cx).open);
            assert!(state.read(cx).trigger_focus.is_focused(window));
        });
    }
    #[gpui::test]
    fn select_vim_navigation_skips_disabled_and_wraps(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, false, false);
        cx.simulate_keystrokes("enter j");
        cx.update(|_, cx| assert_eq!(state.read(cx).cursor, Some(2)));
        cx.simulate_keystrokes("j");
        cx.update(|_, cx| assert_eq!(state.read(cx).cursor, Some(0)));
        cx.simulate_keystrokes("k enter");
        cx.update(|window, cx| {
            assert_eq!(state.read(cx).selected().unwrap().value.as_ref(), "last");
            assert!(!state.read(cx).open);
            assert!(state.read(cx).trigger_focus.is_focused(window));
        });
    }

    #[gpui::test]
    fn combobox_vim_navigation_preserves_search_input(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, true, false);
        cx.simulate_keystrokes("enter ctrl-j");
        cx.update(|_, cx| assert_eq!(state.read(cx).cursor, Some(2)));
        cx.simulate_keystrokes("ctrl-k");
        cx.update(|_, cx| assert_eq!(state.read(cx).cursor, Some(0)));
        cx.simulate_keystrokes("ctrl-n");
        cx.update(|_, cx| assert_eq!(state.read(cx).cursor, Some(2)));
        cx.simulate_keystrokes("ctrl-p");
        cx.update(|_, cx| assert_eq!(state.read(cx).cursor, Some(0)));
        cx.simulate_keystrokes("j k");
        cx.update(|_, cx| {
            let choice = state.read(cx);
            assert_eq!(choice.query.read(cx).value().as_ref(), "jk");
            assert!(choice.visible(cx).is_empty());
            assert_eq!(choice.cursor, None);
        });
        cx.simulate_keystrokes("ctrl-j enter");
        cx.update(|_, cx| assert!(state.read(cx).open));
    }

    #[gpui::test]
    fn combobox_filters_and_empty_result_cannot_commit(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, true, false);
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(state.read(cx).open);
        });
        cx.simulate_input("zzzz");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(state.read(cx).visible(cx).is_empty());
        });
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| assert!(state.read(cx).open));
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(state.read(cx).selected().unwrap().value.as_ref(), "first");
            assert!(state.read(cx).trigger_focus.is_focused(window));
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_input("LAST");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| {
            assert_eq!(state.read(cx).selected().unwrap().value.as_ref(), "last");
            assert!(!state.read(cx).open);
        });
    }
    #[gpui::test]
    fn disabled_choice_does_not_open(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, false, true);
        cx.simulate_keystrokes("down enter space");
        cx.simulate_click(gpui::point(px(10.), px(10.)), gpui::Modifiers::default());
        cx.update(|_, cx| assert!(!state.read(cx).open));
    }
    #[gpui::test]
    fn tab_leaves_combobox_without_focusing_hidden_search(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, true, false);
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let choice = state.read(cx);
            assert!(!choice.open);
            assert!(!choice.trigger_focus.is_focused(window));
            assert!(
                !choice.query.read(cx).focus_handle(cx).is_focused(window),
                "Tab must leave the closed popup's input"
            );
            assert!(window.focused(cx).is_some());
        });
        cx.simulate_keystrokes("shift-tab");
        cx.update(|window, cx| assert!(state.read(cx).trigger_focus.is_focused(window)));
    }

    #[gpui::test]
    fn pointer_selects_and_outside_click_preserves_value(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, false, false);
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let row = cx.debug_bounds("omarchy-choice-option-2").unwrap();
        cx.simulate_click(row.center(), gpui::Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(!state.read(cx).open);
            assert_eq!(state.read(cx).selected().unwrap().value.as_ref(), "last");
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_click(gpui::point(px(400.), px(400.)), gpui::Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(!state.read(cx).open);
            assert_eq!(state.read(cx).selected().unwrap().value.as_ref(), "last");
        });
    }
    #[gpui::test]
    fn keyboard_cursor_scrolls_long_list_into_view(cx: &mut TestAppContext) {
        let (state, cx) = harness(cx, false, false);
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.items = (0..30)
                    .map(|i| ChoiceItem::new(i.to_string(), format!("Workspace {i}")))
                    .collect();
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("end");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let last = cx.debug_bounds("omarchy-choice-option-29").unwrap();
        let popup = cx.debug_bounds("omarchy-choice-popup").unwrap();
        assert!(popup.contains(&last.center()));
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| assert_eq!(state.read(cx).selected().unwrap().value.as_ref(), "29"));
    }
}
