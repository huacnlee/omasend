use crate::popover_surface;
use gpui_kit::base::{HoverCard, HoverCardState};
use gpui_kit::{ElementId, IntoElement, ParentElement, Styled, prelude::*, px};
use std::time::Duration;

/// Supplementary hover content. Keep essential information available inline.
pub fn hover_card<E: IntoElement>(
    id: impl Into<ElementId>,
    trigger: impl IntoElement,
    content: impl FnOnce(
        &mut HoverCardState,
        &mut gpui_kit::Window,
        &mut gpui_kit::Context<HoverCardState>,
    ) -> E
    + 'static,
) -> HoverCard {
    HoverCard::new(id)
        .trigger(trigger)
        .open_delay(Duration::from_millis(400))
        .close_delay(Duration::from_millis(200))
        .content(move |state, window, cx| {
            let body = content(state, window, cx);
            popover_surface(cx)
                .id("hover-card-surface")
                .mt(px(4.))
                .child(body)
        })
}
