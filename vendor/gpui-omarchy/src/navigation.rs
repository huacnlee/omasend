//! Composable section navigation with gpui-base state and accessibility.
use crate::{ActiveTheme, ButtonVariant, button};
use gpui_kit::base::{
    Accordion, AccordionPanel, AccordionTrigger, Pagination, PaginationItem, PaginationState,
};
use gpui_kit::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, div, px,
};

pub fn accordion(id: impl Into<ElementId>, cx: &App) -> Accordion {
    Accordion::new(id)
        .w_full()
        .border_t_1()
        .border_color(cx.omarchy().border)
}

pub fn accordion_trigger(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    open: bool,
    cx: &App,
) -> AccordionTrigger {
    let t = cx.omarchy();
    AccordionTrigger::new(id)
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .h(px(32.))
        .px(px(8.))
        .border_2()
        .border_color(t.background)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .font_weight(FontWeight::BOLD)
        .hover(|s| s.bg(t.surface))
        .focus_visible(|s| s.border_color(t.accent))
        .child(label.into())
        .child(crate::icon(if open {
            crate::IconName::ChevronDown
        } else {
            crate::IconName::ChevronRight
        }))
}

pub fn accordion_panel(cx: &App) -> AccordionPanel {
    let t = cx.omarchy();
    AccordionPanel::new()
        .px(px(10.))
        .py(px(8.))
        .border_b_1()
        .border_color(t.border)
        .text_size(px(12.))
        .font_family(t.font.clone())
        .text_color(t.secondary)
}

pub fn pagination(id: impl Into<ElementId>, state: PaginationState, cx: &App) -> Pagination {
    let items = state.items();
    Pagination::new(id, state.clone())
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(4.))
        .children(items.into_iter().map(|item| {
            match item {
                PaginationItem::Page(page) => {
                    let state = state.clone();
                    button(
                        ("page", page),
                        page.to_string(),
                        ButtonVariant::Secondary,
                        cx,
                    )
                    .selected(page == state.current_page())
                    .w(px(32.))
                    .px(px(0.))
                    .on_click(move |_, window, cx| state.request_page(page, window, cx))
                    .into_any_element()
                }
                PaginationItem::Ellipsis(_) => div()
                    .w(px(24.))
                    .text_color(cx.omarchy().secondary)
                    .child("…")
                    .into_any_element(),
            }
        }))
}

/// A controlled expandable region. Ordinary children remain visible; `content`
/// is rendered only while open. The caller owns the trigger and its state.
pub fn collapsible(open: bool, cx: &App) -> gpui_kit::base::Collapsible {
    gpui_kit::base::Collapsible::new()
        .open(open)
        .flex()
        .flex_col()
        .gap(px(14.))
        .w_full()
        .text_color(cx.omarchy().foreground)
        .text_size(px(12.))
}

/// Stateful page navigation with the base push/pop/forward lifecycle.
pub fn nav_stack(
    state: &gpui_kit::Entity<gpui_kit::base::NavStackState>,
    cx: &App,
) -> gpui_kit::base::NavStack {
    let t = cx.omarchy();
    gpui_kit::base::NavStack::new(state)
        .w_full()
        .h(px(280.))
        .border_1()
        .border_color(t.border)
        .bg(t.background)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
}
