//! The existing vector emblem, with short native discovery/confirmation strokes.
use super::{appearance::theme, session::Session};
use nsh_core::{
    canvas::{Path, svg_path},
    ui::prelude::*,
};
use std::{sync::OnceLock, time::Duration};

fn paths() -> &'static [Path] {
    static PATHS: OnceLock<Vec<Path>> = OnceLock::new();
    PATHS.get_or_init(|| {
        include_str!("../../assets/logo.svg")
            .split(" d=\"")
            .skip(1)
            .flat_map(|part| {
                part.split('"')
                    .next()
                    .unwrap()
                    .split('M')
                    .filter(|part| !part.is_empty())
                    .map(|part| svg_path(&format!("M{part}")))
                    .collect::<Vec<_>>()
            })
            .collect()
    })
}

fn burst(levels: &[NativeValue], active: Memo<bool>, epoch: Signal<u64>) {
    if !active.get_untracked() || reduce_motion() {
        return;
    }
    let current = epoch.get_untracked();
    for (turn, index) in [0, 4, 1, 5, 3, 6, 2, 7].into_iter().enumerate() {
        let level = levels[index].clone();
        set_timeout(Duration::from_millis(turn as u64 * 55), move || {
            if epoch.get_untracked() != current || !active.get_untracked() || reduce_motion() {
                return;
            }
            level.timing(1.0, 50.0, Easing::Linear);
            set_timeout(Duration::from_millis(85), move || {
                if epoch.get_untracked() == current && active.get_untracked() && !reduce_motion() {
                    level.timing(0.6, 130.0, Easing::Linear);
                }
            });
        });
    }
}

pub(super) fn emblem(session: Session, size: f32) -> impl IntoElement {
    let state = session.clone();
    let active = computed(move || {
        state.revision.get();
        state.state.borrow().discovering
    });
    let revision = computed(move || {
        session.revision.get();
        session.state.borrow().discovery_revision
    });
    let levels = (0..9).map(|_| native_value(1.0)).collect::<Vec<_>>();
    let changing = levels.clone();
    let epoch = signal(0_u64);
    effect(move || {
        let running = active.get() && !reduce_motion();
        epoch.update(|value| *value += 1);
        for (index, level) in changing.iter().enumerate() {
            level.set(if running && index != 8 { 0.6 } else { 1.0 });
        }
        if running {
            burst(&changing, active, epoch);
        }
    });
    let clock = levels.clone();
    effect(move || {
        if active.get() && !reduce_motion() {
            let clock = clock.clone();
            set_interval(Duration::from_millis(2250), move || {
                burst(&clock, active, epoch)
            });
        }
    });
    let confirmed = native_value(0.0);
    let confirmation = confirmed.clone();
    effect(move || {
        if revision.get() > 0 && !reduce_motion() {
            confirmation.set(1.0);
            confirmation.timing(0.0, 420.0, Easing::Linear);
        } else {
            confirmation.set(0.0);
        }
    });
    let mut logo = view().relative().size(px(size));
    for (index, path) in paths().iter().cloned().enumerate() {
        let level = levels[index].clone();
        logo = logo.child(
            canvas(move || {
                Painter::new()
                    .scale(size / 32.0, size / 32.0)
                    .fill_color(color(theme().link))
                    .fill_path(&path, nsh_core::canvas::FillRule::NonZero)
            })
            .absolute()
            .size(px(size))
            .bind_opacity(&level),
        );
    }
    logo.child(
        canvas(move || {
            let mut painter = Painter::new()
                .scale(size / 32.0, size / 32.0)
                .fill_color(color(theme().link));
            for path in paths() {
                painter = painter.fill_path(path, nsh_core::canvas::FillRule::NonZero);
            }
            painter
        })
        .absolute()
        .size(px(size))
        .bind_opacity(&confirmed),
    )
}
