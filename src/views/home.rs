use super::*;
use anyhow::{Context as _, Result};
use gpui_omarchy::gpui::{
    self, AnyElement, Context, ExternalPaths, FocusHandle, PathPromptOptions, Window, div,
    prelude::*, rems,
};
use gpui_omarchy::{
    ActiveTheme, ButtonVariant, IconName, MenuItem, button, focus_scope, icon, menu, separator,
};
use omasend::{
    localsend::{Node, NodeConfig, TransferEvent},
    model::{AppState, ComposerItem, SendItem, TransferStatus},
};

pub struct Home {
    pub state: AppState,

    pub language: omasend::i18n::Language,
    pub node: Option<Node>,
    pub runtime: tokio::runtime::Handle,
    pub focus: FocusHandle,
    pub window_handle: Option<gpui::AnyWindowHandle>,
    pub modal_focus: FocusHandle,
    pub nearby_expanded: bool,
    pub nearby_scroll: gpui::UniformListScrollHandle,
    pub history_expanded: bool,
    pub history_scroll: gpui::ScrollHandle,
    pub logs: Option<super::logs::LogsPanel>,
    pub about_open: bool,
    pub update_state: omasend::updates::UpdateState,
    pub show_update_status: bool,
    pub update_status_dismiss: Option<gpui::Task<()>>,
    pub restore_focus: Option<FocusHandle>,
    pub preview: Option<String>,
    pub loading_input: bool,
    pub starting: bool,
}

impl Home {
    pub fn new(
        runtime: tokio::runtime::Handle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut view = Self {
            state: AppState::default(),
            language: omasend::i18n::Language::system(),
            node: None,
            runtime,
            focus,
            window_handle: Some(window.window_handle()),
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
        };
        view.watch_theme(window, cx);
        view.connect(window, cx);
        view.start_update_checks(cx);
        view
    }

    pub fn attach_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.window_handle = Some(window.window_handle());
        self.watch_theme(window, cx);
        if self.state.incoming.is_some() || self.about_open {
            self.modal_focus.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }

    fn watch_theme(&self, window: &mut Window, cx: &mut Context<Self>) {
        use super::theme::ThemeMode;
        cx.global::<ThemeMode>().to_owned().apply(window, cx);
        cx.observe_window_appearance(window, |_, window, cx| {
            if *cx.global::<ThemeMode>() == ThemeMode::System && !cfg!(target_os = "linux") {
                ThemeMode::System.apply(window, cx);
                cx.notify();
            }
        })
        .detach();
    }

    pub fn close_window(&mut self, window: &mut Window, _: &mut Context<Self>) {
        self.window_handle = None;
        self.logs = None;
        self.about_open = false;
        self.restore_focus = None;
        self.history_expanded = false;
        self.preview = None;
        window.remove_window();
    }

