//! Application policy around the official LocalSend Rust core.
rust_i18n::i18n!("locales", fallback = "en");

pub mod clipboard;
pub mod diagnostics;
pub mod localsend;
pub mod model;

pub mod i18n;

pub mod updates;

// Let the shared UI use the same policy paths as external consumers.
extern crate self as omasend;

#[cfg(feature = "ui")]
pub mod views;
#[cfg(feature = "ui")]
pub use views::App;

#[cfg(feature = "ui")]
#[nsh_core::element::native_module("omasend/native")]
fn native(registry: &mut nsh_core::element::Registry) {
    registry.register::<App>();
}

#[cfg(feature = "ui")]
pub fn app_root() -> nsh_core::element::Root {
    views::appearance::root()
}

/// Both frontends may be compiled together; update and logging policy follow
/// the running executable rather than the union of Cargo features.
pub(crate) fn terminal_binary() -> bool {
    static TERMINAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TERMINAL.get_or_init(|| {
        cfg!(all(feature = "tui", not(feature = "desktop")))
            || std::env::current_exe()
                .is_ok_and(|path| path.file_stem().is_some_and(|name| name == "omasend-tui"))
    })
}
