//! Theme for the Rust UI: nsh-omarchy draws every component and owns the palette.
//! On Omarchy the app follows the system theme; elsewhere the user picks
//! System (follows the OS appearance), Light (Flexoki Light) or Dark (Tokyo Night).
use nsh_core::{platform::Appearance, prelude::*};
use nsh_omarchy::{self as om, tokens};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Mode {
    #[default]
    System,
    Light,
    Dark,
}
impl Mode {
    pub fn parse(value: &str) -> Self {
        match value.trim() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
    pub fn value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "theme.system",
            Self::Light => "theme.light",
            Self::Dark => "theme.dark",
        }
    }
}

/// Type size and family from the nsh-omarchy tokens.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Metrics {
    pub font_size: f32,
    pub family: SharedString,
}
impl Metrics {
    fn current() -> Self {
        Self {
            font_size: if cfg!(any(target_os = "ios", target_os = "android")) {
                15.0
            } else {
                tokens::TEXT_MD
            },
            family: tokens::font_mono(),
        }
    }
}

pub(crate) fn root() -> nsh_core::element::Root {
    nsh_core::element::Root::of::<super::App>()
}

#[derive(Clone)]
pub(super) struct Controller {
    pub mode: Signal<Mode>,
    omarchy: bool,
    persist: bool,
}
pub(super) fn install() -> Controller {
    let omarchy = cfg!(target_os = "linux")
        && std::fs::read_to_string("/etc/os-release").is_ok_and(|s| omarchy_os_release(&s));
    let mode = if omarchy {
        Mode::System
    } else {
        super::preferences::read("theme")
            .map(|s| Mode::parse(&s))
            .unwrap_or_default()
    };
    install_with(mode, omarchy, true)
}

fn install_with(mode: Mode, omarchy: bool, persist: bool) -> Controller {
    let controller = Controller {
        mode: signal(mode),
        omarchy,
        persist,
    };
    provide_context(controller.clone());
    controller.apply(mode);
    controller
}
#[cfg(all(test, feature = "ui-tests"))]
pub(super) fn install_for_test() -> Controller {
    install_with(Mode::System, false, false)
}

impl Controller {
    pub fn select(&self, mode: Mode) -> anyhow::Result<()> {
        let mode = if self.omarchy { Mode::System } else { mode };
        self.apply(mode);
        if self.persist && !self.omarchy {
            super::preferences::write("theme", mode.value())
        } else {
            Ok(())
        }
    }
    pub fn reload(&self) {
        self.apply(self.mode.get_untracked());
    }
    fn apply(&self, mode: Mode) {
        self.mode.set(mode);
        match mode {
            Mode::System if self.omarchy => om::init(),
            // Without an Omarchy theme, System uses the default palette and
            // lets the window follow the OS appearance.
            Mode::System => {
                apply_builtin(false);
                set_color_scheme(Appearance::System);
            }
            Mode::Light => apply_builtin(true),
            Mode::Dark => apply_builtin(false),
        }
    }
}

fn apply_builtin(light: bool) {
    if light {
        om::Theme::flexoki_light().apply()
    } else {
        om::Theme::tokyo_night().apply()
    }
}

/// Only an actual Omarchy installation may consume Omarchy shell preferences.
pub(super) fn is_omarchy() -> bool {
    use_context::<Controller>().is_some_and(|c| c.omarchy)
}

/// Mobile ports keep the terminal chrome without reading Linux preferences.
pub(super) fn is_omarchy_style() -> bool {
    is_omarchy() || cfg!(any(target_os = "ios", target_os = "android"))
}

pub(super) fn is_terminal() -> bool {
    idiom() == nsh_core::platform::Idiom::Terminal
}

pub(super) fn panel_background() -> AdaptiveColor {
    if is_omarchy_style() {
        tokens::background()
    } else {
        tokens::surface()
    }
}

pub(super) fn divider() -> AdaptiveColor {
    tokens::divider()
}

pub(super) fn font_size() -> f32 {
    Metrics::current().font_size
}

fn omarchy_os_release(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.split_once('=').is_some_and(|(key, value)| {
            key.trim() == "ID" && value.trim().trim_matches(['\'', '"']) == "omarchy"
        })
    })
}

pub(super) fn metrics() -> Metrics {
    Metrics::current()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_round_trip_and_invalid_preferences_follow_system() {
        for mode in [Mode::System, Mode::Light, Mode::Dark] {
            assert_eq!(Mode::parse(mode.value()), mode);
        }
        assert_eq!(Mode::parse("unknown"), Mode::System);
    }

    #[test]
    fn only_omarchy_os_id_selects_the_system_style() {
        assert!(omarchy_os_release("NAME=Omarchy\nID=omarchy\n"));
        assert!(omarchy_os_release("ID=\"omarchy\"\n"));
        for release in [
            "ID=arch\nID_LIKE=omarchy",
            "NAME=Omarchy\nID=ubuntu",
            "ID=omarchy-test",
        ] {
            assert!(!omarchy_os_release(release));
        }
    }
}
