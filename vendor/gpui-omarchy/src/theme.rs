//! Semantic palette and its projection into gpui-base.
use gpui::{App, Global, Hsla, SharedString, px, rgb};
use gpui_base::{ColorTokens, RadiusTokens, ThemeAppearance};

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: SharedString,
    pub appearance: ThemeAppearance,
    pub background: Hsla,
    pub surface: Hsla,
    pub inset: Hsla,
    pub foreground: Hsla,
    pub secondary: Hsla,
    pub bright: Hsla,
    pub accent: Hsla,
    pub on_accent: Hsla,
    pub selection: Hsla,
    pub border: Hsla,
    pub danger: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    pub font: SharedString,
}
impl Global for Theme {}

impl Theme {
    /// Omarchy's semantic Tokyo Night palette (quattro/themes/tokyo-night/colors.toml).
    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night".into(),
            appearance: ThemeAppearance::Dark,
            background: rgb(0x1a1b26).into(),
            surface: rgb(0x24283b).into(),
            inset: rgb(0x13141c).into(),
            foreground: rgb(0xa9b1d6).into(),
            secondary: rgb(0xa9b1d6).into(),
            bright: rgb(0xc0caf5).into(),
            accent: rgb(0x7aa2f7).into(),
            on_accent: rgb(0x13141c).into(),
            selection: rgb(0x292e42).into(),
            border: rgb(0x414868).into(),
            danger: rgb(0xf7768e).into(),
            warning: rgb(0xe0af68).into(),
            success: rgb(0x9ece6a).into(),
            font: ".SystemUIFont".into(),
        }
    }

    /// Warm light presentation; status colors are darkened for readable text.
    pub fn flexoki_light() -> Self {
        Self {
            name: "Flexoki Light".into(),
            appearance: ThemeAppearance::Light,
            background: rgb(0xfffcf0).into(),
            surface: rgb(0xf2f0e5).into(),
            inset: rgb(0xe6e4d9).into(),
            foreground: rgb(0x100f0f).into(),
            secondary: rgb(0x575653).into(),
            bright: rgb(0x100f0f).into(),
            accent: rgb(0x205ea6).into(),
            on_accent: rgb(0xfffcf0).into(),
            selection: rgb(0xdad8ce).into(),
            border: rgb(0xb7b5ac).into(),
            danger: rgb(0xaf3029).into(),
            warning: rgb(0x855b00).into(),
            success: rgb(0x526600).into(),
            font: ".SystemUIFont".into(),
        }
    }

    /// Shared shell control colors, separate from the palette's selection ramp.
    pub fn normal_fill(&self) -> Hsla {
        self.foreground.opacity(0.04)
    }
    pub fn hover_fill(&self) -> Hsla {
        self.foreground.opacity(0.08)
    }
    pub fn selected_fill(&self) -> Hsla {
        self.foreground.opacity(0.18)
    }
    pub fn pressed_fill(&self) -> Hsla {
        self.foreground.opacity(0.22)
    }
    pub fn control_border(&self) -> Hsla {
        self.foreground.opacity(0.4)
    }
    pub fn focus_border(&self) -> Hsla {
        self.foreground.opacity(0.25)
    }
    pub fn divider(&self) -> Hsla {
        self.foreground.opacity(0.12)
    }

    /// Shell editors use foreground at 35%, independently of colors.toml selection.
    pub fn input_style(&self) -> gpui_base::input::InputEditorStyle {
        gpui_base::input::InputEditorStyle {
            foreground: self.foreground,
            muted_foreground: self.foreground.opacity(0.55),
            background: self.normal_fill(),
            border: self.control_border(),
            selection: self.foreground.opacity(0.35),
            caret: self.foreground,
            ..Default::default()
        }
    }

    pub fn tokens(&self) -> ColorTokens {
        ColorTokens {
            background: self.background,
            foreground: self.foreground,
            surface: self.surface,
            surface_foreground: self.foreground,
            primary: self.accent,
            primary_foreground: self.on_accent,
            secondary: self.surface,
            secondary_foreground: self.foreground,
            muted: self.inset,
            muted_foreground: self.secondary,
            accent: self.selection,
            accent_foreground: self.bright,
            destructive: self.danger,
            destructive_foreground: self.background,
            border: self.border,
            input: self.border,
            ring: self.accent,
            selection: self.selection,
        }
    }

    /// Atomically update presentation and base tokens, then redraw all windows.
    /// Applying an explicit theme stops following system theme changes.
    pub fn apply(self, cx: &mut App) {
        crate::system_theme::stop_following(cx);
        self.apply_palette(cx);
    }

    pub(crate) fn apply_palette(self, cx: &mut App) {
        let base = gpui_base::Theme::global_mut(cx);
        base.appearance = self.appearance;
        base.tokens.colors = self.tokens();
        base.tokens.radius = RadiusTokens {
            none: px(0.),
            sm: px(0.),
            md: px(0.),
            lg: px(0.),
            xl: px(0.),
            full: px(0.),
        };
        let type_scale = &mut base.tokens.typography;
        type_scale.sans = self.font.clone();
        type_scale.mono = self.font.clone();
        for (token, size) in [
            (&mut type_scale.xs, 10.),
            (&mut type_scale.sm, 11.),
            (&mut type_scale.md, 12.),
            (&mut type_scale.lg, 14.),
            (&mut type_scale.xl, 16.),
            (&mut type_scale.mono_md, 12.),
        ] {
            token.size = px(size);
            token.line_height = px(size * 1.5);
        }
        cx.set_global(self);
        cx.refresh_windows();
    }
}

pub trait ActiveTheme {
    fn omarchy(&self) -> &Theme;
}
impl ActiveTheme for App {
    fn omarchy(&self) -> &Theme {
        self.global::<Theme>()
    }
}
