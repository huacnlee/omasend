#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

fn main() -> std::process::ExitCode {
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("omasend {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    omasend::diagnostics::init();
    platform::set_ui_sets(vec![
        nsh_ui_mobile::registry(),
        omasend::__nsh_native_module(),
    ]);
    let executable = std::env::current_exe();
    let status = platform::run_app(
        platform::AppSource::Rust(omasend::app_root()),
        platform::AppOptions {
            title: "Omasend".into(),
            size: (840, 630),
            #[cfg(target_os = "macos")]
            keep_alive_on_close: true,
            script: std::env::var("NSH_SCRIPT").ok(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: "omasend".into(),
            ..Default::default()
        },
    );
    if omasend::updates::RESTART_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst) {
        let result = executable
            .map_err(anyhow::Error::from)
            .and_then(|path| omasend::updates::install::restart(&path));
        if let Err(error) = result {
            tracing::error!(%error, "Could not restart Omasend");
            return std::process::ExitCode::FAILURE;
        }
    }
    std::process::Termination::report(status)
}