    // State updates outlive the native window. Focus changes apply only to the
    // currently attached window, after the entity's update borrow has ended.
    fn with_window(
        &self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let Some(handle) = self.window_handle else {
            return;
        };
        let entity = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                let _ = entity.update(cx, |view, cx| update(view, window, cx));
            });
        });
    }

    fn connect(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.starting || self.node.is_some() {
            return;
        }
        self.starting = true;
        let task = self
            .runtime
            .spawn(async { Node::start(NodeConfig::desktop()?).await });
        cx.spawn(async move |this, cx| {
            let result = task
                .await
                .context("Network task stopped")
                .and_then(|result| result);
            match result {
                Ok(node) => {
                    let events = node.events.clone();
                    if this
                        .update(cx, |view, cx| {
                            view.node = Some(node);
                            view.starting = false;
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                    while let Ok(event) = events.recv().await {
                        if this
                            .update(cx, |view, cx| {
                                let was_incoming = view.state.incoming.is_some();
                                let incoming =
                                    matches!(&event, TransferEvent::IncomingRequest { .. });
                                let preserve_focus = view.history_expanded
                                    || view.preview.is_some()
                                    || view.logs.is_some()
                                    || view.about_open;
                                if incoming {
                                    view.logs = None;
                                    view.about_open = false;
                                    view.history_expanded = false;
                                    view.preview = None;
                                }
                                view.state.apply(event);
                                let restore = was_incoming && view.state.incoming.is_none();
                                if incoming || restore {
                                    view.with_window(cx, move |view, window, cx| {
                                        if incoming && view.state.incoming.is_some() {
                                            if !preserve_focus {
                                                view.restore_focus = window.focused(cx);
                                            }
                                            view.modal_focus.focus(window, cx);
                                        } else if restore && view.state.incoming.is_none() {
                                            view.restore(window, cx);
                                        }
                                    });
                                }
                                cx.notify();
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                Err(error) => {
                    let _ = this.update(cx, |view, cx| {
                        view.starting = false;
                        view.state.error = Some(format!("{error:#}"));
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub fn restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.restore_focus
            .take()
            .unwrap_or_else(|| self.focus.clone())
            .focus(window, cx);
    }

    pub fn add_items(&mut self, items: Vec<SendItem>, _: &mut Window, cx: &mut Context<Self>) {
        if self.loading_input
            || self.history_expanded
            || self.logs.is_some()
            || self.about_open
            || self.preview.is_some()
            || self.state.incoming.is_some()
        {
            return;
        }
        self.prepare_items(items, cx);
    }

    fn prepare_items(&mut self, items: Vec<SendItem>, cx: &mut Context<Self>) {
        self.loading_input = true;
        let task = self.runtime.spawn_blocking(move || {
            items
                .into_iter()
                .map(ComposerItem::new)
                .collect::<Result<Vec<_>>>()
        });
        cx.spawn(async move |this, cx| {
            let result = task
                .await
                .context("File preparation stopped")
                .and_then(|result| result);
            let _ = this.update(cx, |view, cx| {
                view.loading_input = false;
                match result {
                    Ok(items) => {
                        view.state.composer.extend(items);
                        view.state.error = None;
                    }
                    Err(error) => view.state.error = Some(format!("{error:#}")),
                }
                view.with_window(cx, |view, window, cx| {
                    if view.logs.is_none()
                        && !view.about_open
                        && !view.history_expanded
                        && view.preview.is_none()
                        && view.state.incoming.is_none()
                    {
                        view.focus.focus(window, cx);
                    }
                });
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        if self.loading_input
            || self.history_expanded
            || self.logs.is_some()
            || self.about_open
            || self.preview.is_some()
            || self.state.incoming.is_some()
        {
            return;
        }
        #[cfg(target_os = "linux")]
        {
            self.loading_input = true;
            let task = self.runtime.spawn(omasend::clipboard::wayland::paste());
            cx.spawn(async move |this, cx| {
                let result = task
                    .await
                    .context("Clipboard task stopped")
                    .and_then(|result| result);
                let _ = this.update(cx, |view, cx| {
                    view.loading_input = false;
                    match result {
                        Ok(items) => view.prepare_items(items, cx),
                        Err(error) => view.state.error = Some(format!("{error:#}")),
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let Some(clipboard) = cx.read_from_clipboard() else {
                self.state.error = Some("Clipboard is empty".into());
                cx.notify();
                return;
            };
            // The development host uses GPUI's typed clipboard entries, with
            // the same file/image/text priority as the Wayland implementation.
            let entries: Vec<_> = clipboard.into_entries().collect();
            let paths: Vec<_> = entries
                .iter()
                .filter_map(|entry| {
                    if let gpui::ClipboardEntry::ExternalPaths(paths) = entry {
                        Some(paths.0.iter().cloned())
                    } else {
                        None
                    }
                })
                .flatten()
                .map(SendItem::File)
                .collect();
            if !paths.is_empty() {
                self.add_items(paths, _window, cx);
                return;
            }
            if let Some(image) = entries.iter().find_map(|entry| {
                if let gpui::ClipboardEntry::Image(image) = entry {
                    Some(image)
                } else {
                    None
                }
            }) {
                let mime = match image.format() {
                    gpui::ImageFormat::Png => "image/png",
                    gpui::ImageFormat::Jpeg => "image/jpeg",
                    gpui::ImageFormat::Tiff => "image/tiff",
                    gpui::ImageFormat::Webp => "image/webp",
                    gpui::ImageFormat::Gif => "image/gif",
                    gpui::ImageFormat::Bmp => "image/bmp",
                    _ => {
                        self.state.error = Some("Copy the image as PNG or JPEG".into());
                        cx.notify();
                        return;
                    }
                };
                let result = omasend::clipboard::read_offer(
                    omasend::clipboard::ClipboardKind::Image(mime.into()),
                    image.bytes(),
                    &std::env::temp_dir(),
                );
                match result {
                    Ok(items) => self.add_items(items, _window, cx),
                    Err(error) => self.state.error = Some(error.to_string()),
                }
            } else {
                let items = entries
                    .into_iter()
                    .filter_map(|entry| {
                        if let gpui::ClipboardEntry::String(value) = entry {
                            Some(SendItem::Text(value.text().clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                self.add_items(items, _window, cx);
            }
        }
        cx.notify();
    }

    pub(super) fn open_files(&mut self, _: &OpenFiles, _: &mut Window, cx: &mut Context<Self>) {
        if self.loading_input
            || self.history_expanded
            || self.logs.is_some()
            || self.about_open
            || self.preview.is_some()
            || self.state.incoming.is_some()
        {
            return;
        }
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some(self.language.text("Add to OmaSend").into()),
        });
        cx.spawn(async move |this, cx| {
            let result = prompt.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        view.prepare_items(paths.into_iter().map(SendItem::File).collect(), cx)
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => view.state.error = Some(error.to_string()),
                    Err(error) => view.state.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn send(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        if self.state.sending() || self.state.composer.is_empty() || self.loading_input {
            return;
        }
        let Some(node) = &self.node else { return };
        let Some(device) = self.state.selected_device().cloned() else {
            return;
        };
        let files = self
            .state
            .composer
            .iter()
            .flat_map(|item| item.uploads.clone())
            .collect();
        match node.handle.send(device, files, None) {
            Ok(id) => self.state.track_send(id),
            Err(error) => self.state.error = Some(error.to_string()),
        }
        cx.notify();
    }

    pub fn decide(&mut self, accept: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let (Some(request), Some(node)) = (&self.state.incoming, &self.node)
            && let Err(error) = node.handle.decide(&request.id, accept)
        {
            self.state.error = Some(error.to_string());
        }
        self.state.incoming = None;
        self.restore(window, cx);
        cx.notify();
    }

    fn back(&mut self, _: &Back, window: &mut Window, cx: &mut Context<Self>) {
        if self.about_open {
            self.close_about(window, cx);
        } else if self.logs.is_some() {
            self.close_logs(window, cx);
        } else if self.history_expanded {
            self.close_history(window, cx);
        } else if self.preview.take().is_some() {
            self.restore(window, cx);
        } else if self.state.incoming.is_some() {
            self.decide(false, window, cx);
        } else if let Some(active) = self
            .state
            .transfers
            .iter()
            .rev()
            .find(|transfer| transfer.status == TransferStatus::Active)
        {
            if let Some(node) = &self.node
                && let Err(error) = node.handle.cancel(&active.id)
            {
                self.state.error = Some(error.to_string());
            }
        } else {
            self.state.error = None;
            self.focus.focus(window, cx);
        }
        cx.notify();
    }
}

impl Home {
    fn status_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.omarchy().clone();
        let name = self
            .node
            .as_ref()
            .map(|node| node.device.alias.clone())
            .unwrap_or_else(|| gethostname::gethostname().to_string_lossy().into_owned());
        let (status, color) = if self.starting {
            ("Connecting…", theme.secondary)
        } else if self.node.is_none() {
            ("Disconnected", theme.danger)
        } else if self.state.devices.is_empty() {
            ("Searching for devices…", theme.secondary)
        } else {
            ("Connected", theme.success)
        };
        div()
            .id("status-bar")
            .flex()
            .items_center()
            .flex_shrink_0()
            .gap_3()
            .px(rems(PANEL_PADDING))
            .h(rems(1.75))
            .border_t_1()
            .border_color(theme.divider())
            .text_size(rems(0.6875))
            .text_color(theme.secondary)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(name),
            )
            .when(self.update_action_available(), |bar| {
                bar.child(
                    button(
                        "available-update",
                        self.update_status_label(),
                        ButtonVariant::Secondary,
                        cx,
                    )
                    .py_0()
                    .text_color(
                        if matches!(
                            self.update_state,
                            omasend::updates::UpdateState::InstallFailed { .. }
                        ) {
                            theme.danger
                        } else {
                            theme.accent
                        },
                    )
                    .on_click(cx.listener(|view, _, window, cx| view.activate_update(window, cx))),
                )
            })
            .when(
                self.show_update_status && !self.update_action_available(),
                |bar| {
                    bar.child(
                        div()
                            .id("update-status")
                            .min_w_0()
                            .truncate()
                            .text_color(
                                if self.update_state == omasend::updates::UpdateState::Failed {
                                    theme.danger
                                } else {
                                    theme.secondary
                                },
                            )
                            .child(self.update_status_label()),
                    )
                },
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_shrink_0()
                    .child(div().size(rems(0.3)).bg(color))
                    .child(self.language.text(status)),
            )
            .into_any_element()
    }
}

impl Render for Home {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Keep the application and popup controls in the same mono face.
        // Published gpui-omarchy defaults to the platform sans face; preserve
        // any explicit theme font while adapting that fallback for OmaSend.
        if cx.omarchy().font.as_ref() == ".SystemUIFont" {
            cx.global_mut::<gpui_omarchy::Theme>().font = if cfg!(target_os = "macos") {
                "Menlo"
            } else {
                "monospace"
            }
            .into();
        }
        let theme = cx.omarchy().clone();
        let can_send = self.node.is_some()
            && self.state.selected_device().is_some()
            && !self.state.composer.is_empty()
            && !self.state.sending()
            && !self.loading_input;
        let send_label = self.language.text(
            if self.state.transfers.iter().any(|transfer| {
                transfer.sending
                    && transfer.status == TransferStatus::Active
                    && transfer.awaiting_acceptance
            }) {
                "Waiting for receiver"
            } else if self.state.sending() {
                "Sending"
            } else {
                "Send"
            },
        );
        let mut root = focus_scope("omasend")
            .key_context("OmaSend OmarchyFocusScope")
            .track_focus(&self.focus)
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .font_family(theme.font.clone())
            .text_size(rems(0.75))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::open_files))
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(|view, action: &Confirm, window, cx| {
                if view.focus.is_focused(window) {
                    view.send(action, window, cx);
                } else {
                    // Let the focused button or nested control handle Enter.
                    cx.propagate();
                }
            }))
            .on_action(cx.listener(|view, _: &PreviousDevice, window, cx| {
                if view.logs.is_some()
                    || view.about_open
                    || view.preview.is_some()
                    || view.state.incoming.is_some()
                    || view.history_expanded
                {
                    return;
                }
                if !view.focus.is_focused(window) {
                    cx.propagate();
                    return;
                }
                view.state.navigate(-1);
                view.reveal_selected_device(window);
                view.focus.focus(window, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &NextDevice, window, cx| {
                if view.logs.is_some()
                    || view.about_open
                    || view.preview.is_some()
                    || view.state.incoming.is_some()
                    || view.history_expanded
                {
                    return;
                }
                if !view.focus.is_focused(window) {
                    cx.propagate();
                    return;
                }
                view.state.navigate(1);
                view.reveal_selected_device(window);
                view.focus.focus(window, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &CloseWindow, window, cx| {
                view.close_window(window, cx);
            }))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .on_drop(cx.listener(|view, paths: &ExternalPaths, window, cx| {
                view.add_items(
                    paths.0.iter().cloned().map(SendItem::File).collect(),
                    window,
                    cx,
                )
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(rems(PANEL_PADDING))
                    .py_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(super::motion::logo(24., theme.accent))
                            .child(
                                div()
                                    .text_color(theme.bright)
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("Omasend"),
                            ),
                    )
                    .child(menu(
                        "app-menu",
                        button("menu-trigger", "", ButtonVariant::Secondary, cx)
                            .accessibility_label(self.language.text("Menu"))
                            .map(|button| {
                                gpui_omarchy::with_tooltip(button, self.language.text("Menu"))
                            })
                            .p_1()
                            .child(icon(IconName::Menu).size(rems(0.875))),
                        vec![
                            MenuItem::new(self.language.text("Paste"))
                                .shortcut("ctrl+v")
                                .disabled(self.loading_input),
                            MenuItem::new(self.language.text("Add files…"))
                                .shortcut("ctrl+o")
                                .disabled(self.loading_input),
                            MenuItem::new(self.language.text("Language"))
                                .separator_before()
                                .submenu(vec![
                                    (
                                        12,
                                        MenuItem::new("English")
                                            .checked(self.language == omasend::i18n::Language::En),
                                    ),
                                    (
                                        13,
                                        MenuItem::new("简体中文").checked(
                                            self.language == omasend::i18n::Language::ZhCn,
                                        ),
                                    ),
                                ]),
                            MenuItem::new(self.language.text("Theme")).submenu(
                                [
                                    (20, "System", super::theme::ThemeMode::System),
                                    (21, "Light", super::theme::ThemeMode::Light),
                                    (22, "Dark", super::theme::ThemeMode::Dark),
                                ]
                                .into_iter()
                                .map(|(id, label, mode)| {
                                    (
                                        id,
                                        MenuItem::new(self.language.text(label)).checked(
                                            *cx.global::<super::theme::ThemeMode>() == mode,
                                        ),
                                    )
                                })
                                .collect(),
                            ),
                            MenuItem::new(self.language.text("Check for update"))
                                .separator_before(),
                            MenuItem::new(self.language.text("About…")).separator_before(),
                            MenuItem::new("OmaSend…"),
                            MenuItem::new("GitHub…"),
                            MenuItem::new(self.language.text("Logs…")).separator_before(),
                            MenuItem::new(self.language.text("Exit")).separator_before(),
                        ],
                        {
                            let view = cx.entity();
                            move |index, window, cx| {
                                view.update(cx, |view, cx| match index {
                                    0 => view.paste(&Paste, window, cx),
                                    1 => view.open_files(&OpenFiles, window, cx),
                                    12 | 13 => {
                                        view.language = if index == 12 {
                                            omasend::i18n::Language::En
                                        } else {
                                            omasend::i18n::Language::ZhCn
                                        };
                                        cx.notify();
                                    }
                                    6 => cx.open_url("https://huacnlee.github.io/omasend/"),
                                    7 => cx.open_url("https://github.com/huacnlee/omasend"),
                                    8 => view.open_logs(window, cx),
                                    9 => cx.quit(),
                                    5 => view.open_about(window, cx),
                                    4 => view.check_updates_manually(cx),
                                    20..=22 => {
                                        let mode = match index {
                                            21 => super::theme::ThemeMode::Light,
                                            22 => super::theme::ThemeMode::Dark,
                                            _ => super::theme::ThemeMode::System,
                                        };
                                        mode.select(window, cx);
                                        cx.notify();
                                    }
                                    _ => {}
                                });
                            }
                        },
                    )),
            )
            .child(separator(cx));
        if let Some(error) = &self.state.error {
            root = root.child(
                gpui_omarchy::alert(self.language.error(error), gpui_omarchy::Status::Error, cx)
                    .mx_4()
                    .mt_3()
                    .when(self.node.is_none() && !self.starting, |row| {
                        row.child(
                            button(
                                "reconnect",
                                self.language.text("Retry"),
                                ButtonVariant::Outline,
                                cx,
                            )
                            .on_click(cx.listener(|view, _, window, cx| view.connect(window, cx))),
                        )
                    })
                    .child(
                        button("dismiss-error", "", ButtonVariant::Secondary, cx)
                            .accessibility_label(self.language.text("Dismiss"))
                            .map(|button| {
                                gpui_omarchy::with_tooltip(button, self.language.text("Dismiss"))
                            })
                            .p_1()
                            .child(icon(IconName::Close).size(rems(0.875)))
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.state.error = None;
                                cx.notify();
                            })),
                    ),
            );
        }
        root = root
            .child(self.nearby(window, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .px(rems(PANEL_PADDING))
                    .pt(rems(PANEL_PADDING))
                    .pb(rems(PANEL_GAP))
                    .gap(rems(PANEL_GAP))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.bright)
                                            .child(self.language.text("Outbox")),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_ellipsis()
                                            .overflow_hidden()
                                            .text_color(theme.secondary)
                                            .child(
                                                self.state
                                                    .transfers
                                                    .iter()
                                                    .find(|transfer| {
                                                        transfer.sending
                                                            && transfer.status
                                                                == TransferStatus::Active
                                                    })
                                                    .map(|transfer| {
                                                        self.language
                                                            .named("to {name}", &transfer.peer)
                                                    })
                                                    .or_else(|| {
                                                        self.state.selected_device().map(|device| {
                                                            self.language
                                                                .named("to {name}", &device.alias)
                                                        })
                                                    })
                                                    .unwrap_or_else(|| {
                                                        self.language
                                                            .text("Choose a nearby device")
                                                            .into()
                                                    }),
                                            ),
                                    ),
                            )
                            .child(div().flex_shrink_0().text_color(theme.secondary).child(
                                if self.loading_input {
                                    self.language.text("Preparing…").into()
                                } else if self.state.composer.is_empty() {
                                    String::new()
                                } else {
                                    format!(
                                        "{} / {}",
                                        self.language.count(
                                            self.state.composer.len(),
                                            "{count} item",
                                            "{count} items"
                                        ),
                                        size_label(
                                            self.state
                                                .composer
                                                .iter()
                                                .map(|item| item.size())
                                                .sum()
                                        )
                                    )
                                },
                            )),
                    )
                    .child(self.composer(cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        button(
                                            "add-files",
                                            self.language.text("Add files…"),
                                            ButtonVariant::Outline,
                                            cx,
                                        )
                                        .disabled(self.loading_input)
                                        .on_click(
                                            cx.listener(|view, _, window, cx| {
                                                view.open_files(&OpenFiles, window, cx)
                                            }),
                                        ),
                                    )
                                    .child(
                                        button(
                                            "paste-content",
                                            self.language.text("Paste"),
                                            ButtonVariant::Secondary,
                                            cx,
                                        )
                                        .disabled(self.loading_input)
                                        .on_click(
                                            cx.listener(|view, _, window, cx| {
                                                view.paste(&Paste, window, cx)
                                            }),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(self.history_button(cx))
                                    .child(
                                        button("send", "", ButtonVariant::Primary, cx)
                                            .accessibility_label(send_label)
                                            .child(icon(IconName::Send).size(rems(0.875)))
                                            .child(send_label)
                                            .disabled(!can_send)
                                            .on_click(cx.listener(|view, _, window, cx| {
                                                view.send(&Confirm, window, cx)
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(self.status_bar(cx));
        if self.logs.is_some() && self.state.incoming.is_none() {
            root = root.child(self.logs_overlay(window, cx));
        } else if self.history_expanded && self.state.incoming.is_none() && self.preview.is_none() {
            root = root.child(self.history_overlay(window, cx));
        }
        if self.state.incoming.is_some() {
            root = root.child(self.receive_dialog(cx));
        } else if self.about_open {
            root = root.child(self.about_dialog(cx));
        } else if self.preview.is_some() {
            root = root.child(self.preview_dialog(cx));
        }
        root
    }
}

#[cfg(all(test, feature = "ui-tests"))]
mod keyboard_tests {
    use super::*;
    use gpui::{KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext};

    #[gpui::test]
    fn update_restart_preserves_outbox_and_check_does_not_interrupt_install(
        cx: &mut TestAppContext,
    ) {
        with_home(cx, |view, cx| {
            view.update_in(cx, |view, window, cx| {
                view.update_state = omasend::updates::UpdateState::Ready {
                    version: "v9.0.0".into(),
                };
                view.activate_update(window, cx);
                assert!(view.state.error.is_some());
                assert_eq!(view.state.composer.len(), 1);
                assert!(
                    !omasend::updates::RESTART_REQUESTED.load(std::sync::atomic::Ordering::SeqCst)
                );
                view.check_updates_manually(cx);
                assert!(matches!(
                    view.update_state,
                    omasend::updates::UpdateState::Ready { .. }
                ));
                view.update_state = omasend::updates::UpdateState::Installing {
                    downloaded: 50,
                    total: Some(100),
                };
                view.check_updates_manually(cx);
                assert!(matches!(
                    view.update_state,
                    omasend::updates::UpdateState::Installing { downloaded: 50, .. }
                ));
            });
        });
    }

    #[gpui::test]
    fn tab_then_enter_can_remove_a_composer_item(cx: &mut TestAppContext) {
        remove_with_key(cx, "enter");
    }
    #[gpui::test]
    fn tab_then_space_can_remove_a_composer_item(cx: &mut TestAppContext) {
        remove_with_key(cx, "space");
    }
    fn with_home(
        cx: &mut TestAppContext,
        test: impl FnOnce(gpui::Entity<Home>, &mut gpui::VisualTestContext),
    ) {
        cx.update(|cx| {
            gpui_omarchy::init(cx);
            super::super::init(cx);
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let handle = runtime.handle().clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            let focus = cx.focus_handle();
            focus.focus(window, cx);
            let mut state = AppState::default();
            state
                .composer
                .push(ComposerItem::new(SendItem::Text("test".into())).unwrap());
            Home {
                state,
                language: omasend::i18n::Language::En,
                node: None,
                runtime: handle,
                window_handle: Some(window.window_handle()),
                focus,
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
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        test(view, cx);
    }

    #[gpui::test]
    fn explicit_theme_and_system_appearance_switch_immediately(cx: &mut TestAppContext) {
        with_home(cx, |_, cx| {
            use super::super::theme::ThemeMode;
            cx.update(|window, cx| {
                assert!(*cx.global::<ThemeMode>() == ThemeMode::System);
                ThemeMode::Light.apply(window, cx);
                assert_eq!(cx.omarchy().appearance, gpui_base::ThemeAppearance::Light);
                ThemeMode::Dark.apply(window, cx);
                assert_eq!(cx.omarchy().appearance, gpui_base::ThemeAppearance::Dark);
                ThemeMode::System.apply(window, cx);
                if !cfg!(target_os = "linux") {
                    let light = matches!(
                        window.appearance(),
                        gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight
                    );
                    assert_eq!(
                        cx.omarchy().appearance == gpui_base::ThemeAppearance::Light,
                        light
                    );
                }
            });
        });
    }

    #[gpui::test]
    fn theme_flyout_pointer_selection_keeps_parent_and_dispatches_choice(cx: &mut TestAppContext) {
        struct MenuHarness {
            selected: Option<usize>,
        }
        impl Render for MenuHarness {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let view = cx.entity();
                menu(
                    "theme-test",
                    button("trigger", "Menu", ButtonVariant::Secondary, cx),
                    vec![MenuItem::new("Theme").submenu(vec![
                        (9, MenuItem::new("System").checked(true)),
                        (10, MenuItem::new("Light").checked(false)),
                        (11, MenuItem::new("Dark").checked(false)),
                    ])],
                    move |index, _, cx| {
                        view.update(cx, |view, cx| {
                            view.selected = Some(index);
                            cx.notify();
                        })
                    },
                )
            }
        }
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(|_, _| MenuHarness { selected: None });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(
            gpui::point(gpui::px(15.), gpui::px(12.)),
            Default::default(),
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let parent = cx.debug_bounds("omarchy-menu-content").unwrap();
        cx.simulate_click(
            parent.origin + gpui::point(gpui::px(24.), gpui::px(20.)),
            Default::default(),
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            cx.debug_bounds("omarchy-menu-content").is_some(),
            "parent menu stays visible"
        );
        let child = cx
            .debug_bounds("omarchy-submenu-content")
            .expect("Theme must open its flyout");
        cx.simulate_click(
            child.origin + gpui::point(gpui::px(24.), gpui::px(50.)),
            Default::default(),
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(view.read_with(cx, |view, _| view.selected), Some(10));
        assert!(cx.debug_bounds("omarchy-menu-content").is_none());
    }

    #[gpui::test]
    fn about_dialog_traps_keys_and_restores_focus(cx: &mut TestAppContext) {
        with_home(cx, |view, cx| {
            cx.simulate_keystrokes("tab");
            let trigger = cx.update(|window, cx| window.focused(cx).unwrap());
            view.update_in(cx, |view, window, cx| view.open_about(window, cx));
            for key in ["tab", "right", "left", "ctrl-v", "ctrl-o"] {
                cx.simulate_keystrokes(key);
                cx.update(|window, cx| {
                    let view = view.read(cx);
                    assert!(view.about_open);
                    assert!(view.modal_focus.contains_focused(window, cx));
                    assert_eq!(view.state.composer.len(), 1);
                });
            }
            cx.simulate_keystrokes("escape");
            cx.update(|window, cx| {
                assert!(!view.read(cx).about_open);
                assert_eq!(window.focused(cx), Some(trigger));
            });
        });
    }

    fn remove_with_key(cx: &mut TestAppContext, activation: &str) {
        with_home(cx, |view, cx| {
            // Tab walks to the menu and then the item's Remove button.
            cx.simulate_keystrokes("tab tab");
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let key = Keystroke::parse(activation).unwrap();
            cx.simulate_event(KeyDownEvent {
                keystroke: key.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            cx.simulate_event(KeyUpEvent { keystroke: key });
            view.read_with(cx, |view, _| {
                assert!(
                    view.state.composer.is_empty(),
                    "The focused Remove button must handle activation"
                )
            });
        });
    }

    #[gpui::test]
    fn preview_keeps_arrow_keys_and_restores_focus(cx: &mut TestAppContext) {
        modal_keeps_arrow_keys(cx, false);
    }

    #[gpui::test]
    fn receive_dialog_keeps_arrow_keys_and_restores_focus(cx: &mut TestAppContext) {
        modal_keeps_arrow_keys(cx, true);
    }

    fn modal_keeps_arrow_keys(cx: &mut TestAppContext, incoming: bool) {
        with_home(cx, |view, cx| {
            cx.simulate_keystrokes("tab tab");
            let trigger = cx.update(|window, cx| window.focused(cx).unwrap());
            view.update_in(cx, |view, window, cx| {
                for name in ["a", "b"] {
                    view.state
                        .apply(TransferEvent::DeviceFound(omasend::localsend::Device {
                            fingerprint: name.into(),
                            alias: name.into(),
                            model: "test".into(),
                            host: "127.0.0.1".into(),
                            port: 53317,
                        }));
                }
                view.state.selected = Some("a".into());
                if incoming {
                    view.state.apply(TransferEvent::IncomingRequest {
                        id: "incoming".into(),
                        peer: view.state.devices[0].clone(),
                        files: vec![omasend::localsend::OfferedFile {
                            name: "test.txt".into(),
                            size: 4,
                            mime: "text/plain".into(),
                        }],
                    });
                } else {
                    view.preview = Some(view.state.composer[0].id.clone());
                }
                view.restore_focus = window.focused(cx);
                view.modal_focus.focus(window, cx);
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            for key in ["tab", "tab", "shift-tab", "right", "left", "down", "up"] {
                cx.simulate_keystrokes(key);
                view.read_with(cx, |view, _| {
                    assert_eq!(
                        view.state.selected.as_deref(),
                        Some("a"),
                        "{key} changed a device behind the preview"
                    );
                });
                cx.update(|window, cx| {
                    assert!(
                        view.read(cx).modal_focus.contains_focused(window, cx),
                        "{key} escaped the modal"
                    );
                });
            }
            cx.simulate_keystrokes("escape");
            cx.update(|window, cx| {
                assert!(view.read(cx).preview.is_none());
                assert!(view.read(cx).state.incoming.is_none());
                assert_eq!(window.focused(cx), Some(trigger.clone()));
            });
        });
    }
    #[gpui::test]
    fn menu_language_can_be_changed_with_keyboard(cx: &mut TestAppContext) {
        with_home(cx, |view, cx| {
            for key in ["tab", "enter", "down", "down", "right", "down", "enter"] {
                let keystroke = Keystroke::parse(key).unwrap();
                cx.simulate_event(KeyDownEvent {
                    keystroke: keystroke.clone(),
                    is_held: false,
                    prefer_character_input: false,
                });
                cx.simulate_event(KeyUpEvent { keystroke });
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
            view.read_with(cx, |view, _| {
                assert_eq!(view.language, omasend::i18n::Language::ZhCn);
                assert_eq!(view.state.composer.len(), 1);
            });
        });
    }
    #[gpui::test]
    fn theme_submenu_returns_to_parent_without_activating_a_choice(cx: &mut TestAppContext) {
        with_home(cx, |view, cx| {
            for key in [
                "tab", "enter", "down", "down", "down", "right", "left", "home", "down", "down",
                "right", "down", "enter",
            ] {
                let keystroke = Keystroke::parse(key).unwrap();
                cx.simulate_event(KeyDownEvent {
                    keystroke: keystroke.clone(),
                    is_held: false,
                    prefer_character_input: false,
                });
                cx.simulate_event(KeyUpEvent { keystroke });
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
            view.read_with(cx, |view, _| {
                assert_eq!(view.language, omasend::i18n::Language::ZhCn);
                assert_eq!(view.state.composer.len(), 1);
            });
        });
    }
    #[gpui::test]
    fn home_arrows_select_devices(cx: &mut TestAppContext) {
        with_home(cx, |view, cx| {
            view.update_in(cx, |view, _, cx| {
                for name in ["a", "b"] {
                    view.state
                        .apply(TransferEvent::DeviceFound(omasend::localsend::Device {
                            fingerprint: name.into(),
                            alias: name.into(),
                            model: "test".into(),
                            host: "127.0.0.1".into(),
                            port: 53317,
                        }));
                }
                view.state.selected = Some("a".into());
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            for (key, selected) in [("right", "b"), ("left", "a"), ("down", "b"), ("up", "a")] {
                cx.simulate_keystrokes(key);
                view.read_with(cx, |view, _| {
                    assert_eq!(view.state.selected.as_deref(), Some(selected))
                });
            }
        });
    }
    #[gpui::test]
    fn expanded_devices_scroll_to_keyboard_selection(cx: &mut TestAppContext) {
        with_home(cx, |view, cx| {
            view.update_in(cx, |view, _, cx| {
                view.nearby_expanded = true;
                for index in 0..40 {
                    view.state
                        .apply(TransferEvent::DeviceFound(omasend::localsend::Device {
                            fingerprint: format!("{index:02}"),
                            alias: format!("Device {index:02}"),
                            model: "test".into(),
                            host: "127.0.0.1".into(),
                            port: 53317,
                        }));
                }
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            for _ in 0..39 {
                cx.simulate_keystrokes("right");
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
            view.read_with(cx, |view, _| {
                assert_eq!(view.state.selected.as_deref(), Some("39"));
                assert_eq!(
                    view.nearby_scroll.is_scrolled_to_end(),
                    Some(true),
                    "scroll state: {:?}",
                    view.nearby_scroll.0.borrow()
                );
            });
        });
    }
    #[gpui::test]
    fn history_dock_scrolls_all_records_and_restores_keyboard_focus(cx: &mut TestAppContext) {
        with_home(cx, |view, cx| {
            // Use a real focusable control as the restoration target.
            cx.simulate_keystrokes("tab tab");
            let trigger = cx.update(|window, cx| window.focused(cx).unwrap());
            view.update_in(cx, |view, window, cx| {
                for index in 0..12 {
                    let id = format!("history-{index}");
                    view.state.apply(TransferEvent::Started {
                        id: id.clone(),
                        peer: "Nearby peer".into(),
                        sending: false,
                        files: vec![omasend::localsend::OfferedFile {
                            name: format!("record-{index}.txt"),
                            size: 4,
                            mime: "text/plain".into(),
                        }],
                        total: 4,
                    });
                    if index % 2 == 0 {
                        view.state.apply(TransferEvent::Failed {
                            id,
                            error: "Transfer failed".into(),
                        });
                    } else {
                        view.state.apply(TransferEvent::Completed {
                            id,
                            paths: vec![std::path::PathBuf::from(format!(
                                "/tmp/record-{index}.txt"
                            ))],
                        });
                    }
                }
                for name in ["a", "b"] {
                    view.state
                        .apply(TransferEvent::DeviceFound(omasend::localsend::Device {
                            fingerprint: name.into(),
                            alias: name.into(),
                            model: "test".into(),
                            host: "127.0.0.1".into(),
                            port: 53317,
                        }));
                }
                view.state.selected = Some("a".into());
                view.open_history(window, cx);
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            view.read_with(cx, |view, _| {
                let scroll = &view.history_scroll;
                assert!(
                    scroll.max_offset().y > gpui::px(0.),
                    "fixture must overflow the dock"
                );
                assert_eq!(
                    scroll.bottom_item(),
                    11,
                    "the newest record belongs inside the common scroller"
                );
                assert!(
                    scroll.bounds_for_item(12).is_none(),
                    "each transfer renders exactly once in the dock"
                );
                assert_eq!(
                    scroll.offset().y,
                    -scroll.max_offset().y,
                    "opening starts at the bottom"
                );
                for index in 0..11 {
                    let current = scroll.bounds_for_item(index).unwrap();
                    let next = scroll.bounds_for_item(index + 1).unwrap();
                    assert!(
                        current.bottom() <= next.top(),
                        "records must retain chronological layout order"
                    );
                }
            });
            for key in ["tab", "tab", "shift-tab", "right", "left", "down", "up"] {
                cx.simulate_keystrokes(key);
                cx.update(|window, cx| {
                    assert!(
                        view.read(cx).modal_focus.contains_focused(window, cx),
                        "{key} escaped history focus"
                    );
                    assert_eq!(
                        view.read(cx).state.selected.as_deref(),
                        Some("a"),
                        "{key} changed the device behind history"
                    );
                    assert_eq!(view.read(cx).state.composer.len(), 1);
                });
            }
            cx.simulate_keystrokes("escape");
            cx.update(|window, cx| {
                assert!(!view.read(cx).history_expanded);
                assert_eq!(window.focused(cx), Some(trigger.clone()));
            });
            view.update_in(cx, |view, window, cx| view.open_history(window, cx));
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.simulate_click(
                gpui::point(gpui::px(10.), gpui::px(10.)),
                Default::default(),
            );
            cx.update(|window, cx| {
                assert!(
                    !view.read(cx).history_expanded,
                    "backdrop click closes the sheet"
                );
                assert_eq!(window.focused(cx), Some(trigger.clone()));
            });
        });
    }
    #[gpui::test]
    fn logs_panel_copies_raw_text_and_restores_focus(cx: &mut TestAppContext) {
        with_home(cx, |view, cx| {
            cx.simulate_keystrokes("tab tab");
            let trigger = cx.update(|window, cx| window.focused(cx).unwrap());
            view.update_in(cx, |view, window, cx| {
                view.open_logs(window, cx);
                let logs = view.logs.as_mut().unwrap();
                // Freeze the live backend while exercising exact viewer content.
                logs.refresh = None;
                logs.text = "WARN local discovery failed\nDEBUG retrying peer".into();
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.simulate_keystrokes("tab");
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let enter = Keystroke::parse("enter").unwrap();
            cx.simulate_event(KeyDownEvent {
                keystroke: enter.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            cx.simulate_event(KeyUpEvent { keystroke: enter });
            cx.update(|window, cx| {
                let clipboard = cx.read_from_clipboard().expect("Copy logs writes text");
                let text = clipboard
                    .into_entries()
                    .filter_map(|entry| match entry {
                        gpui::ClipboardEntry::String(value) => Some(value.text().clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                assert_eq!(text, "WARN local discovery failed\nDEBUG retrying peer");
                assert!(view.read(cx).modal_focus.contains_focused(window, cx));
            });
            for key in ["tab", "shift-tab", "right", "left", "ctrl-v"] {
                cx.simulate_keystrokes(key);
                cx.update(|window, cx| {
                    assert!(view.read(cx).logs.is_some());
                    assert!(
                        view.read(cx).modal_focus.contains_focused(window, cx),
                        "{key} escaped logs"
                    );
                    assert_eq!(
                        view.read(cx).state.composer.len(),
                        1,
                        "{key} changed the composer behind logs"
                    );
                });
            }
            cx.simulate_keystrokes("escape");
            cx.update(|window, cx| {
                assert!(view.read(cx).logs.is_none());
                assert_eq!(window.focused(cx), Some(trigger.clone()));
            });
        });
    }
}
