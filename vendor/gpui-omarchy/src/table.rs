//! Primitive tables: application-owned columns, rows and content.
use crate::ActiveTheme;
use gpui::{App, ElementId, FontWeight, InteractiveElement, Styled, prelude::FluentBuilder, px};
use gpui_base::{Table, TableCell, TableHead, TableRow};

pub fn table(id: impl Into<ElementId>, cx: &App) -> Table {
    let t = cx.omarchy();
    Table::new(id)
        .w_full()
        .border_1()
        .border_color(t.border)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
}

/// `index` is the one-based accessibility row index, including the header.
/// Rows after the first draw a top separator; the table owns the outer border.
pub fn table_row(id: impl Into<ElementId>, index: usize, cx: &App) -> TableRow {
    let t = cx.omarchy();
    TableRow::new(id, index)
        .flex()
        .min_h(px(28.))
        .when(index > 1, |row| row.border_t_1())
        .border_color(t.border)
        .hover(|s| s.bg(t.surface))
}

/// `index` is the one-based accessibility column index.
pub fn table_head(id: impl Into<ElementId>, index: usize, cx: &App) -> TableHead {
    let t = cx.omarchy();
    TableHead::new(id, index)
        .flex_1()
        .min_w_0()
        .px(px(8.))
        .py(px(6.))
        .bg(t.surface)
        .text_color(t.bright)
        .font_weight(FontWeight::BOLD)
}

pub fn table_cell(id: impl Into<ElementId>, index: usize, _cx: &App) -> TableCell {
    TableCell::new(id, index)
        .flex_1()
        .min_w_0()
        .px(px(8.))
        .py(px(6.))
}
