//! Short, non-interactive explanations with GPUI's tooltip lifecycle.
use crate::ActiveTheme;
use gpui_kit::{
    App, AppContext, Context, Render, SharedString, StatefulInteractiveElement, Window, prelude::*,
    px,
};
use std::time::Duration;

/// A composable tooltip surface, retaining the base tooltip role and styling API.
pub fn tooltip(text: impl Into<SharedString>, cx: &App) -> gpui_kit::base::Tooltip {
    let t = cx.omarchy();
    gpui_kit::base::Tooltip::new("omarchy-tooltip")
        .max_w(px(320.))
        .px(px(10.))
        .py(px(6.))
        .border_1()
        .rounded(px(0.))
        .border_color(t.control_border())
        .bg(t.background)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(11.))
        .child(text.into())
}

/// Attach an Omarchy tooltip without changing the control's concrete type.
/// GPUI owns hover delay, placement, dismissal, and cleanup when the owner leaves.
/// Keep an accessible label on icon-only controls; a tooltip is supplementary.
pub fn with_tooltip<T: StatefulInteractiveElement>(control: T, text: impl Into<SharedString>) -> T {
    let text = text.into();
    control
        .tooltip_show_delay(Duration::from_millis(400))
        .tooltip(move |_, cx| cx.new(|_| TooltipText(text.clone())).into())
}

struct TooltipText(SharedString);
impl Render for TooltipText {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::div()
            .debug_selector(|| "omarchy-tooltip-surface".into())
            .child(tooltip(self.0.clone(), cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::gpui;
    use gpui_kit::{Render, TestAppContext, point};
    struct Harness;
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            gpui_kit::div().size_full().child(with_tooltip(
                crate::button("favorite", "Favorite", crate::ButtonVariant::Secondary, cx),
                "Add to favorites",
            ))
        }
    }
    #[gpui::test]
    fn tooltip_waits_for_hover_and_hides_on_pointer_leave(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| Harness);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_event(gpui_kit::MouseMoveEvent {
            position: point(px(10.), px(10.)),
            pressed_button: None,
            modifiers: Default::default(),
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.dispatcher.advance_clock(Duration::from_millis(399));
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("omarchy-tooltip-surface").is_none());
        cx.dispatcher.advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("omarchy-tooltip-surface").is_some());
        cx.simulate_event(gpui_kit::MouseMoveEvent {
            position: point(px(400.), px(400.)),
            pressed_button: None,
            modifiers: Default::default(),
        });
        cx.dispatcher.advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("omarchy-tooltip-surface").is_none());
    }
}
