//! Omarchy's current theme is a directory (or symlink), not an OS identity flag.
use crate::Theme;
use gpui::{Hsla, Rgba, rgb};
use gpui_base::ThemeAppearance;
use std::{fmt, fs, path::Path};

#[derive(Debug)]
pub struct ThemeLoadError(String);
impl fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ThemeLoadError {}

#[cfg(not(target_family = "wasm"))]
struct SystemThemeWatcher {
    _task: gpui::Task<()>,
}
#[cfg(not(target_family = "wasm"))]
impl gpui::Global for SystemThemeWatcher {}

pub(crate) fn stop_following(cx: &mut gpui::App) {
    #[cfg(not(target_family = "wasm"))]
    if cx.try_global::<SystemThemeWatcher>().is_some() {
        cx.remove_global::<SystemThemeWatcher>();
    }
    #[cfg(target_family = "wasm")]
    let _ = cx;
}

impl Theme {
    /// Apply the system palette and follow changes until an explicit theme is applied.
    /// Native apps check once per second off the UI thread, following replaced symlinks.
    /// Browsers use the default palette without filesystem monitoring.
    pub fn follow_system(cx: &mut gpui::App) {
        #[cfg(not(target_family = "wasm"))]
        Self::follow_system_from_home(std::env::var_os("HOME").map(Into::into), cx);
        #[cfg(target_family = "wasm")]
        Self::tokyo_night().apply(cx);
    }

