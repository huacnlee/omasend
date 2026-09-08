use crate::ActiveTheme;
use gpui_kit::base::{ResizablePanel, ResizablePanelGroup};
use gpui_kit::{App, Axis, ElementId, div, prelude::*, px};
use std::rc::Rc;

/// Resizable panes with a fine divider and base-owned drag hit area and limits.
pub fn resizable(id: impl Into<ElementId>, axis: Axis, cx: &App) -> ResizablePanelGroup {
    let t = cx.omarchy().clone();
    ResizablePanelGroup::new(id)
        .axis(axis)
        .with_handle_appearance(Rc::new(move |handle, _, _| {
            Some(
                div()
                    .bg(if handle.is_active() {
                        t.accent
                    } else {
                        t.border
                    })
                    .when(handle.axis() == Axis::Horizontal, |line| {
                        line.w(px(1.)).h_full()
                    })
                    .when(handle.axis() == Axis::Vertical, |line| {
                        line.h(px(1.)).w_full()
                    })
                    .into_any_element(),
            )
        }))
}

pub fn resizable_panel() -> ResizablePanel {
    gpui_kit::base::resizable_panel()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::base::ResizableState;
    use gpui_kit::gpui;
    use gpui_kit::{Context, Entity, MouseButton, Render, TestAppContext, Window, point};
    struct Harness {
        state: Entity<ResizableState>,
        axis: Axis,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size(px(400.)).child(
                resizable("split", self.axis, cx)
                    .with_state(&self.state)
                    .child(
                        resizable_panel()
                            .size(px(160.))
                            .size_range(px(120.)..px(260.))
                            .child(
                                div()
                                    .size_full()
                                    .debug_selector(|| "resizable-first".into()),
                            ),
                    )
                    .child(
                        resizable_panel()
                            .size(px(240.))
                            .size_range(px(120.)..px(280.))
                            .child(
                                div()
                                    .size_full()
                                    .debug_selector(|| "resizable-second".into()),
                            ),
                    ),
            )
        }
    }
    #[gpui::test]
    fn styled_divider_drags_and_respects_panel_limits(cx: &mut TestAppContext) {
        cx.update(crate::init);
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let (view, cx) = cx.add_window_view(|_, cx| Harness {
                state: cx.new(|_| ResizableState::default()),
                axis,
            });
            for _ in 0..2 {
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
            let second = cx.debug_bounds("resizable-second").unwrap();
            let boundary = if axis == Axis::Horizontal {
                second.left()
            } else {
                second.top()
            };
            let pointer = |coordinate| {
                if axis == Axis::Horizontal {
                    point(coordinate, px(50.))
                } else {
                    point(px(50.), coordinate)
                }
            };
            cx.simulate_mouse_down(
                pointer(boundary - px(2.)),
                MouseButton::Left,
                Default::default(),
            );
            cx.simulate_mouse_move(
                pointer(boundary + px(10.)),
                Some(MouseButton::Left),
                Default::default(),
            );
            cx.simulate_mouse_move(
                pointer(px(350.)),
                Some(MouseButton::Left),
                Default::default(),
            );
            cx.simulate_mouse_up(pointer(px(350.)), MouseButton::Left, Default::default());
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let sizes = view.read(cx).state.read(cx).sizes();
                assert_eq!(sizes[0], px(260.));
                assert!((f32::from(sizes[1]) - 140.).abs() < 0.01);
            });
        }
    }
}
