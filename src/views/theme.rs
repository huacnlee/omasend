use gpui_kit::{App, Global, Window, WindowAppearance};
use gpui_omarchy::Theme;
use std::sync::OnceLock;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}
impl Global for ThemeMode {}

impl ThemeMode {
    fn parse(value: &str) -> Self {
        match value.trim() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
    fn value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
    pub fn apply(self, window: &Window, cx: &mut App) {
        match self {
            Self::Light => Theme::flexoki_light().apply(cx),
            Self::Dark => Theme::tokyo_night().apply(cx),
            Self::System => {
                if cfg!(target_os = "linux") {
                    Theme::follow_system(cx);
                } else {
                    match window.appearance() {
                        WindowAppearance::Light | WindowAppearance::VibrantLight => {
                            Theme::flexoki_light().apply(cx)
                        }
                        _ => Theme::tokyo_night().apply(cx),
                    }
                }
            }
        }
    }
    pub fn select(self, window: &Window, cx: &mut App) {
        let selected = if is_omarchy() { Self::System } else { self };
        cx.set_global(selected);
        selected.apply(window, cx);
        if is_omarchy() {
            return;
        }
        if let Some(path) = settings_path() {
            let result = (|| -> anyhow::Result<()> {
                let parent = path.parent().unwrap();
                std::fs::create_dir_all(parent)?;
                let mut file = tempfile::NamedTempFile::new_in(parent)?;
                use std::io::Write;
                file.write_all(self.value().as_bytes())?;
                file.persist(path)?;
                Ok(())
            })();
            if let Err(error) = result {
                tracing::warn!(%error, "Could not save theme preference");
            }
        }
    }
}

fn is_omarchy_os_release(contents: &str) -> bool {
    contents.lines().any(|line| {
        let Some((key, value)) = line.trim().split_once('=') else {
            return false;
        };
        key == "ID" && value.trim().trim_matches(['\'', '"']) == "omarchy"
    })
}

pub fn is_omarchy() -> bool {
    static IS_OMARCHY: OnceLock<bool> = OnceLock::new();
    *IS_OMARCHY.get_or_init(|| {
        cfg!(target_os = "linux")
            && std::fs::read_to_string("/etc/os-release")
                .is_ok_and(|contents| is_omarchy_os_release(&contents))
    })
}

fn mode_for_environment(saved: Option<&str>, omarchy: bool) -> ThemeMode {
    if omarchy {
        ThemeMode::System
    } else {
        saved.map(ThemeMode::parse).unwrap_or_default()
    }
}

fn settings_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|path| path.join("omasend/theme"))
}
pub fn load(cx: &mut App) {
    let saved = settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|value| value.trim().to_owned());
    let mode = mode_for_environment(saved.as_deref(), is_omarchy());
    cx.set_global(mode);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_default_to_system_and_round_trip() {
        for value in ["", "invalid", "system"] {
            assert!(ThemeMode::parse(value) == ThemeMode::System);
        }
        for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
            assert!(ThemeMode::parse(mode.value()) == mode);
        }
    }

    #[test]
    fn omarchy_os_release_is_detected_from_id() {
        assert!(is_omarchy_os_release(
            "NAME=\"Omarchy\"\nID=omarchy\nID_LIKE=arch\n"
        ));
        assert!(is_omarchy_os_release("ID=\"omarchy\"\n"));
        assert!(!is_omarchy_os_release("NAME=Omarchy Theme\nID=arch\n"));
    }

    #[test]
    fn omarchy_ignores_saved_theme_preference() {
        assert!(mode_for_environment(Some("dark"), true) == ThemeMode::System);
        assert!(mode_for_environment(Some("light"), true) == ThemeMode::System);
        assert!(mode_for_environment(Some("dark"), false) == ThemeMode::Dark);
    }
}
