//! Network diagnostics: runs `omasend::netcheck` against the running node; the
//! interface shows the Markdown report the user copies into an AI assistant.
use super::Session;
use nsh_core::prelude::*;
use omasend::netcheck::{self, Report};

/// Root-owned, so a run keeps going while its sheet is closed.
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
}

fn frontend() -> &'static str {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        "mobile"
    } else if crate::terminal_binary() {
        "terminal"
    } else {
        "desktop"
    }
}
