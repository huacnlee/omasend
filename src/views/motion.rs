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

/// Animate crops of the real app icon so the emblem never drifts from its asset.
/// The center remains steady; the surrounding eight regions advance in steps.
pub fn discovery_logo(
    logo: std::sync::Arc<gpui_omarchy::gpui::Image>,
    discovering: bool,
    revision: u64,
) -> gpui_omarchy::gpui::AnyElement {
    use gpui_omarchy::gpui::{AnimationExt, SharedString, div, img, prelude::*, px};
    let size = 24.;
    let mut root = div().relative().size(px(size)).flex_shrink_0();
    if discovering {
        let animated_logo = logo.clone();
        root = root.child(
            div().size_full().with_animation(
                "discovery-pixels",
                Animation::new(Duration::from_millis(960))
                    .repeat()
                    .with_max_fps(16.),
                move |mut frame, phase| {
                    let step = ((phase * 8.) as usize).min(7);
                    let cuts = [0., 0.35, 0.65, 1.];
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
                            let alpha = if row == 1 && col == 1 {
                                1.
                            } else {
                                let index =
                                    order.iter().position(|cell| *cell == (col, row)).unwrap();
                                match (step + 8 - index) % 8 {
                                    0 => 1.,
                                    1 => 0.65,
                                    _ => 0.25,
                                }
                            };
                            frame = frame.child(
                                div()
                                    .absolute()
                                    .left(px(cuts[col] * size))
                                    .top(px(cuts[row] * size))
                                    .w(px((cuts[col + 1] - cuts[col]) * size))
                                    .h(px((cuts[row + 1] - cuts[row]) * size))
                                    .overflow_hidden()
                                    .opacity(alpha)
                                    .child(
                                        img(animated_logo.clone())
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
        root = root.child(img(logo.clone()).size(px(size)));
    }
    if revision > 0 {
        root = root.child(
            div()
                .absolute()
                .inset_0()
                .child(img(logo).size(px(size)))
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
