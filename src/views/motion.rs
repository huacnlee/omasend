//! State-driven transitions and a segmented discovery indicator.
//! GPUI automatically renders the final frame when reduced motion is enabled.
use gpui_omarchy::gpui::Animation;
use std::time::Duration;

pub fn content_enter() -> Animation {
    Animation::new(Duration::from_millis(120)).with_easing(ease_out)
}

pub fn popup_enter() -> Animation {
    Animation::new(Duration::from_millis(140)).with_easing(ease_out)
}

fn ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

/// Animate theme-colored segments of the shared pixel emblem.
/// The center remains steady; the surrounding eight regions advance in steps.
pub fn discovery_logo(
    color: gpui_omarchy::gpui::Hsla,
    discovering: bool,
    revision: u64,
    size: f32,
) -> gpui_omarchy::gpui::AnyElement {
    use gpui_omarchy::gpui::{AnimationExt, SharedString, div, prelude::*, px};
    let mut root = div().relative().size(px(size)).flex_shrink_0();
    if discovering {
        root = root.child(
            div().size_full().with_animation(
                "discovery-pixels",
                Animation::new(Duration::from_millis(960))
                    .repeat()
                    .with_max_fps(60.),
                move |mut frame, phase| {
                    let cuts = [0., 10. / 32., 22. / 32., 1.];
                    // Clockwise perimeter, with the center excluded from the chase.
                    let order = [
                        (1, 0),
                        (2, 0),
                        (2, 1),
                        (2, 2),
                        (1, 2),
                        (0, 2),
                        (0, 1),
                        (0, 0),
                    ];
                    for row in 0..3 {
                        for col in 0..3 {
                            let (alpha, lift) = if row == 1 && col == 1 {
                                (1., 0.)
                            } else {
                                let index =
                                    order.iter().position(|cell| *cell == (col, row)).unwrap();
                                ripple((phase - index as f32 / 8.).rem_euclid(1.))
                            };
                            let distance = size * if row != 1 && col != 1 { 0.0375 } else { 0.05 };
                            let dx = (col as f32 - 1.) * distance * lift;
                            let dy = (row as f32 - 1.) * distance * lift;
                            frame = frame.child(
                                div()
                                    .absolute()
                                    .left(px(cuts[col] * size + dx))
                                    .top(px(cuts[row] * size + dy))
                                    .w(px((cuts[col + 1] - cuts[col]) * size))
                                    .h(px((cuts[row + 1] - cuts[row]) * size))
                                    .overflow_hidden()
                                    .opacity(alpha)
                                    .child(
                                        logo(size, color)
                                            .absolute()
                                            .left(px(-cuts[col] * size))
                                            .top(px(-cuts[row] * size))
                                            .size(px(size)),
                                    ),
                            );
                        }
                    }
                    frame
                },
            ),
        );
    } else {
        root = root.child(logo(size, color));
    }
    if revision > 0 {
        root = root.child(
            div()
                .absolute()
                .inset_0()
                .child(logo(size, color))
                .with_animation(
                    SharedString::from(format!("device-confirmed-{revision}")),
                    Animation::new(Duration::from_millis(420)),
                    |overlay, phase| {
                        overlay.opacity(if phase < 0.45 {
                            1.
                        } else {
                            (1. - phase) / 0.55
                        })
                    },
                ),
        );
    }
    root.into_any_element()
}

/// Transparent vector emblem, tinted by the active Omarchy theme.
pub fn logo(size: f32, color: gpui_omarchy::gpui::Hsla) -> gpui_omarchy::gpui::Svg {
    use gpui_omarchy::gpui::{Styled, px, svg};
    svg()
        .data(include_bytes!("../../assets/logo.svg"))
        .size(px(size))
        .text_color(color)
}

// Shared with the website: adjacent segments overlap as one clockwise wave.
fn ripple(phase: f32) -> (f32, f32) {
    let wave = ((1. + (phase * std::f32::consts::TAU).cos()) * 0.5).powi(2);
    (0.25 + 0.75 * wave, 0.65 * wave)
}
