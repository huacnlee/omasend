//! Omarchy palette loading and atomic Rust-component theme changes.
//! Native inputs explicitly bind the same palette through `editor()`.
use nsh_base::Theme;
use nsh_core::{platform::Appearance, ui::prelude::*};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, OnceLock},
};

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

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Metrics {
    pub font_size: f32,
    pub family: String,
    pub spacing_scale: f32,
}
impl Default for Metrics {
    fn default() -> Self {
        Self {
            font_size: if cfg!(target_os = "ios") { 15.0 } else { 12.0 },
            family: if cfg!(any(target_os = "ios", target_os = "macos")) {
                "Menlo"
            } else {
                "monospace"
            }
            .into(),
            spacing_scale: 1.0,
        }
    }
}
impl Metrics {
    pub fn space(&self, value: f32) -> f32 {
        if value <= 0.0 {
            0.0
        } else {
            (value * self.spacing_scale).round().max(1.0)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Palette {
    light: bool,
    background: u32,
    foreground: u32,
    accent: u32,
    surface: u32,
    inset: u32,
    secondary: u32,
    bright: u32,
    border: u32,
    selection: u32,
    danger: u32,
    warning: u32,
    success: u32,
    on_accent: u32,
}
impl Palette {
    fn builtin(light: bool) -> Self {
        let (
            background,
            foreground,
            accent,
            surface,
            inset,
            secondary,
            border,
            selection,
            danger,
            warning,
            success,
        ) = if light {
            (
                0xfffcf0, 0x100f0f, 0x205ea6, 0xf2f0e5, 0xe6e4d9, 0x575653, 0xb7b5ac, 0xdad8ce,
                0xaf3029, 0x855b00, 0x526600,
            )
        } else {
            (
                0x1a1b26, 0xc0caf5, 0x7aa2f7, 0x24283b, 0x13141c, 0xa9b1d6, 0x414868, 0x292e42,
                0xf7768e, 0xe0af68, 0x9ece6a,
            )
        };
        Self {
            light,
            background,
            foreground,
            accent,
            surface,
            inset,
            secondary,
            bright: foreground,
            border,
            selection,
            danger,
            warning,
            success,
            on_accent: ink(accent),
        }
    }

    // Standalone Omasend borrows Disktree's neutral surfaces and green accent.
    // Its light appearance uses the same roles instead of an Omarchy palette.
    fn standalone(light: bool) -> Self {
        let (
            background,
            foreground,
            accent,
            surface,
            inset,
            secondary,
            border,
            selection,
            danger,
            warning,
            success,
        ) = if light {
            (
                0xf7f7f8, 0x18181b, 0x157f43, 0xececef, 0xe2e2e7, 0x62626d, 0xc9c9d0, 0xd4eddf,
                0xb42335, 0x8a5800, 0x157f43,
            )
        } else {
            (
                0x070708, 0xededf0, 0x65f193, 0x101012, 0x161619, 0x85858f, 0x29292d, 0x203329,
                0xff7d88, 0xe6b968, 0x65f193,
            )
        };
        Self {
            light,
            background,
            foreground,
            accent,
            surface,
            inset,
            secondary,
            bright: foreground,
            border,
            selection,
            danger,
            warning,
            success,
            on_accent: ink(accent),
        }
    }

    fn parse(contents: &str) -> Option<Self> {
        let values = contents.parse::<toml::Table>().ok()?;
        // Reject malformed declared colors rather than combining unrelated palettes.
        let role_names = [
            "background",
            "foreground",
            "accent",
            "red",
            "yellow",
            "green",
            "lighter_background",
            "dark_background",
            "bright_foreground",
            "selection_foreground",
            "cursor",
            "muted",
            "selection",
            "selection_background",
        ];
        for (key, value) in &values {
            if key == "mode" {
                if !matches!(value.as_str(), Some("light" | "dark")) {
                    return None;
                }
                continue;
            }
            if (role_names.contains(&key.as_str())
                || key
                    .strip_prefix("color")
                    .is_some_and(|index| index.parse::<u8>().is_ok_and(|n| n < 16)))
                && value.as_str().and_then(hex).is_none()
            {
                return None;
            }
        }
        let get = |keys: &[&str]| -> Option<u32> {
            keys.iter()
                .find_map(|key| values.get(*key).and_then(toml::Value::as_str).and_then(hex))
        };
        let background = get(&["background"])?;
        let foreground = get(&["foreground"])?;
        let accent = get(&["accent", "color4"])?;
        let danger = get(&["red", "color1"])?;
        let warning = get(&["yellow", "color3"])?;
        let success = get(&["green", "color2"])?;
        let light = values
            .get("mode")
            .and_then(toml::Value::as_str)
            .map(|mode| mode == "light")
            .unwrap_or_else(|| luminance(background) > luminance(foreground));
        Some(Self {
            light,
            background,
            foreground,
            accent,
            surface: get(&["lighter_background"])
                .unwrap_or_else(|| mix(background, foreground, 0.05)),
            inset: get(&["dark_background"]).unwrap_or_else(|| mix(background, foreground, 0.08)),
            secondary: mix(background, foreground, 0.75),
            bright: get(&["bright_foreground", "selection_foreground", "cursor"])
                .unwrap_or(foreground),
            border: get(&["muted", "color8"]).unwrap_or_else(|| mix(background, foreground, 0.25)),
            selection: get(&["selection", "selection_background"])
                .unwrap_or_else(|| mix(background, accent, 0.2)),
            danger: legible(danger, background, foreground),
            warning: legible(warning, background, foreground),
            success: legible(success, background, foreground),
            on_accent: ink(accent),
        })
    }
}

fn hex(value: &str) -> Option<u32> {
    (value.len() == 7 && value.starts_with('#'))
        .then(|| u32::from_str_radix(&value[1..], 16).ok())
        .flatten()
}
fn mix(from: u32, to: u32, fraction: f32) -> u32 {
    [16, 8, 0].into_iter().fold(0, |result, shift| {
        let a = ((from >> shift) & 255) as f32;
        let b = ((to >> shift) & 255) as f32;
        result | (((a + (b - a) * fraction).round() as u32) << shift)
    })
}
fn luminance(color: u32) -> f64 {
    let linear = |shift: u32| {
        let value = ((color >> shift) & 255) as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
}
fn contrast(a: u32, b: u32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn ink(background: u32) -> u32 {
    if contrast(0, background) > contrast(0xffffff, background) {
        0
    } else {
        0xffffff
    }
}
fn legible(color: u32, background: u32, foreground: u32) -> u32 {
    (0..=20)
        .map(|step| mix(color, foreground, step as f32 / 20.0))
        .find(|color| contrast(*color, background) >= 4.5)
        .unwrap_or(foreground)
}

fn theme_file(home: &Path) -> PathBuf {
    let current = home.join(".local/state/omarchy/current");
    // symlink_metadata also recognizes a dangling current symlink. Its stale
    // legacy predecessor must not silently become active again.
    if std::fs::symlink_metadata(&current).is_ok() {
        current.join("theme/colors.toml")
    } else {
        home.join(".config/omarchy/current/theme/colors.toml")
    }
}
fn load_palette(home: Option<&Path>) -> Option<Palette> {
    home.and_then(|home| std::fs::read_to_string(theme_file(home)).ok())
        .and_then(|text| Palette::parse(&text))
}

fn metrics_at(home: Option<&Path>) -> Metrics {
    let mut metrics = Metrics::default();
    let Some(home) = home else {
        return metrics;
    };
    let mut shell = toml::Table::new();
    for path in [
        theme_file(home).with_file_name("shell.toml"),
        home.join(".config/omarchy/shell.toml"),
    ] {
        if let Some(table) = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| s.parse::<toml::Table>().ok())
        {
            for (key, value) in table {
                if let (Some(existing), Some(incoming)) = (
                    shell.get_mut(&key).and_then(toml::Value::as_table_mut),
                    value.as_table(),
                ) {
                    existing.extend(incoming.clone());
                } else {
                    shell.insert(key, value);
                }
            }
        }
    }
    if let Some(font) = shell.get("font").and_then(toml::Value::as_table) {
        if let Some(size) = number(font.get("base-size")) {
            metrics.font_size = size.max(1.0);
        }
        if let Some(family) = font
            .get("family")
            .and_then(toml::Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            metrics.family = family.to_owned();
        }
    }
    let spacing = shell.get("spacing").and_then(toml::Value::as_table);
    let scale = spacing.and_then(|s| number(s.get("scale"))).unwrap_or(1.0);
    let with_font = spacing
        .and_then(|s| s.get("scale-with-font"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    metrics.spacing_scale = scale
        * if with_font {
            metrics.font_size / 12.0
        } else {
            1.0
        };
    metrics
}
fn number(value: Option<&toml::Value>) -> Option<f32> {
    value
        .and_then(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|n| n as f64))
        })
        .map(|n| n as f32)
        .filter(|n| n.is_finite() && *n > 0.0)
}

#[derive(Clone)]
struct Startup {
    omarchy: bool,
    system: Option<Palette>,
    metrics: Metrics,
    mode: Mode,
}
impl Startup {
    fn load() -> Self {
        let home = dirs::home_dir();
        let saved = super::preferences::read("theme");
        let omarchy = cfg!(target_os = "linux")
            && std::fs::read_to_string("/etc/os-release").is_ok_and(|s| omarchy_os_release(&s));
        Self::for_environment(home.as_deref(), saved.as_deref(), omarchy)
    }
    fn for_environment(home: Option<&Path>, saved: Option<&str>, omarchy: bool) -> Self {
        Self {
            omarchy,
            system: if omarchy { load_palette(home) } else { None },
            metrics: if omarchy {
                metrics_at(home)
            } else {
                Metrics::default()
            },
            mode: if omarchy {
                Mode::System
            } else {
                saved.map(Mode::parse).unwrap_or_default()
            },
        }
    }
    fn theme(&self, mode: Mode) -> Theme {
        if mode == Mode::System
            && let Some(palette) = &self.system
        {
            return make_theme(palette, palette, self.metrics.font_size);
        }
        let palette = if self.omarchy {
            Palette::builtin
        } else {
            Palette::standalone
        };
        make_theme(&palette(true), &palette(false), self.metrics.font_size)
    }
    fn appearance(&self, mode: Mode) -> Appearance {
        match mode {
            Mode::System => self
                .system
                .as_ref()
                .map(|p| {
                    if p.light {
                        Appearance::Light
                    } else {
                        Appearance::Dark
                    }
                })
                .unwrap_or(Appearance::System),
            Mode::Light => Appearance::Light,
            Mode::Dark => Appearance::Dark,
        }
    }
}
fn startup() -> &'static Startup {
    static STARTUP: OnceLock<Startup> = OnceLock::new();
    STARTUP.get_or_init(Startup::load)
}

pub(crate) fn root() -> nsh_core::ui::Root {
    let initial = startup();
    nsh_core::ui::Root::of::<super::App>()
        .tokens(initial.theme(initial.mode).into_tokens())
        .appearance(initial.appearance(initial.mode))
}

#[derive(Clone)]
pub(super) struct Controller {
    pub mode: Signal<Mode>,
    palette: Signal<Arc<Theme>>,
    metrics: Signal<Metrics>,
    source: Rc<RefCell<Startup>>,
    persist: bool,
}
pub(super) fn install() -> Controller {
    install_with(startup().clone(), true)
}

fn install_with(initial: Startup, persist: bool) -> Controller {
    let controller = Controller {
        mode: signal(initial.mode),
        palette: signal(Arc::new(initial.theme(initial.mode))),
        metrics: signal(initial.metrics.clone()),
        source: Rc::new(RefCell::new(initial)),
        persist,
    };
    provide_context(controller.clone());
    set_color_scheme(
        controller
            .source
            .borrow()
            .appearance(controller.mode.get_untracked()),
    );
    controller
}
#[cfg(all(test, feature = "ui-tests"))]
pub(super) fn install_for_test() -> Controller {
    install_with(
        Startup {
            omarchy: false,
            system: None,
            metrics: Metrics::default(),
            mode: Mode::System,
        },
        false,
    )
}

impl Controller {
    pub fn select(&self, mode: Mode) -> anyhow::Result<()> {
        let mode = if self.source.borrow().omarchy {
            Mode::System
        } else {
            mode
        };
        self.apply(mode);
        if self.persist && !self.source.borrow().omarchy {
            super::preferences::write("theme", mode.value())
        } else {
            Ok(())
        }
    }
    pub fn reload(&self) {
        *self.source.borrow_mut() = Startup::load();
        self.apply(self.mode.get_untracked());
    }
    fn apply(&self, mode: Mode) {
        let source = self.source.borrow();
        batch(|| {
            self.mode.set(mode);
            self.palette.set(Arc::new(source.theme(mode)));
            self.metrics.set(source.metrics.clone());
            set_color_scheme(source.appearance(mode));
        });
    }
}
pub(super) fn theme() -> Arc<Theme> {
    use_context::<Controller>()
        .map(|controller| controller.palette.get())
        .unwrap_or_else(|| Arc::new(nsh_base::theme()))
}
/// Only an actual Omarchy installation may consume Omarchy shell preferences.
pub(super) fn is_omarchy() -> bool {
    use_context::<Controller>()
        .map(|c| c.source.borrow().omarchy)
        .unwrap_or(false)
}

/// The iPhone port keeps the terminal chrome without reading Linux preferences.
pub(super) fn is_omarchy_style() -> bool {
    is_omarchy() || cfg!(target_os = "ios")
}

pub(super) fn panel_background() -> AdaptiveColor {
    if is_omarchy_style() {
        theme().background()
    } else {
        theme().color("group_box.background").unwrap()
    }
}

pub(super) fn divider() -> AdaptiveColor {
    if is_omarchy_style() {
        theme().foreground().opacity(0.12)
    } else {
        theme().border()
    }
}

fn omarchy_os_release(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.split_once('=').is_some_and(|(key, value)| {
            key.trim() == "ID" && value.trim().trim_matches(['\'', '"']) == "omarchy"
        })
    })
}

