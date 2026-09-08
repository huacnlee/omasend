#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod views;

use gpui_omarchy::gpui::{
    App, AppContext, Bounds, Entity, Global, WindowBounds, WindowOptions, px, size,
};

// On macOS the application session survives closing its last native window.
// Reopening attaches the same entity, preserving the node and pending transfers.
struct DesktopSession {
    home: Option<Entity<views::Home>>,
}
impl Global for DesktopSession {}

fn open_or_activate_window(cx: &mut App, runtime: tokio::runtime::Handle) -> anyhow::Result<()> {
    if let Some(window) = cx.windows().first().copied() {
        window.update(cx, |_, window, _| window.activate_window())?;
        cx.activate(true);
        return Ok(());
    }
    let existing = cx.global::<DesktopSession>().home.clone();
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(840.), px(630.)),
                cx,
            ))),
            window_min_size: Some(size(px(640.), px(520.))),
            titlebar: None,
            app_id: Some("omasend".into()),
            ..Default::default()
        },
        |window, cx| {
            let home = if let Some(home) = existing {
                home.update(cx, |home, cx| home.attach_window(window, cx));
                home
            } else {
                cx.new(|cx| views::Home::new(runtime, window, cx))
            };
            cx.global_mut::<DesktopSession>().home = Some(home.clone());
            home
        },
    )?;
    cx.activate(true);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let executable = std::env::current_exe()?;
    omasend::diagnostics::init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("omasend-network")
        .build()?;
    let handle = runtime.handle().clone();
    let application = gpui_omarchy::application();
    #[cfg(target_os = "macos")]
    {
        let handle = handle.clone();
        application.on_reopen(move |cx| {
            if let Err(error) = open_or_activate_window(cx, handle.clone()) {
                tracing::error!("Could not reopen Omasend: {error:#}");
            }
        });
    }
    application.run(move |cx| {
        gpui_omarchy::init(cx);
        cx.set_app_identity("omasend", "Omasend");
        views::init(cx);
        views::theme::load(cx);
        cx.set_global(DesktopSession { home: None });
        cx.on_action(|_: &views::Quit, cx| cx.quit());
        #[cfg(target_os = "macos")]
        cx.set_menus(vec![
            gpui_omarchy::gpui::Menu::new("Omasend")
                .items([gpui_omarchy::gpui::MenuItem::action("Exit", views::Quit)]),
        ]);
        #[cfg(not(target_os = "macos"))]
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        if let Err(error) = open_or_activate_window(cx, handle) {
            eprintln!("Could not open Omasend: {error:#}");
            cx.quit();
        }
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    if omasend::updates::RESTART_REQUESTED.load(std::sync::atomic::Ordering::SeqCst) {
        omasend::updates::install::restart(&executable)?;
    }
    Ok(())
}

#[cfg(all(test, feature = "ui-tests"))]
mod lifecycle_tests {
    use super::*;
    use gpui_omarchy::gpui;

    #[gpui::test]
    async fn closing_and_reopening_retains_session_without_duplicate_windows(
        cx: &mut gpui::TestAppContext,
    ) {
        // This exercises production preparation on Tokio's blocking pool. Its
        // completion wakes GPUI from a real worker thread, so opt into the test
        // scheduler's supported mixed-I/O mode instead of deterministic-only mode.
        cx.executor().allow_parking();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let home = cx.update(|cx| {
            gpui_omarchy::init(cx);
            views::init(cx);
            let home = cx.new(|cx| views::Home {
                state: omasend::model::AppState::default(),
                language: omasend::i18n::Language::En,
                node: None,
                runtime: runtime.handle().clone(),
                focus: cx.focus_handle(),
                window_handle: None,
                modal_focus: cx.focus_handle(),
                nearby_expanded: false,
                nearby_scroll: gpui::UniformListScrollHandle::new(),
                history_expanded: false,
                history_scroll: gpui::ScrollHandle::new(),
                logs: None,
                about_open: false,
                update_state: Default::default(),
                show_update_status: false,
                update_status_dismiss: None,
                restore_focus: None,
                preview: None,
                loading_input: false,
                starting: false,
            });
            cx.set_global(DesktopSession {
                home: Some(home.clone()),
            });
            open_or_activate_window(cx, runtime.handle().clone()).unwrap();
            open_or_activate_window(cx, runtime.handle().clone()).unwrap();
            assert_eq!(cx.windows().len(), 1);
            home
        });
        cx.update(|cx| {
            let window = cx.windows()[0];
            window
                .update(cx, |_, window, cx| {
                    home.update(cx, |home, cx| {
                        home.add_items(
                            vec![omasend::model::SendItem::Text(
                                "Prepared after closing".into(),
                            )],
                            window,
                            cx,
                        );
                        home.close_window(window, cx);
                    });
                })
                .unwrap();
        });
        cx.condition(&home, |home, _| !home.loading_input).await;
        cx.run_until_parked();
        cx.update(|cx| {
            assert!(cx.windows().is_empty());
            home.update(cx, |home, cx| {
                home.state
                    .apply(omasend::localsend::TransferEvent::DeviceFound(
                        omasend::localsend::Device {
                            fingerprint: "offline-test-peer".into(),
                            alias: "Nearby peer".into(),
                            model: "Linux".into(),
                            host: "127.0.0.1".into(),
                            port: 53317,
                        },
                    ));
                cx.notify();
            });
            open_or_activate_window(cx, runtime.handle().clone()).unwrap();
            assert_eq!(cx.windows().len(), 1);
            let retained = cx.global::<DesktopSession>().home.as_ref().unwrap();
            assert_eq!(retained.entity_id(), home.entity_id());
            assert_eq!(retained.read(cx).state.composer.len(), 1);
            assert_eq!(retained.read(cx).state.devices.len(), 1);
            assert!(retained.read(cx).window_handle.is_some());
        });
    }
}
