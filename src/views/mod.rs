mod about;
mod history;
mod home;
mod logs;
mod motion;
mod nearby;
mod panels;
pub mod theme;
mod updates;

use gpui_omarchy::gpui::{App, KeyBinding, actions};
pub use home::Home;

// Application spacing uses a 4px grid, expressed in rems to follow text scale.
const PANEL_PADDING: f32 = 1.;
const PANEL_GAP: f32 = 12. / 16.;
const CONTROL_GAP: f32 = 8. / 16.;

actions!(
    omasend,
    [
        Paste,
        OpenFiles,
        Back,
        Confirm,
        PreviousDevice,
        NextDevice,
        Quit,
        CloseWindow
    ]
);

pub fn init(cx: &mut App) {
    cx.set_global(theme::ThemeMode::default());
    cx.bind_keys([
        KeyBinding::new("ctrl-v", Paste, Some("Omasend")),
        KeyBinding::new("ctrl-o", OpenFiles, Some("Omasend")),
        KeyBinding::new("escape", Back, Some("Omasend")),
        KeyBinding::new("enter", Confirm, Some("Omasend")),
        KeyBinding::new("up", PreviousDevice, Some("Omasend")),
        KeyBinding::new("left", PreviousDevice, Some("Omasend")),
        KeyBinding::new("down", NextDevice, Some("Omasend")),
        KeyBinding::new("right", NextDevice, Some("Omasend")),
    ]);
    #[cfg(target_os = "macos")]
    cx.bind_keys([
        KeyBinding::new("cmd-v", Paste, Some("Omasend")),
        KeyBinding::new("cmd-o", OpenFiles, Some("Omasend")),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseWindow, Some("Omasend")),
    ]);
}

pub fn size_label(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024. * 1024.))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024. * 1024. * 1024.))
    }
}
