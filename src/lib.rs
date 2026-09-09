//! Application policy around the official LocalSend Rust core.
rust_i18n::i18n!("locales", fallback = "en");

pub mod clipboard;
pub mod diagnostics;
pub mod localsend;
pub mod model;

pub mod i18n;

pub mod updates;
