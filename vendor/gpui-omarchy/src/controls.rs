//! Styled constructors retain gpui-base's controlled-state builder APIs.
use crate::{ActiveTheme, Button, IconName, Link, icon};
use gpui::prelude::FluentBuilder;
use gpui::{
    App, ElementId, FontWeight, InteractiveElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px,
};
use gpui_base::{
    Checkbox, CheckboxIndicator, CheckboxState, Radio, Switch, SwitchThumb, SwitchTrack, Tab, Tabs,
    Toggle,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Outline,
    #[default]
    Secondary,
    Danger,
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    cx: &App,
) -> Button {
    let t = cx.omarchy();
    let (bg, fg) = match variant {
        ButtonVariant::Primary => (t.foreground.opacity(0.), t.accent),
        ButtonVariant::Outline | ButtonVariant::Secondary => {
            (t.foreground.opacity(0.), t.foreground)
        }
        ButtonVariant::Danger => (t.foreground.opacity(0.), t.danger),
    };
    let interaction_border = if variant == ButtonVariant::Primary {
        t.accent
    } else {
        t.focus_border()
    };
    let label = label.into();
    Button::new(id)
        .accessibility_label(label.clone())
        .when(!label.is_empty(), |button| button.child(label))
        .py(px(6.))
        .px(px(10.))
        .gap(px(8.))
        .border_1()
        .rounded(px(0.))
        .border_color(if variant == ButtonVariant::Primary {
            t.accent
        } else if variant == ButtonVariant::Outline {
            t.control_border()
        } else {
            t.foreground.opacity(0.)
        })
        .font_family(t.font.clone())
        .text_size(px(12.))
        .font_weight(FontWeight::NORMAL)
        .bg(bg)
        .text_color(fg)
        .hover(|s| s.bg(t.hover_fill()).border_color(interaction_border))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(interaction_border))
        .active(|s| s.bg(t.pressed_fill()))
        .styles(|s| {
            s.selected(|s| s.bg(t.selected_fill()))
                .disabled(|s| s.opacity(0.45))
        })
}

