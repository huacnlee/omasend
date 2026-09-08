use super::Home;
use gpui_kit::{self, AnyElement, Context, Pixels, SharedString, Window, div, prelude::*, rems};
use gpui_omarchy::{ActiveTheme, ButtonVariant, IconName, avatar, button, icon};

impl Home {
    pub fn nearby(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let (columns, width) = nearby_layout(window);
        let visible = nearby_indices(
            self.state.devices.len(),
            columns,
            self.nearby_expanded,
            self.state
                .devices
                .iter()
                .position(|device| self.state.selected.as_ref() == Some(&device.fingerprint)),
        );
        let rows = visible.len().div_ceil(columns);
        let card_height = 3.625;
        let row_height = card_height + if rows > 1 { super::CONTROL_GAP } else { 0. };
        let list = if rows == 0 {
            div()
                .text_color(theme.secondary)
                .child(
                    self.language
                        .text("Looking for devices… Open LocalSend on the same Wi-Fi."),
                )
                .into_any_element()
        } else {
            gpui_kit::uniform_list(
                "nearby-list",
                rows,
                cx.processor(move |view, range: std::ops::Range<usize>, _, cx| {
                    range
                        .map(|row| {
                            div()
                                .h(rems(row_height))
                                .flex()
                                .gap(rems(super::CONTROL_GAP))
                                .children(
                                    visible
                                        [row * columns..((row + 1) * columns).min(visible.len())]
                                        .iter()
                                        .map(|&index| view.device_card(index, width, cx)),
                                )
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .w_full()
            .h(rems(row_height * rows.min(3) as f32))
            .track_scroll(&self.nearby_scroll)
            .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap(rems(super::CONTROL_GAP))
            .px(rems(super::PANEL_PADDING))
            .pt(rems(super::PANEL_GAP))
            // Multi-row lists already reserve the control gap after the last card.
            .pb(rems(super::PANEL_GAP - (row_height - card_height)))
            .border_b_1()
            .border_color(theme.divider())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_weight(gpui_kit::FontWeight::BOLD)
                                    .child(self.language.text("Nearby")),
                            )
                            .child(
                                div()
                                    .text_color(theme.secondary)
                                    .child(self.state.devices.len().to_string()),
                            ),
                    )
                    .when(self.state.devices.len() > columns, |header| {
                        header.child(
                            button(
                                "toggle-nearby",
                                self.language.text(if self.nearby_expanded {
                                    "Collapse"
                                } else {
                                    "Expand"
                                }),
                                ButtonVariant::Secondary,
                                cx,
                            )
                            .flex_shrink_0()
                            .px_2()
                            .py_1()
                            .child(
                                icon(if self.nearby_expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .size(rems(0.75)),
                            )
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.nearby_expanded = !view.nearby_expanded;
                                view.nearby_scroll
                                    .scroll_to_item(0, gpui_kit::ScrollStrategy::Top);
                                cx.notify();
                            })),
                        )
                    }),
            )
            .child(list)
            .into_any_element()
    }

    fn device_card(&self, index: usize, width: Pixels, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let device = &self.state.devices[index];
        let id = device.fingerprint.clone();
        let selected = self.state.selected.as_ref() == Some(&id);
        let initials = device_initials(&device.alias);
        // Hash the displayed letters, not list order or a transient connection ID.
        let hash = initials.bytes().fold(0x811c9dc5u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x01000193)
        });
        let colors = [theme.accent, theme.success, theme.warning, theme.danger];
        let avatar_color = colors[hash as usize % colors.len()];
        let edge = if selected {
            theme.accent
        } else {
            theme.divider()
        };
        button(
            SharedString::from(format!("device-{id}")),
            "",
            ButtonVariant::Secondary,
            cx,
        )
        .selected(selected)
        .w(width)
        .h(rems(3.625))
        .flex_shrink_0()
        .justify_start()
        .items_center()
        .gap_3()
        .p_3()
        .border_color(edge)
        .styles(|s| s.selected(|s| s.bg(theme.normal_fill())))
        .hover(|style| style.bg(theme.hover_fill()).border_color(edge))
        .focus_visible(|style| {
            style.bg(theme.hover_fill()).border_color(if selected {
                edge
            } else {
                theme.focus_border()
            })
        })
        .accessibility_label(self.language.named("Select {name}", &device.alias))
        .child(
            avatar(initials, cx)
                .size(rems(2.))
                .border_color(avatar_color.opacity(0.45))
                .bg(avatar_color.opacity(0.12))
                .text_color(avatar_color)
                .text_size(rems(0.6875)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap_0()
                .line_height(rems(1.))
                .child(
                    div()
                        .text_ellipsis()
                        .overflow_hidden()
                        .text_color(theme.foreground)
                        .child(device.alias.clone()),
                )
                .child(
                    div()
                        .text_size(rems(0.6875))
                        .text_color(theme.secondary)
                        .text_ellipsis()
                        .overflow_hidden()
                        .child(if device.model.is_empty() {
                            "LocalSend".to_owned()
                        } else {
                            device.model.clone()
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(rems(0.875))
                .flex_shrink_0()
                .text_color(theme.accent)
                .when(selected, |slot| {
                    slot.child(icon(IconName::Check).size(rems(0.875)))
                }),
        )
        .on_click(cx.listener(move |view, _, window, cx| {
            view.state.selected = Some(id.clone());
            view.focus.focus(window, cx);
            cx.notify();
        }))
        .map(|button| {
            gpui_omarchy::with_tooltip(button, format!("{}\n{}", device.alias, device.model))
        })
        .into_any_element()
    }

    pub(super) fn reveal_selected_device(&self, window: &Window) {
        if self.nearby_expanded
            && let Some(index) = self
                .state
                .devices
                .iter()
                .position(|device| self.state.selected.as_ref() == Some(&device.fingerprint))
        {
            self.nearby_scroll.scroll_to_item(
                index / nearby_layout(window).0,
                gpui_kit::ScrollStrategy::Center,
            );
        }
    }
}

fn nearby_layout(window: &Window) -> (usize, Pixels) {
    let gap = rems(super::CONTROL_GAP).to_pixels(window.rem_size());
    let available =
        window.viewport_size().width - rems(super::PANEL_PADDING * 2.).to_pixels(window.rem_size());
    let columns =
        (((available + gap) / rems(15.).to_pixels(window.rem_size())).floor() as usize).max(1);
    (
        columns,
        (available - gap * (columns - 1) as f32) / columns as f32,
    )
}

fn device_initials(name: &str) -> String {
    let mut words = name.split_whitespace();
    let first = words.next().unwrap_or("?");
    let letters = match words.next() {
        Some(second) => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect::<String>(),
        None => first.chars().take(2).collect(),
    };
    letters.to_uppercase().chars().take(2).collect()
}

fn nearby_indices(
    total: usize,
    columns: usize,
    expanded: bool,
    selected: Option<usize>,
) -> Vec<usize> {
    if expanded {
        return (0..total).collect();
    }
    let mut visible: Vec<_> = (0..total.min(columns)).collect();
    if let Some(selected) = selected.filter(|&index| index < total && index >= columns)
        && let Some(last) = visible.last_mut()
    {
        *last = selected;
    }
    visible
}
