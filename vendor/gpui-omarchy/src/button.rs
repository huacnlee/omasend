//! Button presentation that suppresses transient styles while disabled.
use gpui::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window,
};
use gpui_base::{ButtonStyles, RoleOverride};

/// An Omarchy button backed by gpui-base's activation and focus behavior.
/// Transient styles are applied only after the final disabled state is known.
#[derive(IntoElement)]
pub struct Button {
    base: gpui_base::Button,
    disabled: bool,
    hover: Option<Box<StyleRefinement>>,
    active: Option<Box<StyleRefinement>>,
    focus_visible: Option<Box<StyleRefinement>>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: gpui_base::Button::new(id),
            disabled: false,
            hover: None,
            active: None,
            focus_visible: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.base = self.base.disabled(disabled);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.base = self.base.selected(selected);
        self
    }

    pub fn styles(mut self, build: impl FnOnce(ButtonStyles) -> ButtonStyles) -> Self {
        self.base = self.base.styles(build);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.base = self.base.accessibility_label(label);
        self
    }

    pub fn role(mut self, role: impl Into<RoleOverride>) -> Self {
        self.base = self.base.role(role);
        self
    }

    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.base = self.base.track_focus(focus);
        self
    }

    pub fn tab_index(mut self, index: isize) -> Self {
        self.base = self.base.tab_index(index);
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.base = self.base.tab_stop(tab_stop);
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.base = self.base.focusable(focusable);
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_click(handler);
        self
    }
}

impl gpui_base::Selectable for Button {
    fn selected(self, selected: bool) -> Self {
        Button::selected(self, selected)
    }

    fn is_selected(&self) -> bool {
        self.base.is_selected()
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }

    fn hover(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.hover = Some(Box::new(build(StyleRefinement::default())));
        self
    }
    fn focus_visible(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.focus_visible = Some(Box::new(build(StyleRefinement::default())));
        self
    }
}

impl StatefulInteractiveElement for Button {
    fn active(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.active = Some(Box::new(build(StyleRefinement::default())));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut base = self.base;
        if !self.disabled {
            if let Some(style) = self.hover {
                base = base.hover(|_| *style);
            }
            if let Some(style) = self.active {
                base = base.active(|_| *style);
            }
            if let Some(style) = self.focus_visible {
                base = base.focus_visible(|_| *style);
            }
        }
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Modifiers, MouseButton, Render, TestAppContext, div, point, px};
    #[gpui::test]
    fn disabled_button_suppresses_hover_and_pressed_geometry(cx: &mut TestAppContext) {
        cx.update(crate::init);
        struct States {
            disabled: bool,
            disabled_first: bool,
        }
        impl Render for States {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let button = crate::button("button", "Apply", crate::ButtonVariant::Primary, cx)
                    .w(px(100.))
                    .h(px(40.));
                let button = if self.disabled_first {
                    button
                        .disabled(self.disabled)
                        .hover(|s| s.w(px(160.)))
                        .active(|s| s.w(px(200.)))
                } else {
                    button
                        .hover(|s| s.w(px(160.)))
                        .active(|s| s.w(px(200.)))
                        .disabled(self.disabled)
                };
                div()
                    .size(px(400.))
                    .child(button.debug_selector(|| "state-button".into()))
            }
        }
        for disabled_first in [false, true] {
            let (view, cx) = cx.add_window_view(move |_, _| States {
                disabled: true,
                disabled_first,
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let position = point(px(10.), px(10.));
            cx.simulate_mouse_move(position, None, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                cx.debug_bounds("state-button").unwrap().size.width,
                px(100.)
            );
            cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                cx.debug_bounds("state-button").unwrap().size.width,
                px(100.)
            );
            cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
            view.update(cx, |this, cx| {
                this.disabled = false;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.simulate_mouse_move(point(px(300.), px(300.)), None, Modifiers::default());
            cx.simulate_mouse_move(position, None, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                cx.debug_bounds("state-button").unwrap().size.width,
                px(160.)
            );
            cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                cx.debug_bounds("state-button").unwrap().size.width,
                px(200.)
            );
            cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        }
    }
}
