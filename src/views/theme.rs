use gpui_omarchy::{
    Theme,
    gpui::{App, Global, Window, WindowAppearance},
};

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
        cx.set_global(self);
        self.apply(window, cx);
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
fn settings_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|path| path.join("omasend/theme"))
}
pub fn load(cx: &mut App) {
    let mode = settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|value| ThemeMode::parse(&value))
        .unwrap_or_default();
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
}
