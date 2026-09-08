//! Virtualized lists retain base's sizing and scroll model.
use crate::ActiveTheme;
use gpui::{
    App, Context, ElementId, Entity, IntoElement, Pixels, Render, Size, Styled, Window, px,
};
use std::{ops::Range, rc::Rc};

/// Each supplied size must match the corresponding row's rendered height.
/// Only the visible range is built. Attach a base VirtualListScrollHandle to
/// preserve scroll position and support navigation to a specific item.
pub fn virtual_list<V: Render, R: IntoElement>(
    view: Entity<V>,
    id: impl Into<ElementId>,
    sizes: Rc<Vec<Size<Pixels>>>,
    render: impl Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R> + 'static,
    cx: &App,
) -> gpui_base::VirtualList {
    let t = cx.omarchy();
    gpui_base::v_virtual_list(view, id, sizes, render)
        .w_full()
        .h(px(280.))
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
        .bg(t.background)
}

/// Overlay scrollbar for a scrollable region. Render after that region inside a
/// relative parent, using the same handle as the scrollable content.
pub fn scrollbar<H: gpui_base::ScrollbarHandle + Clone>(
    id: impl Into<ElementId>,
    axis: gpui_base::ScrollbarAxis,
    handle: &H,
    cx: &App,
) -> gpui_base::Scrollbar {
    let t = cx.omarchy();
    gpui_base::Scrollbar::new(handle)
        .id(id)
        .axis(axis)
        .mode(gpui_base::ScrollbarMode::Always)
        .styles(|style| {
            style
                .track(|track| track.width(px(8.)).bg(t.normal_fill()))
                .thumb(|thumb| {
                    thumb
                        .width(px(6.))
                        .inset(px(1.))
                        .radius(px(0.))
                        .bg(t.foreground.opacity(0.35))
                })
                .thumb_hover(|thumb| thumb.bg(t.foreground.opacity(0.55)))
                .thumb_active(|thumb| thumb.bg(t.foreground.opacity(0.7)))
        })
}
