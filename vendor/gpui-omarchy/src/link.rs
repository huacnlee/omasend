//! Link presentation that suppresses transient styles while disabled.
use gpui_kit::base::LinkStyles;
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, Interactivity, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window,
};

/// An Omarchy link backed by gpui-base's activation and focus behavior.
/// Transient styles are applied only after the final disabled state is known.
#[derive(IntoElement)]
pub struct Link {
    base: gpui_kit::base::Link,
    disabled: bool,
    hover: Option<Box<StyleRefinement>>,
    active: Option<Box<StyleRefinement>>,
    focus_visible: Option<Box<StyleRefinement>>,
}

impl Link {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: gpui_kit::base::Link::new(id),
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

    pub fn styles(mut self, build: impl FnOnce(LinkStyles) -> LinkStyles) -> Self {
        self.base = self.base.styles(build);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.base = self.base.accessibility_label(label);
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

    pub fn href(mut self, href: impl Into<SharedString>) -> Self {
        self.base = self.base.href(href);
        self
    }

    pub fn open_with(
        mut self,
        open: impl Fn(&str, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.open_with(open);
        self
    }

    pub fn on_activate(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_activate(handler);
        self
    }
}

impl Styled for Link {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Link {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for Link {
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

impl StatefulInteractiveElement for Link {
    fn active(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.active = Some(Box::new(build(StyleRefinement::default())));
        self
    }
}

impl RenderOnce for Link {
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
    use gpui_kit::gpui;
    use gpui_kit::{Context, Modifiers, MouseButton, Render, TestAppContext, div, point, px};
    #[gpui::test]
    fn disabled_link_suppresses_hover_and_pressed_geometry(cx: &mut TestAppContext) {
        cx.update(crate::init);
        struct States {
            disabled: bool,
            disabled_first: bool,
        }
        impl Render for States {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let link = crate::link("link", "Project", "https://example.com", cx)
                    .w(px(100.))
                    .h(px(40.));
                let link = if self.disabled_first {
                    link.disabled(self.disabled)
                        .hover(|s| s.w(px(160.)))
                        .active(|s| s.w(px(200.)))
                } else {
                    link.hover(|s| s.w(px(160.)))
                        .active(|s| s.w(px(200.)))
                        .disabled(self.disabled)
                };
                div()
                    .size(px(400.))
                    .child(link.debug_selector(|| "state-link".into()))
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
            assert_eq!(cx.debug_bounds("state-link").unwrap().size.width, px(100.));
            cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(cx.debug_bounds("state-link").unwrap().size.width, px(100.));
            cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
            view.update(cx, |this, cx| {
                this.disabled = false;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.simulate_mouse_move(point(px(300.), px(300.)), None, Modifiers::default());
            cx.simulate_mouse_move(position, None, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(cx.debug_bounds("state-link").unwrap().size.width, px(160.));
            cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(cx.debug_bounds("state-link").unwrap().size.width, px(200.));
            cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        }
    }
}
