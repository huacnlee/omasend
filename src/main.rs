mod views;

use gpui_omarchy::gpui::{AppContext, Bounds, WindowBounds, WindowOptions, px, size};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_ansi(false)
        .init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("omasend-network")
        .build()?;
    let handle = runtime.handle().clone();
    gpui_omarchy::application().run(move |cx| {
        gpui_omarchy::init(cx);
        cx.set_app_identity("omasend", "OmaSend");
        views::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        if let Err(error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(840.), px(720.)),
                    cx,
                ))),
                window_min_size: Some(size(px(640.), px(520.))),
                titlebar: None,
                app_id: Some("omasend".into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| views::Home::new(handle.clone(), window, cx)),
        ) {
            eprintln!("Could not open OmaSend: {error:#}");
            cx.quit();
        }
        cx.activate(true);
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    Ok(())
}
