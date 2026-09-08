//! Keyboard traversal for a composed desktop region.
use gpui::{App, Div, ElementId, KeyBinding, Stateful, Window, actions, div, prelude::*};

actions!(omarchy_focus, [Next, Previous]);

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(
            if cfg!(target_os = "macos") {
                "cmd-c"
            } else {
                "ctrl-c"
            },
            gpui_base::input::Copy,
            Some("OmarchyFocusScope"),
        ),
        KeyBinding::new("tab", Next, Some("OmarchyFocusScope")),
        KeyBinding::new("shift-tab", Previous, Some("OmarchyFocusScope")),
    ]);
}

/// Wrap a window or form in this scope to connect Tab and Shift+Tab to GPUI's
/// tab order. Nested editors may consume Tab for indentation; single-line
/// fields propagate it here. Popup-specific handlers take priority.
pub fn focus_scope(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .tab_group()
        .key_context("OmarchyFocusScope")
        .on_action(|_: &gpui_base::input::Copy, window: &mut Window, cx| {
            let selected = gpui_base::TextSelection::selected_text(window, cx);
            if selected.is_empty() {
                cx.propagate();
            } else {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
            }
        })
        .on_action(|_: &Next, window: &mut Window, cx| traverse(window, cx, false))
        .on_action(|_: &Previous, window: &mut Window, cx| traverse(window, cx, true))
        .on_action(
            |_: &gpui_base::input::IndentInline, window: &mut Window, cx| {
                traverse(window, cx, false)
            },
        )
        .on_action(
            |_: &gpui_base::input::OutdentInline, window: &mut Window, cx| {
                traverse(window, cx, true)
            },
        )
}

// GPUI's window tab order includes background controls. Respect the modal
// boundary registered by gpui-base before settling on the next tab stop.
fn traverse(window: &mut Window, cx: &mut App, backwards: bool) {
    let trap = gpui_base::active_focus_trap(window, cx);
    let mut visited = Vec::new();
    loop {
        if backwards {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        let Some(trap) = &trap else {
            return;
        };
        if trap.contains_focused(window, cx) {
            return;
        }
        let focused = window.focused(cx);
        if focused.is_none() || visited.contains(&focused) {
            trap.focus(window, cx);
            return;
        }
        visited.push(focused);
    }
}