pub fn checkbox(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    state: CheckboxState,
    cx: &App,
) -> Checkbox {
    let t = cx.omarchy();
    let label = label.into();
    Checkbox::new(id)
        .state(state)
        .accessibility_label(label.clone())
        .flex()
        .items_center()
        .gap(px(8.))
        .min_h(px(28.))
        .px(px(4.))
        .border_1()
        .border_color(t.foreground.opacity(0.))
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .child(
            CheckboxIndicator::new()
                .state(state)
                .size(px(16.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(0.))
                .border_1()
                .border_color(t.control_border())
                .bg(t.normal_fill())
                .text_color(t.foreground)
                .styles(|s| {
                    s.checked(|s| {
                        s.bg(t.selected_fill())
                            .border_color(t.foreground.opacity(0.))
                    })
                    .indeterminate(|s| {
                        s.bg(t.selected_fill())
                            .border_color(t.foreground.opacity(0.))
                    })
                })
                .when(state != CheckboxState::Unchecked, |indicator| {
                    indicator.child(
                        icon(if state == CheckboxState::Checked {
                            IconName::Check
                        } else {
                            IconName::Minus
                        })
                        .size(px(14.))
                        .text_color(t.foreground),
                    )
                }),
        )
        .child(label)
        .hover(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .active(|s| s.bg(t.pressed_fill()))
        .styles(|s| s.disabled(|s| s.opacity(0.45)))
}

pub fn switch(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    checked: bool,
    cx: &App,
) -> Switch {
    let t = cx.omarchy();
    let id = id.into();
    let label = label.into();
    Switch::new(id.clone())
        .checked(checked)
        .accessibility_label(label.clone())
        .flex()
        .items_center()
        .gap(px(8.))
        .py(px(5.))
        .px(px(5.))
        .border_1()
        .border_color(t.foreground.opacity(0.))
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .hover(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .active(|s| s.bg(t.pressed_fill()))
        .styles(|s| s.disabled(|s| s.opacity(0.45)))
        .child(
            SwitchTrack::new(id)
                .checked(checked)
                .w(px(42.))
                .h(px(22.))
                .flex_shrink_0()
                .p(px(2.))
                .border_1()
                .border_color(if checked {
                    t.foreground.opacity(0.)
                } else {
                    t.control_border()
                })
                .rounded(px(0.))
                .bg(if checked {
                    t.selected_fill()
                } else {
                    t.normal_fill()
                })
                .child(
                    SwitchThumb::new(checked)
                        .size(px(16.))
                        .rounded(px(0.))
                        .bg(if checked { t.foreground } else { t.secondary })
                        .ml(px(if checked { 20. } else { 0. })),
                ),
        )
        .child(label)
}

pub fn radio(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    checked: bool,
    cx: &App,
) -> Radio {
    let t = cx.omarchy();
    let label = label.into();
    Radio::new(id)
        .checked(checked)
        .accessibility_label(label.clone())
        .flex()
        .items_center()
        .gap(px(8.))
        .min_h(px(28.))
        .px(px(4.))
        .border_1()
        .border_color(t.foreground.opacity(0.))
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .child(
            div()
                .size(px(16.))
                .flex_shrink_0()
                .rounded(px(0.))
                .border_1()
                .border_color(t.control_border())
                .flex()
                .items_center()
                .justify_center()
                .when(checked, |s| {
                    s.child(div().size(px(8.)).rounded(px(0.)).bg(t.foreground))
                }),
        )
        .child(label)
        .hover(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .active(|s| s.bg(t.pressed_fill()))
        .styles(|s| s.disabled(|s| s.opacity(0.45)))
}

pub fn toggle(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    pressed: bool,
    cx: &App,
) -> Toggle {
    let t = cx.omarchy();
    let label = label.into();
    Toggle::new(id)
        .pressed(pressed)
        .accessibility_label(label.clone())
        .flex()
        .items_center()
        .gap(px(8.))
        .py(px(6.))
        .px(px(10.))
        .border_1()
        .rounded(px(0.))
        .border_color(t.border)
        .text_color(t.foreground)
        .font_family(t.font.clone())
        .text_size(px(12.))
        .font_weight(FontWeight::NORMAL)
        .child(label)
        .hover(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .active(|s| s.border_color(t.bright))
        .styles(|s| {
            s.pressed(|s| s.bg(t.selected_fill()))
                .disabled(|s| s.opacity(0.45))
        })
}

/// A multi-choice toolbar. Compose independent `toggle` children; each retains
/// its own keyboard focus and controlled pressed state.
pub fn toggle_group(id: impl Into<ElementId>, cx: &App) -> gpui_base::ToggleGroup {
    let t = cx.omarchy();
    gpui_base::ToggleGroup::new(id)
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(6.))
        .font_family(t.font.clone())
        .text_size(px(12.))
        .text_color(t.foreground)
}

pub fn link(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    href: impl Into<SharedString>,
    cx: &App,
) -> Link {
    let t = cx.omarchy();
    let label = label.into();
    Link::new(id)
        .href(href)
        .open_with(|url, _, _, cx| cx.open_url(url))
        .accessibility_label(label.clone())
        .child(label)
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(12.))
        .font_family(t.font.clone())
        .text_color(t.accent)
        .underline()
        .border_1()
        .border_color(t.foreground.opacity(0.))
        .px(px(4.))
        .py(px(6.))
        .hover(|s| s.text_color(t.bright))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .styles(|s| s.disabled(|s| s.opacity(0.45)))
}

/// Omarchy ButtonGroup presentation with gpui-base Tab semantics.
/// The source shell has no separate Tabs primitive: bordered peers use a quiet
/// selected fill. The native port keeps label weight stable across selection.
pub fn tabs(id: impl Into<ElementId>, _cx: &App) -> Tabs {
    Tabs::new(id).flex().flex_wrap().gap(px(6.))
}

pub fn tab(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    cx: &App,
) -> Tab {
    let t = cx.omarchy();
    let label = label.into();
    Tab::new(id)
        .selected(selected)
        .accessibility_label(label.clone())
        .child(label)
        .px(px(10.))
        .py(px(6.))
        .flex()
        .items_center()
        .border_1()
        .rounded(px(0.))
        .border_color(t.control_border())
        .text_size(px(12.))
        .font_family(t.font.clone())
        .font_weight(FontWeight::NORMAL)
        .text_color(t.foreground)
        .hover(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .focus_visible(|s| s.bg(t.hover_fill()).border_color(t.focus_border()))
        .active(|s| s.bg(t.pressed_fill()))
        .styles(|s| {
            s.selected(|s| s.bg(t.selected_fill()))
                .disabled(|s| s.opacity(0.45))
        })
}
