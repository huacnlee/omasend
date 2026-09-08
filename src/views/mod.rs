mod history;
mod home;
mod logs;
mod motion;
mod nearby;
mod panels;

use gpui_omarchy::gpui::{App, KeyBinding, actions};
pub use home::Home;

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
    cx.bind_keys([
        KeyBinding::new("ctrl-v", Paste, Some("OmaSend")),
        KeyBinding::new("ctrl-o", OpenFiles, Some("OmaSend")),
        KeyBinding::new("escape", Back, Some("OmaSend")),
        KeyBinding::new("enter", Confirm, Some("OmaSend")),
        KeyBinding::new("up", PreviousDevice, Some("OmaSend")),
        KeyBinding::new("left", PreviousDevice, Some("OmaSend")),
        KeyBinding::new("down", NextDevice, Some("OmaSend")),
        KeyBinding::new("right", NextDevice, Some("OmaSend")),
    ]);
    #[cfg(target_os = "macos")]
    cx.bind_keys([
        KeyBinding::new("cmd-v", Paste, Some("OmaSend")),
        KeyBinding::new("cmd-o", OpenFiles, Some("OmaSend")),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseWindow, Some("OmaSend")),
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