pub(super) fn metrics() -> Metrics {
    use_context::<Controller>()
        .map(|controller| controller.metrics.get())
        .unwrap_or_default()
}

fn make_theme(light: &Palette, dark: &Palette, font_size: f32) -> Theme {
    let a = |l: u32, d: u32| AdaptiveColor::new(rgb(l), rgb(d));
    let mut theme = Theme::default();
    theme.set_color("accent.background", a(light.selection, dark.selection));
    theme.set_color("accent.foreground", a(light.foreground, dark.foreground));
    theme.set_color("background", a(light.background, dark.background));
    theme.set_color("foreground", a(light.foreground, dark.foreground));
    theme.set_color("border", a(light.border, dark.border));
    theme.set_color("caret", a(light.accent, dark.accent));
    theme.set_color("danger.background", a(light.danger, dark.danger));
    theme.set_color("danger.active.background", a(light.danger, dark.danger));
    theme.set_color("danger.foreground", a(ink(light.danger), ink(dark.danger)));
    theme.set_color("group_box.background", a(light.surface, dark.surface));
    theme.set_color("group_box.foreground", a(light.foreground, dark.foreground));
    theme.set_color("info.background", a(light.accent, dark.accent));
    theme.set_color("info.active.background", a(light.accent, dark.accent));
    theme.set_color("info.foreground", a(light.on_accent, dark.on_accent));
    theme.set_color("input.border", a(light.border, dark.border));
    theme.set_color("link", a(light.accent, dark.accent));
    theme.set_color("link.active", a(light.bright, dark.bright));
    theme.set_color("list.background", a(light.background, dark.background));
    theme.set_color("list.active.background", a(light.selection, dark.selection));
    theme.set_color("list.active.border", a(light.accent, dark.accent));
    theme.set_color("list.even.background", a(light.surface, dark.surface));
    theme.set_color("list.head.background", a(light.surface, dark.surface));
    theme.set_color("muted.background", a(light.inset, dark.inset));
    theme.set_color("muted.foreground", a(light.secondary, dark.secondary));
    theme.set_color("overlay", AdaptiveColor::from(rgb(0)).opacity(0.6));
    theme.set_color("popover.background", a(light.background, dark.background));
    theme.set_color("popover.foreground", a(light.foreground, dark.foreground));
    theme.set_color("primary.background", a(light.accent, dark.accent));
    theme.set_color("primary.active.background", a(light.accent, dark.accent));
    theme.set_color("primary.foreground", a(light.on_accent, dark.on_accent));
    theme.set_color("progress.bar.background", a(light.accent, dark.accent));
    theme.set_color("ring", a(light.foreground, dark.foreground));
    theme.set_color("secondary.background", a(light.surface, dark.surface));
    theme.set_color(
        "secondary.active.background",
        a(light.selection, dark.selection),
    );
    theme.set_color("secondary.foreground", a(light.foreground, dark.foreground));
    theme.set_color("selection.background", a(light.selection, dark.selection));
    theme.set_color("skeleton.background", a(light.inset, dark.inset));
    theme.set_color("slider.background", a(light.accent, dark.accent));
    theme.set_color(
        "slider.thumb.background",
        a(light.foreground, dark.foreground),
    );
    theme.set_color("success.background", a(light.success, dark.success));
    theme.set_color("success.active.background", a(light.success, dark.success));
    theme.set_color(
        "success.foreground",
        a(ink(light.success), ink(dark.success)),
    );
    theme.set_color("switch.background", a(light.inset, dark.inset));
    theme.set_color(
        "switch.thumb.background",
        a(light.foreground, dark.foreground),
    );
    theme.set_color("tab.background", a(light.background, dark.background));
    theme.set_color("tab.active.background", a(light.selection, dark.selection));
    theme.set_color(
        "tab.active.foreground",
        a(light.foreground, dark.foreground),
    );
    theme.set_color("tab_bar.background", a(light.background, dark.background));
    theme.set_color(
        "tab_bar.segmented.background",
        a(light.surface, dark.surface),
    );
    theme.set_color("tab.foreground", a(light.secondary, dark.secondary));
    theme.set_color("warning.background", a(light.warning, dark.warning));
    theme.set_color("warning.active.background", a(light.warning, dark.warning));
    theme.set_color(
        "warning.foreground",
        a(ink(light.warning), ink(dark.warning)),
    );
    theme.set_number("radius", 0.0);
    theme.set_number("radius.lg", 0.0);
    theme.set_number("font.size", font_size);
    theme
}

