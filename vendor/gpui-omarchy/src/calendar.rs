//! Calendar presentation over gpui-base's date and view state.
use crate::{ActiveTheme, IconName, icon};
use gpui::{App, ElementId, Entity, Role, prelude::*, px};
use gpui_base::{Calendar, CalendarItemKind, CalendarState};

/// A calendar with selectable days, month/year navigation and range styling.
/// Observe the supplied state or subscribe to base CalendarEvent::Selected.
pub fn calendar(id: impl Into<ElementId>, state: &Entity<CalendarState>, cx: &App) -> Calendar {
    let t = cx.omarchy().clone();
    Calendar::new(id, state)
        .w(px(248.))
        .p(px(12.))
        .gap(px(8.))
        .border_1()
        .rounded(px(0.))
        .border_color(t.border)
        .bg(t.background)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
        .label(|kind, value| match kind {
            CalendarItemKind::Weekday => {
                ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"][value as usize].into()
            }
            CalendarItemKind::Month | CalendarItemKind::MonthToggle => [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ][(value - 1) as usize]
                .into(),
            CalendarItemKind::Previous => "Previous".into(),
            CalendarItemKind::Next => "Next".into(),
            _ => value.to_string().into(),
        })
        .item(move |item, state, _, _| {
            let kind = state.kind();
            let interactive = kind != CalendarItemKind::Weekday;
            let mut item = item
                .flex()
                .items_center()
                .justify_center()
                .h(px(28.))
                .px(px(4.))
                .border_1()
                .rounded(px(0.))
                .border_color(if state.is_today() {
                    t.accent
                } else {
                    t.foreground.opacity(0.)
                })
                .text_color(if state.is_muted() {
                    t.secondary
                } else {
                    t.foreground
                })
                .bg(if state.is_active() {
                    t.selected_fill()
                } else if state.is_in_range() {
                    t.hover_fill()
                } else {
                    t.foreground.opacity(0.)
                })
                .when(interactive, |item| {
                    item.role(Role::Button)
                        .when(!state.is_disabled(), |item| item.focusable())
                })
                .when(state.is_disabled() && interactive, |item| {
                    item.opacity(0.45)
                })
                .when(!state.is_disabled(), |item| {
                    item.cursor_pointer()
                        .hover(|s| s.bg(t.hover_fill()))
                        .focus_visible(|s| s.border_color(t.accent))
                })
                .when(
                    matches!(
                        kind,
                        CalendarItemKind::Day
                            | CalendarItemKind::Weekday
                            | CalendarItemKind::Previous
                            | CalendarItemKind::Next
                    ),
                    |item| item.w(px(32.)).flex_shrink_0(),
                );
            if matches!(kind, CalendarItemKind::Previous | CalendarItemKind::Next) {
                let previous = kind == CalendarItemKind::Previous;
                item = item
                    .clear_children()
                    .aria_label(if previous {
                        "Previous calendar page"
                    } else {
                        "Next calendar page"
                    })
                    .debug_selector(move || {
                        if previous {
                            "calendar-previous"
                        } else {
                            "calendar-next"
                        }
                        .into()
                    })
                    .child(
                        icon(if previous {
                            IconName::ChevronLeft
                        } else {
                            IconName::ChevronRight
                        })
                        .size(px(14.)),
                    );
            }
            item.into_any_element()
        })
}
