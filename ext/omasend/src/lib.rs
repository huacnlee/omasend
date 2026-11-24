//! Application policy around the official LocalSend Rust core.
rust_i18n::i18n!("locales", fallback = "en");

pub mod clipboard;
pub mod diagnostics;
pub mod localsend;
pub mod model;
pub mod netcheck;

pub mod i18n;

pub mod updates;

// Let the shared UI use the same policy paths as external consumers.
extern crate self as omasend;

#[cfg(feature = "ui")]
pub mod api;
#[cfg(feature = "ui")]
pub(crate) mod session;

#[cfg(feature = "ui")]
#[nsh_core::element::native_module("omasend/native")]
fn native(registry: &mut nsh_core::element::Registry) {
    registry.module::<api::Omasend>();
}

/// The TSX interface (`App.tsx`), as `nsh build --release` bundled it: built
/// in when `OMASEND_BUNDLE` named its output directory at compile time, read
/// from the `bundle.js` in the directory `OMASEND_BUNDLE` names otherwise.
#[cfg(feature = "ui")]
pub fn interface() -> anyhow::Result<nsh_engine::Bundle> {
    #[cfg(omasend_bundle)]
    let code = include_str!(env!("OMASEND_BUNDLE_JS")).to_owned();
    #[cfg(not(omasend_bundle))]
    let code = {
        use anyhow::Context;
        let dir = std::env::var_os("OMASEND_BUNDLE").context(
            "This build has no interface: set OMASEND_BUNDLE to the output of `nsh build --release`",
        )?;
        let file = std::path::Path::new(&dir).join("bundle.js");
        std::fs::read_to_string(&file).with_context(|| format!("Cannot read {}", file.display()))?
    };
    Ok(nsh_engine::Bundle::source(code).with_app_id(Some("com.huacnlee.omasend")))
}

/// The native modules the interface imports: NSH's, nsh-omarchy's and ours.
#[cfg(feature = "ui")]
pub fn ui_sets() -> Vec<nsh_core::element::Registry> {
    vec![
        nsh_base::registry(),
        nsh_omarchy::__nsh_native_module(),
        __nsh_native_module(),
    ]
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