#[cfg(test)]
mod tests {
    use super::*;
    const SEMANTIC: &str = "background = '#101820'\nforeground = '#e0e8f0'\naccent = '#70c0d0'\nred = '#ed7070'\nyellow = '#dfcf70'\ngreen = '#80c070'\n";
    #[test]
    fn semantic_and_ansi_palettes_derive_roles_from_the_same_colors() {
        let semantic = Palette::parse(SEMANTIC).unwrap();
        assert_eq!(
            semantic.surface,
            mix(semantic.background, semantic.foreground, 0.05)
        );
        assert_eq!(
            semantic.border,
            mix(semantic.background, semantic.foreground, 0.25)
        );
        let ansi = SEMANTIC
            .replace("accent", "color4")
            .replace("red", "color1")
            .replace("yellow", "color3")
            .replace("green", "color2");
        assert_eq!(Palette::parse(&ansi), Some(semantic));
    }
    #[test]
    fn incomplete_or_malformed_palettes_never_mix_with_a_builtin() {
        assert!(Palette::parse("background = '#ffffff'").is_none());
        assert!(Palette::parse(&SEMANTIC.replace("#ed7070", "#bad")).is_none());
        assert!(Palette::parse(&SEMANTIC.replace("#ed7070", "red")).is_none());
        assert!(Palette::parse("not toml").is_none());
        assert!(Palette::parse(&format!("{SEMANTIC}muted = 'blue'\n")).is_none());
        assert!(Palette::parse(&format!("{SEMANTIC}mode = 'unknown'\n")).is_none());
        assert!(load_palette(None).is_none());
        for value in ["#abc", "101820", "#001122ff", "#GG0000"] {
            assert!(hex(value).is_none());
        }
    }
    #[test]
    fn stale_legacy_theme_is_not_revived_when_current_is_invalid() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join(".config/omarchy/current/theme");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("colors.toml"), SEMANTIC).unwrap();
        assert!(load_palette(Some(root.path())).is_some());
        std::fs::create_dir_all(root.path().join(".local/state/omarchy/current")).unwrap();
        assert!(load_palette(Some(root.path())).is_none());
    }
    #[cfg(unix)]
    #[test]
    fn a_dangling_current_symlink_does_not_enable_legacy_fallback() {
        let root = tempfile::tempdir().unwrap();
        let current = root.path().join(".local/state/omarchy/current");
        std::fs::create_dir_all(current.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(root.path().join("missing"), &current).unwrap();
        assert_eq!(theme_file(root.path()), current.join("theme/colors.toml"));
    }
    #[test]
    fn foreground_and_status_text_and_accent_ink_remain_readable() {
        for light in [true, false] {
            let palette = Palette::builtin(light);
            for text in [
                palette.foreground,
                palette.secondary,
                palette.danger,
                palette.warning,
                palette.success,
            ] {
                assert!(contrast(text, palette.background) >= 4.5);
            }
            assert!(contrast(palette.on_accent, palette.accent) >= 4.5);
        }
        let pale = Palette::parse(
            &SEMANTIC
                .replace("#101820", "#ffffff")
                .replace("#e0e8f0", "#111111")
                .replace("#ed7070", "#ffcccc"),
        )
        .unwrap();
        assert!(pale.light);
        assert!(contrast(pale.danger, pale.background) >= 4.5);
    }
    #[test]
    fn machine_font_and_spacing_overrides_win_once() {
        let root = tempfile::tempdir().unwrap();
        let theme = root.path().join(".config/omarchy/current/theme");
        std::fs::create_dir_all(&theme).unwrap();
        std::fs::write(
            theme.join("shell.toml"),
            "[font]\nbase-size = 14\nfamily = 'Fira Mono'\n[spacing]\nscale = 1.5\n",
        )
        .unwrap();
        let config = root.path().join(".config/omarchy/shell.toml");
        std::fs::write(&config, "[font]\nbase-size = 16\n").unwrap();
        let metrics = metrics_at(Some(root.path()));
        assert_eq!(metrics.font_size, 16.0);
        assert_eq!(metrics.family, "Fira Mono");
        assert_eq!(metrics.space(10.0), 20.0);
        std::fs::write(config, "[spacing]\nscale-with-font = false\n").unwrap();
        assert_eq!(metrics_at(Some(root.path())).space(10.0), 15.0);
    }
    #[test]
    fn modes_round_trip_and_invalid_preferences_follow_system() {
        for mode in [Mode::System, Mode::Light, Mode::Dark] {
            assert_eq!(Mode::parse(mode.value()), mode);
        }
        assert_eq!(Mode::parse("unknown"), Mode::System);
    }
    #[test]
    fn standalone_ignores_omarchy_files_and_uses_its_own_palette() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(".config/omarchy/current/theme");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("colors.toml"), SEMANTIC).unwrap();
        std::fs::write(
            path.join("shell.toml"),
            "[font]\nfamily = 'Unexpected font'\nbase-size = 24\n",
        )
        .unwrap();
        let standalone = Startup::for_environment(Some(root.path()), Some("dark"), false);
        assert!(standalone.system.is_none());
        assert_eq!(standalone.metrics, Metrics::default());
        assert_eq!(standalone.mode, Mode::Dark);
        let palette = Palette::standalone(false);
        assert_eq!(palette.background, 0x070708);
        assert_eq!(palette.accent, 0x65f193);
        let omarchy = Startup::for_environment(Some(root.path()), Some("dark"), true);
        assert!(omarchy.system.is_some());
        assert_eq!(omarchy.metrics.family, "Unexpected font");
        assert_eq!(omarchy.mode, Mode::System);
        assert_ne!(
            omarchy.theme(Mode::System).color("background").unwrap(),
            standalone.theme(Mode::System).color("background").unwrap()
        );
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

    #[test]
    fn standalone_text_remains_readable_in_both_appearances() {
        for light in [true, false] {
            let p = Palette::standalone(light);
            for text in [p.foreground, p.secondary, p.danger, p.warning, p.success] {
                assert!(
                    contrast(text, p.background) >= 4.5,
                    "{text:06x} on {:06x}",
                    p.background
                );
            }
            assert!(contrast(p.on_accent, p.accent) >= 4.5);
        }
    }
}