    #[cfg(not(target_family = "wasm"))]
    fn follow_system_from_home(home: Option<std::path::PathBuf>, cx: &mut gpui::App) {
        let mut previous = Self::system_from_home(home.as_deref());
        previous.clone().apply(cx);
        let task = cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                let home = home.clone();
                let theme = cx
                    .background_executor()
                    .spawn(async move { Self::system_from_home(home.as_deref()) })
                    .await;
                if theme != previous {
                    previous = theme.clone();
                    cx.update(|cx| theme.apply_palette(cx));
                }
            }
        });
        cx.set_global(SystemThemeWatcher { _task: task });
    }

    /// Read `$HOME/.local/state/omarchy/current/theme/colors.toml`.
    /// Older installations fall back to `.config/omarchy/current` when the
    /// current state directory is absent.
    /// Missing HOME, unreadable files, and invalid palettes fall back atomically.
    pub fn system_or_default() -> Self {
        let home = std::env::var_os("HOME");
        Self::system_from_home(home.as_deref().map(Path::new))
    }

    fn system_from_home(home: Option<&Path>) -> Self {
        home.filter(|home| !home.as_os_str().is_empty())
            .and_then(|home| {
                let current = home.join(".local/state/omarchy/current");
                // An existing but unreadable/broken current theme must not revive
                // a stale legacy theme left over from an upgrade.
                let path = match fs::symlink_metadata(&current) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        home.join(".config/omarchy/current")
                    }
                    _ => current,
                };
                Self::from_current_dir(path).ok()
            })
            .unwrap_or_else(Self::tokyo_night)
    }

    /// Read an Omarchy `current` directory, including optional `theme.name`.
    pub fn from_current_dir(path: impl AsRef<Path>) -> Result<Self, ThemeLoadError> {
        let path = path.as_ref();
        let file = path.join("theme/colors.toml");
        let contents = fs::read_to_string(&file)
            .map_err(|error| ThemeLoadError(format!("{}: {error}", file.display())))?;
        let name = fs::read_to_string(path.join("theme.name"))
            .ok()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Omarchy".into());
        Self::from_colors_toml(name.trim(), &contents)
    }

    /// Supports both ANSI `color0..15` and semantic Omarchy palettes.
    /// Required roles must be present. Optional surfaces derive from this palette.
    pub fn from_colors_toml(name: &str, contents: &str) -> Result<Self, ThemeLoadError> {
        let values: toml::Table =
            toml::from_str(contents).map_err(|error| ThemeLoadError(error.to_string()))?;
        let color = |keys: &[&str]| -> Result<Option<Hsla>, ThemeLoadError> {
            for key in keys {
                if let Some(value) = values.get(*key) {
                    let text = value
                        .as_str()
                        .ok_or_else(|| ThemeLoadError(format!("{key} must be a hex color")))?;
                    let hex = text
                        .strip_prefix('#')
                        .filter(|hex| hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
                        .ok_or_else(|| ThemeLoadError(format!("{key} must use #RRGGBB")))?;
                    return Ok(Some(
                        rgb(u32::from_str_radix(hex, 16)
                            .map_err(|e| ThemeLoadError(e.to_string()))?)
                        .into(),
                    ));
                }
            }
            Ok(None)
        };
        let required = |keys: &[&str]| {
            color(keys)?.ok_or_else(|| ThemeLoadError(format!("missing {}", keys[0])))
        };
        let background = required(&["background"])?;
        let foreground = required(&["foreground"])?;
        let accent = required(&["accent"])?;
        let appearance = match values.get("mode") {
            Some(toml::Value::String(mode)) if mode == "light" => ThemeAppearance::Light,
            Some(toml::Value::String(mode)) if mode == "dark" => ThemeAppearance::Dark,
            Some(_) => return Err(ThemeLoadError("mode must be dark or light".into())),
            None => {
                if luminance(background) > luminance(foreground) {
                    ThemeAppearance::Light
                } else {
                    ThemeAppearance::Dark
                }
            }
        };
        // Derive neutral layers from the user's colors, never another theme.
        let surface =
            color(&["lighter_background"])?.unwrap_or_else(|| mix(background, foreground, 0.05));
        let inset =
            color(&["dark_background"])?.unwrap_or_else(|| mix(background, foreground, 0.08));
        let bright =
            color(&["bright_foreground", "selection_foreground", "cursor"])?.unwrap_or(foreground);
        let secondary = mix(background, foreground, 0.75);
        let black: Hsla = rgb(0x000000).into();
        let white: Hsla = rgb(0xffffff).into();
        let on_accent = if contrast(accent, black) > contrast(accent, white) {
            black
        } else {
            white
        };
        Ok(Self {
            name: name.to_owned().into(),
            appearance,
            background,
            foreground,
            accent,
            on_accent,
            surface,
            inset,
            bright,
            secondary,
            selection: color(&["selection", "selection_background"])?
                .unwrap_or_else(|| mix(background, accent, 0.2)),
            border: color(&["muted", "color8"])?
                .unwrap_or_else(|| mix(background, foreground, 0.25)),
            danger: required(&["red", "color1"])?,
            warning: required(&["yellow", "color3"])?,
            success: required(&["green", "color2"])?,
            font: ".SystemUIFont".into(),
        })
    }
}

