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

#[cfg(feature = "desktop")]
pub mod views;
#[cfg(feature = "desktop")]
pub use views::App;

#[cfg(feature = "desktop")]
#[nsh_core::ui::native_module("omasend/native")]
fn native(registry: &mut nsh_core::ui::Registry) {
    registry.register::<App>();
}

#[cfg(feature = "desktop")]
pub fn app_root() -> nsh_core::ui::Root {
    views::appearance::root()
}
