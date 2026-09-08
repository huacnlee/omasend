//! Omarchy's presentation language on gpui-base's interaction primitives.
//!
//! Call [`init`] once, then use the constructors in [`controls`] and [`surface`]
//! inside `Render`. They retain base builder APIs; Button adds disabled-state style gating.

/// GPUI types and traits used to compose Omarchy applications.
pub use gpui;
/// Create an application using the current desktop platform.
#[cfg(not(target_family = "wasm"))]
pub use gpui::application;

pub mod button;
pub mod button_group;
pub mod calendar;
pub mod color_picker;
pub mod controls;
pub mod date_picker;
pub mod dialog;
pub mod dock;
pub mod focus;
pub mod hover_card;
pub mod icon;
pub mod input;
pub mod link;
pub mod list;
pub mod menu;
pub mod navigation;
pub mod otp_input;
pub mod popover;
pub mod resizable;
pub mod select;
pub mod sheet;
pub mod slider;
pub mod surface;
mod system_theme;
pub mod table;
pub mod text;
pub mod theme;
pub mod tooltip;
pub mod tree;

pub use button::Button;
pub use button_group::{button_group, tab_list};
pub use calendar::calendar;
pub use color_picker::color_picker;
pub use controls::*;
pub use date_picker::{DatePickerState, date_picker};
pub use dialog::*;
pub use dock::dock_area;
pub use focus::focus_scope;
pub use hover_card::hover_card;
pub use icon::{IconName, icon};
pub use input::{input, number_input, textarea};
pub use link::Link;
pub use list::{scrollbar, virtual_list};
pub use menu::{MenuItem, menu};
pub use navigation::*;
pub use otp_input::{OtpInput, otp_input};
pub use popover::{popover, popover_surface};
pub use resizable::{resizable, resizable_panel};
pub use select::{ChoiceItem, ChoiceState, combobox, select};
pub use sheet::{sheet, sheet_surface};
pub use slider::slider;
pub use surface::*;
pub use system_theme::ThemeLoadError;
pub use table::*;
pub use text::{html, markdown, text_view_style};
pub use theme::{ActiveTheme, Theme};
pub use tooltip::{tooltip, with_tooltip};
pub use tree::tree;

/// Initialize base behavior and the current Omarchy theme (Tokyo Night when unavailable).
pub fn init(cx: &mut gpui::App) {
    gpui_base::init(cx);
    focus::init(cx);
    button_group::init(cx);
    popover::init(cx);
    date_picker::init(cx);
    Theme::follow_system(cx);
}
