//! GPUI Kit's bundled icons, resolved against the current text color at render time.
use gpui_kit::base::StyledExt;
use gpui_kit::{App, IntoElement, RenderOnce, StyleRefinement, Styled, Window, px, svg};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconName {
    Check,
    Send,
    History,
    Minus,
    Plus,
    ChevronDown,
    ChevronUp,
    ChevronRight,
    ChevronLeft,
    Calendar,
    Star,
    ExternalLink,
    Close,
    Search,
    Menu,
    Settings,
    TriangleAlert,
}
impl IconName {
    pub fn path(self) -> &'static str {
        match self {
            Self::History => "icons/history.svg",
            Self::Send => "icons/send.svg",
            Self::Check => "icons/check.svg",
            Self::Minus => "icons/minus.svg",
            Self::Plus => "icons/plus.svg",
            Self::ChevronUp => "icons/chevron-up.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::Calendar => "icons/calendar.svg",
            Self::ChevronLeft => "icons/chevron-left.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::Star => "icons/star.svg",
            Self::ExternalLink => "icons/external-link.svg",
            Self::Close => "icons/close.svg",
            Self::Search => "icons/search.svg",
            Self::Menu => "icons/menu.svg",
            Self::Settings => "icons/settings.svg",
            Self::TriangleAlert => "icons/triangle-alert.svg",
        }
    }
}

#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    style: StyleRefinement,
}
impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        // Svg only paints when its own text color is set. Resolve inheritance here,
        // then apply caller refinements so explicit size and color still win.
        svg()
            .data(&icon_data(self.name))
            .text_color(window.text_style().color)
            .refine_style(&self.style)
    }
}
#[cfg(not(target_family = "wasm"))]
fn icon_data(name: IconName) -> std::borrow::Cow<'static, [u8]> {
    if name == IconName::Send {
        return std::borrow::Cow::Borrowed(include_bytes!("../assets/icons/send.svg"));
    }
    if name == IconName::History {
        return std::borrow::Cow::Borrowed(include_bytes!("../assets/icons/history.svg"));
    }
    gpui_kit::assets::Assets::get(name.path())
        .expect("bundled icon exists")
        .data
}

// gpui-kit-assets exposes asynchronous AssetSource loading on the web rather
// than Assets::get. Embed the small set used here so first paint is complete.
#[cfg(target_family = "wasm")]
fn icon_data(name: IconName) -> &'static [u8] {
    match name {
        IconName::History => include_bytes!("../assets/icons/history.svg"),
        IconName::Send => include_bytes!("../assets/icons/send.svg"),
        IconName::Check => include_bytes!("../assets/icons/check.svg"),
        IconName::Minus => include_bytes!("../assets/icons/minus.svg"),
        IconName::Plus => include_bytes!("../assets/icons/plus.svg"),
        IconName::ChevronUp => include_bytes!("../assets/icons/chevron-up.svg"),
        IconName::ChevronDown => include_bytes!("../assets/icons/chevron-down.svg"),
        IconName::Calendar => include_bytes!("../assets/icons/calendar.svg"),
        IconName::ChevronLeft => include_bytes!("../assets/icons/chevron-left.svg"),
        IconName::ChevronRight => include_bytes!("../assets/icons/chevron-right.svg"),
        IconName::Star => include_bytes!("../assets/icons/star.svg"),
        IconName::ExternalLink => include_bytes!("../assets/icons/external-link.svg"),
        IconName::Close => include_bytes!("../assets/icons/close.svg"),
        IconName::Search => include_bytes!("../assets/icons/search.svg"),
        IconName::Menu => include_bytes!("../assets/icons/menu.svg"),
        IconName::Settings => include_bytes!("../assets/icons/settings.svg"),
        IconName::TriangleAlert => include_bytes!("../assets/icons/triangle-alert.svg"),
    }
}
/// A 16px GPUI Kit icon. No application AssetSource replacement is needed.
pub fn icon(name: IconName) -> Icon {
    Icon {
        name,
        style: StyleRefinement::default(),
    }
    .size(px(16.))
    .flex_shrink_0()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_named_icons_exist_in_kit_assets() {
        for name in [
            IconName::Send,
            IconName::History,
            IconName::Check,
            IconName::Minus,
            IconName::Plus,
            IconName::ChevronDown,
            IconName::ChevronUp,
            IconName::ChevronRight,
            IconName::ChevronLeft,
            IconName::Calendar,
            IconName::Star,
            IconName::ExternalLink,
            IconName::Close,
            IconName::Search,
            IconName::Menu,
            IconName::Settings,
            IconName::TriangleAlert,
        ] {
            assert!(!icon_data(name).is_empty(), "{}", name.path());
        }
    }
}
