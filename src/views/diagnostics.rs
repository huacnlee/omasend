//! Network diagnostics surface: runs `omasend::netcheck` against the running
//! node and shows a Markdown report the user copies into an AI assistant.
use super::{appearance::theme, control, editor, session::Session};
use nsh_core::prelude::*;
use omasend::netcheck::{self, Report};

/// Root-owned, so a run keeps going while the surface is closed.
#[derive(Clone, Copy)]
pub(crate) struct Diagnostics {
    pub report: Signal<Option<Report>>,
    pub running: Signal<bool>,
    pub step: Signal<Option<&'static str>>,
    pub peer: Signal<String>,
    pub invalid_peer: Signal<bool>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            report: signal_with(None, SignalEquals::Never),
            running: signal(false),
            step: signal(None),
            peer: signal(String::new()),
            invalid_peer: signal(false),
        }
    }
}

impl Session {
    pub fn run_diagnostics(&self) {
        let state = self.diagnostics;
        if state.running.get_untracked() {
            return;
        }
        let peer = match netcheck::parse_peer(&state.peer.get_untracked()) {
            Ok(peer) => peer,
            Err(_) => {
                state.invalid_peer.set(true);
                return;
            }
        };
        state.invalid_peer.set(false);
        state.running.set(true);
        let handle = self.handle.get_untracked();
        self.task(async move {
            let node = match handle {
                Some(handle) => handle.probe().await.map_err(|error| error.to_string()),
                None => Err("the network service has not started".to_owned()),
            };
            let report = netcheck::run(
                node,
                netcheck::Options {
                    peer,
                    frontend: frontend(),
                    ..Default::default()
                },
                move |step| state.step.set(Some(step.key())),
            )
            .await;
            state.report.set(Some(report));
            state.step.set(None);
            state.running.set(false);
        });
    }

    fn diagnostics_markdown(&self) -> Option<String> {
        let language = self.language.get();
        self.diagnostics
            .report
            .get()
            .map(|report| report.to_markdown(language))
    }
}

fn frontend() -> &'static str {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        "mobile"
    } else if super::appearance::is_terminal() {
        "terminal"
    } else {
        "desktop"
    }
}

pub(super) fn surface(session: Session) -> impl IntoElement {
    let state = session.diagnostics;
    let language = session.language;
    let notice = session.notice;
    let run = session.clone();
    let copy = session.clone();
    let markdown = session.clone();
    view()
        .gap_3()
        .child(
            text(session.translated("diagnostics.intro"))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            editor()
                .value(move || state.peer.get())
                .on_change(move |value: &str| {
                    state.peer.set(value.to_owned());
                    state.invalid_peer.set(false);
                })
                .on_submit({
                    let run = run.clone();
                    move |_: &str| run.run_diagnostics()
                })
                .placeholder(move || language.get().text("diagnostics.peer_placeholder"))
                .label(move || language.get().text("diagnostics.peer_label"))
                .test_id("diagnostics-peer"),
        )
        .child(when(
            move || state.invalid_peer.get(),
            move || {
                text(move || language.get().text("diagnostics.peer_invalid"))
                    .text_color(move || theme().color("destructive").unwrap())
            },
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(control(
                    move || {
                        SharedString::from(language.get().text(if state.running.get() {
                            "diagnostics.running"
                        } else if state.report.get().is_some() {
                            "diagnostics.run_again"
                        } else {
                            "diagnostics.run"
                        }))
                    },
                    move || state.running.get(),
                    move || run.run_diagnostics(),
                ))
                .child(control(
                    session.translated("action.copy"),
                    move || state.running.get() || state.report.get().is_none(),
                    move || {
                        if let Some(markdown) = copy.diagnostics_markdown() {
                            set_clipboard(markdown);
                            copy.notify("logs.copied");
                        }
                    },
                ))
                .child(
                    text(move || {
                        state
                            .step
                            .get()
                            .map(|key| language.get().text(key))
                            .or_else(|| notice.get().map(|key| language.get().text(key)))
                            .unwrap_or_default()
                    })
                    .flex_1()
                    .text_color(move || {
                        if state.step.get().is_some() {
                            theme().muted_foreground
                        } else {
                            theme().color("success.background").unwrap()
                        }
                    }),
                ),
        )
        .child(
            editor()
                .value(move || markdown.diagnostics_markdown().unwrap_or_default())
                .placeholder(move || language.get().text("diagnostics.empty"))
                .readonly(true)
                .multiline(true)
                .h(move || px((window().height.0 - 320.0).clamp(100.0, 420.0)))
                .label(move || language.get().text("diagnostics.report_label"))
                .test_id("diagnostics-report"),
        )
}