fn mix(a: Hsla, b: Hsla, amount: f32) -> Hsla {
    let a: Rgba = a.into();
    let b: Rgba = b.into();
    Rgba {
        r: a.r + (b.r - a.r) * amount,
        g: a.g + (b.g - a.g) * amount,
        b: a.b + (b.b - a.b) * amount,
        a: 1.,
    }
    .into()
}
fn luminance(color: Hsla) -> f32 {
    let color: Rgba = color.into();
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}
fn contrast(a: Hsla, b: Hsla) -> f32 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ANSI: &str = "background = '#fffcf0'\nforeground = '#100f0f'\naccent = '#205ea6'\ncolor1 = '#af3029'\ncolor2 = '#526600'\ncolor3 = '#855b00'\n";
    #[cfg(unix)]
    #[gpui::test]
    fn following_system_tracks_symlink_replacement_edits_and_recovery(
        cx: &mut gpui::TestAppContext,
    ) {
        let home = tempfile::tempdir().unwrap();
        let current = home.path().join(".local/state/omarchy/current");
        let first = home.path().join("first");
        let second = home.path().join("second");
        fs::create_dir_all(&current).unwrap();
        for path in [&first, &second] {
            fs::create_dir_all(path).unwrap();
            fs::write(path.join("colors.toml"), ANSI).unwrap();
        }
        std::os::unix::fs::symlink(&first, current.join("theme")).unwrap();
        cx.update(|cx| {
            gpui_base::init(cx);
            Theme::follow_system_from_home(Some(home.path().into()), cx);
        });
        cx.run_until_parked();
        fs::write(
            second.join("colors.toml"),
            ANSI.replace("#205ea6", "#7aa2f7"),
        )
        .unwrap();
        std::os::unix::fs::symlink(&second, current.join("next")).unwrap();
        fs::rename(current.join("next"), current.join("theme")).unwrap();
        cx.dispatcher
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        cx.update(|cx| assert_eq!(cx.global::<Theme>().accent, Hsla::from(rgb(0x7aa2f7))));

        fs::write(second.join("colors.toml"), "broken").unwrap();
        cx.dispatcher
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        cx.update(|cx| assert_eq!(cx.global::<Theme>().name.as_ref(), "Tokyo Night"));

        fs::write(second.join("colors.toml"), ANSI).unwrap();
        fs::write(current.join("theme.name"), "Recovered").unwrap();
        cx.dispatcher
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(cx.global::<Theme>().name.as_ref(), "Recovered");
            assert_eq!(cx.global::<Theme>().accent, Hsla::from(rgb(0x205ea6)));
            assert_eq!(
                gpui_base::Theme::global(cx).tokens.colors.primary,
                Hsla::from(rgb(0x205ea6))
            );
        });
    }

    #[cfg(not(target_family = "wasm"))]
    #[gpui::test]
    fn explicit_theme_stops_following_and_system_can_be_selected_again(
        cx: &mut gpui::TestAppContext,
    ) {
        let home = tempfile::tempdir().unwrap();
        let current = home.path().join(".local/state/omarchy/current/theme");
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("colors.toml"), ANSI).unwrap();
        cx.update(|cx| {
            gpui_base::init(cx);
            Theme::follow_system_from_home(Some(home.path().into()), cx);
        });
        cx.run_until_parked();
        cx.update(|cx| Theme::tokyo_night().apply(cx));
        fs::write(
            current.join("colors.toml"),
            ANSI.replace("#205ea6", "#ffffff"),
        )
        .unwrap();
        cx.dispatcher
            .advance_clock(std::time::Duration::from_secs(2));
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(cx.global::<Theme>().accent, Hsla::from(rgb(0x7aa2f7)));
            Theme::follow_system_from_home(Some(home.path().into()), cx);
            assert_eq!(cx.global::<Theme>().accent, Hsla::from(rgb(0xffffff)));
        });
    }

    #[test]
    fn system_loader_reads_current_theme_and_falls_back_after_corruption() {
        let home = tempfile::tempdir().unwrap();
        let current = home.path().join(".config/omarchy/current");
        fs::create_dir_all(current.join("theme")).unwrap();
        fs::write(current.join("theme/colors.toml"), ANSI).unwrap();
        fs::write(current.join("theme.name"), "Custom Light\n").unwrap();
        let theme = Theme::system_from_home(Some(home.path()));
        assert_eq!(theme.name.as_ref(), "Custom Light");
        assert_eq!(theme.accent, Hsla::from(rgb(0x205ea6)));
        fs::write(current.join("theme/colors.toml"), "invalid").unwrap();
        assert_eq!(
            Theme::system_from_home(Some(home.path())).tokens(),
            Theme::tokyo_night().tokens()
        );
        fs::remove_file(current.join("theme/colors.toml")).unwrap();
        assert_eq!(
            Theme::system_from_home(Some(home.path())).tokens(),
            Theme::tokyo_night().tokens()
        );
        assert_eq!(
            Theme::system_from_home(None).tokens(),
            Theme::tokyo_night().tokens()
        );
    }

    #[test]
    fn state_directory_takes_precedence_over_legacy_and_does_not_revive_stale_theme() {
        let home = tempfile::tempdir().unwrap();
        for (relative, name) in [
            (".config/omarchy/current", "Legacy"),
            (".local/state/omarchy/current", "Current"),
        ] {
            let path = home.path().join(relative);
            fs::create_dir_all(path.join("theme")).unwrap();
            fs::write(path.join("theme/colors.toml"), ANSI).unwrap();
            fs::write(path.join("theme.name"), name).unwrap();
        }
        assert_eq!(
            Theme::system_from_home(Some(home.path())).name.as_ref(),
            "Current"
        );
        let current = home.path().join(".local/state/omarchy/current");
        fs::write(current.join("theme/colors.toml"), "broken").unwrap();
        assert_eq!(
            Theme::system_from_home(Some(home.path())).name.as_ref(),
            "Tokyo Night"
        );
        fs::remove_dir_all(current).unwrap();
        assert_eq!(
            Theme::system_from_home(Some(home.path())).name.as_ref(),
            "Legacy"
        );
    }

    #[cfg(unix)]
    #[test]
    fn system_loader_follows_current_theme_symlink() {
        let home = tempfile::tempdir().unwrap();
        let current = home.path().join(".config/omarchy/current");
        let target = home.path().join("custom-theme");
        fs::create_dir_all(&current).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("colors.toml"), ANSI).unwrap();
        std::os::unix::fs::symlink(target, current.join("theme")).unwrap();
        assert_eq!(
            Theme::system_from_home(Some(home.path())).appearance,
            ThemeAppearance::Light
        );
    }

    #[test]
    fn legacy_light_palette_infers_mode_and_derives_its_own_surfaces() {
        let theme = Theme::from_colors_toml("Test", ANSI).unwrap();
        assert_eq!(theme.appearance, ThemeAppearance::Light);
        assert!(luminance(theme.surface) > 0.7);
        assert!(contrast(theme.accent, theme.on_accent) >= 4.5);
    }
    #[test]
    fn semantic_palette_preserves_explicit_roles() {
        let source = "mode = 'dark'\nbackground = '#1a1b26'\nforeground = '#a9b1d6'\naccent = '#7aa2f7'\nred = '#f7768e'\ngreen = '#9ece6a'\nyellow = '#e0af68'\nlighter_background = '#24283b'\nselection = '#292e42'\n";
        let theme = Theme::from_colors_toml("Custom", source).unwrap();
        assert_eq!(theme.surface, Hsla::from(rgb(0x24283b)));
        assert_eq!(theme.selection, Hsla::from(rgb(0x292e42)));
    }
    #[test]
    fn invalid_palettes_are_rejected_without_partial_defaults() {
        for source in [
            "",
            "background = 42",
            "broken toml",
            &ANSI.replace("#205ea6", "#nope!!"),
            &format!("{ANSI}mode = 'automatic'\n"),
        ] {
            assert!(Theme::from_colors_toml("Bad", source).is_err());
        }
    }
    #[test]
    fn missing_current_directory_is_an_error() {
        assert!(Theme::from_current_dir("/nonexistent-omarchy-theme-test/current").is_err());
    }
    #[test]
    fn bundled_themes_have_readable_text_and_statuses() {
        for theme in [Theme::tokyo_night(), Theme::flexoki_light()] {
            for surface in [theme.background, theme.surface] {
                for ink in [
                    theme.foreground,
                    theme.secondary,
                    theme.accent,
                    theme.danger,
                    theme.warning,
                    theme.success,
                ] {
                    assert!(contrast(surface, ink) >= 4.5, "{} {:?}", theme.name, ink);
                }
            }
            assert!(contrast(theme.accent, theme.on_accent) >= 4.5);
        }
    }
}
