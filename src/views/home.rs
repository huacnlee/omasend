use super::*;
use anyhow::{Context as _, Result};
use gpui_omarchy::gpui::{
    self, Context, ExternalPaths, FocusHandle, PathPromptOptions, Window, div, prelude::*, rems,
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
    pub logo: std::sync::Arc<gpui::Image>,
    pub language: omasend::i18n::Language,
    pub node: Option<Node>,
    pub runtime: tokio::runtime::Handle,
    pub focus: FocusHandle,
    pub modal_focus: FocusHandle,
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
            logo: std::sync::Arc::new(gpui::Image::from_bytes(
                gpui::ImageFormat::Png,
                include_bytes!("../../assets/omasend.png").to_vec(),
            )),
            language: omasend::i18n::Language::system(),
            node: None,
            runtime,
            focus,
            modal_focus: cx.focus_handle(),
            restore_focus: None,
            preview: None,
            loading_input: false,
            starting: false,
        };
        view.connect(window, cx);
        view
    }

    fn connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.starting || self.node.is_some() {
            return;
        }
        self.starting = true;
        let task = self
            .runtime
            .spawn(async { Node::start(NodeConfig::desktop()?).await });
        cx.spawn_in(window, async move |this, cx| {
            let result = task
                .await
                .context("Network task stopped")
                .and_then(|result| result);
            match result {
                Ok(node) => {
                    let events = node.events.clone();
                    if this
                        .update_in(cx, |view, _, cx| {
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
                            .update_in(cx, |view, window, cx| {
                                let was_incoming = view.state.incoming.is_some();
                                if matches!(&event, TransferEvent::IncomingRequest { .. }) {
                                    view.preview = None;
                                    view.restore_focus = window.focused(cx);
                                    view.modal_focus.focus(window, cx);
                                }
                                view.state.apply(event);
                                if was_incoming && view.state.incoming.is_none() {
                                    view.restore(window, cx);
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
                    let _ = this.update_in(cx, |view, _, cx| {
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

    pub fn add_items(&mut self, items: Vec<SendItem>, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading_input {
            return;
        }
        self.loading_input = true;
        let task = self.runtime.spawn_blocking(move || {
            items
                .into_iter()
                .map(ComposerItem::new)
                .collect::<Result<Vec<_>>>()
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task
                .await
                .context("File preparation stopped")
                .and_then(|result| result);
            let _ = this.update_in(cx, |view, window, cx| {
                view.loading_input = false;
                match result {
                    Ok(items) => {
                        view.state.composer.extend(items);
                        view.state.error = None;
                    }
                    Err(error) => view.state.error = Some(format!("{error:#}")),
                }
                view.focus.focus(window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading_input {
            return;
        }
        #[cfg(target_os = "linux")]
        {
            self.loading_input = true;
            let task = self.runtime.spawn(omasend::clipboard::wayland::paste());
            cx.spawn_in(window, async move |this, cx| {
                let result = task
                    .await
                    .context("Clipboard task stopped")
                    .and_then(|result| result);
                let _ = this.update_in(cx, |view, window, cx| {
                    view.loading_input = false;
                    match result {
                        Ok(items) => view.add_items(items, window, cx),
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
                self.add_items(paths, window, cx);
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
                    Ok(items) => self.add_items(items, window, cx),
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
                self.add_items(items, window, cx);
            }
        }
        cx.notify();
    }

    pub(super) fn open_files(
        &mut self,
        _: &OpenFiles,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.loading_input {
            return;
        }
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some(self.language.text("Add to OmaSend").into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = prompt.await;
            let _ = this.update_in(cx, |view, window, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        view.add_items(paths.into_iter().map(SendItem::File).collect(), window, cx)
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
        if self.preview.take().is_some() {
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

impl Render for Home {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let send_label = self.language.text(if self.state.sending() {
            "Sending"
        } else {
            "Send"
        });
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
                view.state.navigate(-1);
                view.focus.focus(window, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &NextDevice, window, cx| {
                view.state.navigate(1);
                view.focus.focus(window, cx);
                cx.notify();
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
                    .px_4()
                    .py_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(super::motion::discovery_logo(
                                self.logo.clone(),
                                self.state.discovering && !cx.reduce_motion(),
                                self.state.discovery_revision,
                            ))
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
                            .p_1()
                            .child(icon(IconName::Menu).size(rems(0.875))),
                        vec![
                            MenuItem::new(self.language.text("Paste"))
                                .shortcut("ctrl+v")
                                .disabled(self.loading_input),
                            MenuItem::new(self.language.text("Add files…"))
                                .shortcut("ctrl+o")
                                .disabled(self.loading_input),
                            MenuItem::new("English")
                                .checked(self.language == omasend::i18n::Language::En)
                                .separator_before(),
                            MenuItem::new("简体中文")
                                .checked(self.language == omasend::i18n::Language::ZhCn),
                            MenuItem::new(self.language.text("Exit")).separator_before(),
                        ],
                        {
                            let view = cx.entity();
                            move |index, window, cx| {
                                view.update(cx, |view, cx| match index {
                                    0 => view.paste(&Paste, window, cx),
                                    1 => view.open_files(&OpenFiles, window, cx),
                                    2 | 3 => {
                                        view.language = if index == 2 {
                                            omasend::i18n::Language::En
                                        } else {
                                            omasend::i18n::Language::ZhCn
                                        };
                                        cx.notify();
                                    }
                                    4 => cx.quit(),
                                    _ => {}
                                });
                            }
                        },
                    )),
            )
            .child(separator(cx));
        if let Some(error) = &self.state.error {
            root = root.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .bg(theme.surface)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(theme.danger)
                            .child(self.language.error(error)),
                    )
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
                        button(
                            "dismiss-error",
                            self.language.text("Dismiss"),
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.state.error = None;
                            cx.notify();
                        })),
                    ),
            );
        }
        root = root
            .child(self.nearby(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .px_4()
                    .py_3()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(self.language.text("Outbox")),
                            )
                            .child(div().text_color(theme.secondary).child(
                                if self.loading_input {
                                    self.language.text("Preparing…").into()
                                } else if self.state.composer.is_empty() {
                                    self.language.text("Nothing added yet").into()
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
                    .child(
                        div().text_color(theme.secondary).child(
                            self.state
                                .selected_device()
                                .map(|device| self.language.named("To {name}", &device.alias))
                                .unwrap_or_else(|| {
                                    self.language.text("Choose a nearby device").into()
                                }),
                        ),
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
                                    .gap_2()
                                    .child(
                                        button(
                                            "add-files",
                                            self.language.text("Add files"),
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
                                button("send", send_label, ButtonVariant::Primary, cx)
                                    .disabled(!can_send)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.send(&Confirm, window, cx)
                                    })),
                            ),
                    ),
            )
            .child(self.transfers(cx));
        if self.state.incoming.is_some() {
            root = root.child(self.receive_dialog(cx));
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
    fn tab_then_enter_can_remove_a_composer_item(cx: &mut TestAppContext) {
        remove_with_key(cx, "enter");
    }
    #[gpui::test]
    fn tab_then_space_can_remove_a_composer_item(cx: &mut TestAppContext) {
        remove_with_key(cx, "space");
    }
    fn remove_with_key(cx: &mut TestAppContext, activation: &str) {
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
                logo: std::sync::Arc::new(gpui::Image::from_bytes(
                    gpui::ImageFormat::Png,
                    include_bytes!("../../assets/omasend.png").to_vec(),
                )),
                language: omasend::i18n::Language::En,
                node: None,
                runtime: handle,
                focus,
                modal_focus: cx.focus_handle(),
                restore_focus: None,
                preview: None,
                loading_input: false,
                starting: false,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
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
                "Enter must activate Remove rather than the background send action"
            )
        });
    }
}
