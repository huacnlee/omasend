use gpui::{
    App, ClickEvent, Context, Entity, FocusHandle, FontWeight, KeyDownEvent, Window, WindowOptions,
    div, prelude::*, px, size,
};
#[cfg(not(target_family = "wasm"))]
use gpui::{Bounds, WindowBounds};
use gpui_base::CheckboxState;
use gpui_omarchy::*;
#[cfg(not(target_family = "wasm"))]
use std::time::Instant as GalleryInstant;
#[cfg(target_family = "wasm")]
use web_time::Instant as GalleryInstant;

const GROUPS: &[(&str, &[&str])] = &[
    ("Explore", &["overview"]),
    (
        "Actions",
        &["button", "button_group", "link", "toggle", "toggle_group"],
    ),
    (
        "Forms",
        &[
            "input",
            "textarea",
            "number_input",
            "select",
            "combobox",
            "calendar",
            "date_picker",
            "color_picker",
            "otp_input",
            "slider",
            "checkbox",
            "switch",
            "radio",
        ],
    ),
    (
        "Navigation",
        &[
            "menu",
            "tabs",
            "accordion",
            "collapsible",
            "nav_stack",
            "pagination",
        ],
    ),
    (
        "Overlays",
        &[
            "sheet",
            "dialog",
            "alert_dialog",
            "popover",
            "tooltip",
            "hover_card",
            "toast",
        ],
    ),
    (
        "Display",
        &[
            "icon",
            "avatar",
            "text_view",
            "panel",
            "virtual_list",
            "scrollbar",
            "table",
            "tree",
            "resizable",
            "dock",
            "separator",
            "keycap",
            "badge",
            "empty_state",
            "progress",
        ],
    ),
];

fn components() -> impl Iterator<Item = &'static str> {
    GROUPS.iter().flat_map(|(_, items)| items.iter().copied())
}

fn display_name(page: &str) -> String {
    let label = page.replace('_', " ");
    let mut letters = label.chars();
    letters.next().map_or(String::new(), |first| {
        first.to_uppercase().collect::<String>() + letters.as_str()
    })
}

fn description(page: &str) -> &'static str {
    match page {
        "overview" => "Explore the components together, then use the sidebar to inspect each one.",
        "button_group" => "Choose one setting from a row of mutually exclusive options.",
        "collapsible" => "Expand a single region for additional settings.",
        "toast" => "Brief feedback that leaves your current task in place.",
        "popover" => "Adjust contextual settings while keeping the workspace in view.",
        "tooltip" => "A short explanation for an action, shown on hover.",
        "select" => "Choose one value from a fixed set of options.",
        "combobox" => "Search a collection, then choose a matching option.",
        "menu" => "An anchored action menu with one pointer and keyboard cursor.",
        "dialog" => "A focused task with confirmation, cancellation and focus return.",
        "alert_dialog" => "An explicit decision that cannot be dismissed by clicking the backdrop.",
        "button" => "Content-sized actions with quiet hover, focus and pressed states.",
        "input" => "Single-line editing with selection, clipboard and IME support.",
        "textarea" => "Multi-line notes with native text editing.",
        "number_input" => "A compact numeric field with keyboard and button stepping.",
        "slider" => "Adjust one value or a range with the pointer or keyboard.",
        "checkbox" => "Independent choices, including a mixed selection.",
        "switch" => "An immediate on/off choice with a visible track and thumb.",
        "radio" => "Choose one option from a group.",
        "tabs" => "Switch between related views while keeping context.",
        "accordion" => "Reveal supporting content when it is needed.",
        "pagination" => "Move through a paged collection.",
        "table" => "Aligned columns for comparing records.",
        "color_picker" => "Choose a label color with Hex and HSLA controls.",
        "text_view" => "Read structured documents with selectable text and links.",
        "sheet" => "Inspect project details in an edge-attached panel.",
        "scrollbar" => "Drag the scroll thumb to move through a long activity log.",
        "virtual_list" => "Browse a large activity log with variable-height rows.",
        "nav_stack" => "Navigate between persistent pages and return to where you left off.",
        "dock" => "Rearrange document panels by dragging their tabs.",
        "tree" => "Explore nested folders and select a workspace document.",
        "resizable" => "Drag the divider to adjust space between panes.",
        "hover_card" => "Preview supporting details without leaving the page.",
        "otp_input" => "Enter a six-digit verification code.",
        "date_picker" => "Choose a date from a calendar anchored to a field.",
        "calendar" => "Choose a date with month and year navigation.",
        "avatar" => "Identify people and workspaces with a square image or initials.",
        "icon" => "Monochrome SVG icons that inherit the surrounding text color.",
        "panel" => "A surface for a related group of settings or information.",
        "separator" => "A quiet boundary between distinct sections.",
        "keycap" => "Compact, readable keyboard hints.",
        "badge" => "Short labels for neutral and semantic status.",
        "empty_state" => "Explain an empty collection and its next step.",
        "progress" => "Show how much of a known task is complete.",
        "toggle_group" => "Combine independent filters to show more than one status.",
        "toggle" => "Keep a command active until it is pressed again.",
        "link" => "Open a named destination.",
        _ => "",
    }
}

struct Gallery {
    workspace_name: Entity<gpui_base::input::InputState>,
    workspace_draft: Entity<gpui_base::input::InputState>,
    saved_workspace: String,
    theme_mode: usize,
    toolbar_position: usize,
    select_choice: Entity<ChoiceState>,
    combo_choice: Entity<ChoiceState>,
    disabled_choice: Entity<ChoiceState>,
    modal_open: bool,
    modal_focus: FocusHandle,
    modal_trigger: FocusHandle,
    modal_result: String,
    menu_result: String,
    slider_value: Entity<gpui_base::slider::SliderState>,
    slider_range: Entity<gpui_base::slider::SliderState>,
    slider_disabled: Entity<gpui_base::slider::SliderState>,
    navigation_focus: FocusHandle,
    navigation_list: gpui::ListState,
    navigation_rows: Vec<(bool, &'static str)>,
    expanded: [bool; 3],
    collapse_open: bool,
    toast_message: Option<&'static str>,
    toast_saved: bool,
    toast_lifecycle: gpui_base::ToastManager<u8, &'static str>,
    toast_timer: Option<gpui::Task<()>>,
    toast_hovered: bool,
    toast_focused: bool,
    toast_focus: FocusHandle,
    current_page: usize,
    number: Entity<gpui_base::input::InputState>,
    input: Entity<gpui_base::input::InputState>,
    textarea: Entity<gpui_base::input::TextareaState>,
    page: &'static str,
    count: usize,
    checked: bool,
    mixed: bool,
    enabled: bool,
    choice: usize,
    radio_focus: FocusHandle,
    sheet_open: bool,
    sheet_focus: FocusHandle,
    sheet_trigger: FocusHandle,
    activity_scroll: gpui_base::VirtualListScrollHandle,
    timeline_scroll: gpui::ScrollHandle,
    activity_sizes: std::rc::Rc<Vec<gpui::Size<gpui::Pixels>>>,
    activity_rendered: usize,
    pressed: bool,
    article_filters: [bool; 3],
    tab: usize,
    progress: f32,
    calendar_state: Entity<gpui_base::CalendarState>,
    tree_state: Entity<gpui_base::TreeState>,
    otp_state: Entity<gpui_base::OtpState>,
    otp_disabled: bool,
    color_state: Entity<gpui_base::ColorPickerState>,
    nav_state: Entity<gpui_base::NavStackState>,
    date_picker_state: Entity<DatePickerState>,
    dock_state: Entity<gpui_base::dock::DockArea>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let number = cx.new(|cx| gpui_base::input::InputState::new(window, cx).default_value("1"));
        let input =
            cx.new(|cx| gpui_base::input::InputState::new(window, cx).placeholder("Project name"));
        let textarea = cx.new(|cx| {
            gpui_base::input::TextareaState::new(window, cx)
                .rows(4)
                .placeholder("Add notes")
        });
        let slider_value = cx.new(|_| {
            gpui_base::slider::SliderState::new()
                .step(5.)
                .default_value(40.)
        });
        let slider_range = cx.new(|_| {
            gpui_base::slider::SliderState::new()
                .step(5.)
                .default_value((20., 80.))
        });
        let slider_disabled = cx.new(|_| gpui_base::slider::SliderState::new().default_value(60.));
        for state in [&slider_value, &slider_range] {
            cx.observe(state, |_, _, cx| cx.notify()).detach();
        }
        let workspace_name = cx.new(|cx| {
            gpui_base::input::InputState::new(window, cx).default_value("Personal workspace")
        });
        let workspace_draft = cx.new(|cx| {
            gpui_base::input::InputState::new(window, cx).default_value("Personal workspace")
        });
        for state in [&workspace_name, &workspace_draft] {
            cx.observe(state, |_, _, cx| cx.notify()).detach();
        }
        let options = || {
            vec![
                ChoiceItem::new("personal", "Personal workspace"),
                ChoiceItem::new("team", "Team workspace"),
                ChoiceItem::new("archive", "Archived workspace").disabled(true),
                ChoiceItem::new("sandbox", "Sandbox"),
            ]
        };
        let select_choice = cx.new(|cx| {
            ChoiceState::new(options(), window, cx)
                .label("Default workspace")
                .default_selected(0)
        });
        let combo_choice = cx.new(|cx| {
            ChoiceState::new(options(), window, cx)
                .label("Find a workspace")
                .placeholder("Find a workspace…")
        });
        let disabled_choice = cx.new(|cx| {
            ChoiceState::new(options(), window, cx)
                .label("Managed workspace")
                .default_selected(1)
                .disabled(true)
        });
        for state in [&select_choice, &combo_choice] {
            cx.observe(state, |_, _, cx| cx.notify()).detach();
        }
        let calendar_state =
            cx.new(|cx| gpui_base::CalendarState::new(window, cx).disabled_matcher(vec![0, 6]));
        cx.observe(&calendar_state, |_, _, cx| cx.notify()).detach();
        let tree_state = cx.new(|cx| {
            gpui_base::TreeState::new(cx).items(vec![
                gpui_base::TreeItem::new("documents", "Documents")
                    .expanded(true)
                    .child(gpui_base::TreeItem::new("brief", "Project brief.md"))
                    .child(gpui_base::TreeItem::new("notes", "Meeting notes.md")),
                gpui_base::TreeItem::new("projects", "Projects")
                    .expanded(true)
                    .child(
                        gpui_base::TreeItem::new("website", "Website")
                            .child(gpui_base::TreeItem::new("homepage", "Homepage.md"))
                            .child(gpui_base::TreeItem::new("assets", "Assets.md")),
                    ),
                gpui_base::TreeItem::new("archive", "Archive (unavailable)").disabled(true),
            ])
        });
        cx.observe(&tree_state, |_, _, cx| cx.notify()).detach();
        let otp_state = cx.new(|cx| gpui_base::OtpState::new(6, window, cx));
        cx.observe(&otp_state, |_, _, cx| cx.notify()).detach();
        let accent = cx.omarchy().accent;
        let color_state =
            cx.new(|cx| gpui_base::ColorPickerState::new(window, cx).default_value(accent));
        cx.observe(&color_state, |_, _, cx| cx.notify()).detach();
        let nav_state = cx.new(|_| gpui_base::NavStackState::new());
        let task_page = cx.new(|cx| NavigationPage {
            level: 2,
            next: None,
            navigation: nav_state.downgrade(),
            notes: cx.new(|cx| {
                gpui_base::input::InputState::new(window, cx)
                    .default_value("Check the narrow window layout before Friday.")
            }),
        });
        let project_page = cx.new(|cx| NavigationPage {
            level: 1,
            next: Some(task_page),
            navigation: nav_state.downgrade(),
            notes: cx.new(|cx| gpui_base::input::InputState::new(window, cx)),
        });
        let root_page = cx.new(|cx| NavigationPage {
            level: 0,
            next: Some(project_page),
            navigation: nav_state.downgrade(),
            notes: cx.new(|cx| gpui_base::input::InputState::new(window, cx)),
        });
        nav_state.update(cx, |state, cx| {
            state.push(root_page, gpui_base::NavMotion::Immediate, cx)
        });
        cx.observe(&nav_state, |_, _, cx| cx.notify()).detach();
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        cx.observe(&date_picker_state, |_, _, cx| cx.notify())
            .detach();
        let dock_state = cx.new(|cx| dock_area("gallery-workspace", window, cx));
        let layout = demo_dock_layout(cx);
        dock_state.update(cx, |state, cx| state.set_center(layout, window, cx));
        Self {
            dock_state,
            date_picker_state,
            color_state,
            nav_state,
            otp_state,
            otp_disabled: false,
            tree_state,
            calendar_state,
            select_choice,
            combo_choice,
            disabled_choice,
            workspace_name,
            workspace_draft,
            saved_workspace: "Personal workspace".into(),
            theme_mode: 0,
            toolbar_position: 0,
            modal_open: false,
            modal_focus: cx.focus_handle(),
            modal_trigger: cx.focus_handle(),
            modal_result: String::new(),
            menu_result: "No action selected".into(),
            slider_value,
            slider_range,
            slider_disabled,
            navigation_focus: cx.focus_handle(),
            navigation_list: gpui::ListState::new(
                GROUPS.len() + components().count(),
                gpui::ListAlignment::Top,
                px(100.),
            ),
            navigation_rows: GROUPS
                .iter()
                .flat_map(|(group, items)| {
                    std::iter::once((true, *group)).chain(items.iter().map(|&name| (false, name)))
                })
                .collect(),
            expanded: [true, false, false],
            collapse_open: false,
            toast_message: None,
            toast_saved: false,
            toast_lifecycle: gpui_base::ToastManager::new(gpui_base::ToastMotion {
                duration: std::time::Duration::ZERO,
                exit_duration: std::time::Duration::ZERO,
                ..Default::default()
            }),
            toast_timer: None,
            toast_hovered: false,
            toast_focused: false,
            toast_focus: cx.focus_handle(),
            current_page: 1,
            number,
            input,
            textarea,
            page: initial_page(),
            count: 0,
            checked: true,
            mixed: false,
            enabled: true,
            choice: 0,
            radio_focus: cx.focus_handle(),
            sheet_open: false,
            sheet_focus: cx.focus_handle(),
            sheet_trigger: cx.focus_handle(),
            activity_scroll: gpui_base::VirtualListScrollHandle::new(),
            timeline_scroll: gpui::ScrollHandle::new(),
            activity_sizes: std::rc::Rc::new(
                (0..1000)
                    .map(|index| size(px(400.), px(if index % 5 == 0 { 44. } else { 28. })))
                    .collect(),
            ),
            activity_rendered: 0,
            pressed: false,
            article_filters: [true, true, false],
            tab: 0,
            progress: 30.,
        }
    }
    fn save_workspace(&mut self, modal: bool, window: &mut Window, cx: &mut Context<Self>) {
        let value = if modal {
            self.workspace_draft.read(cx)
        } else {
            self.workspace_name.read(cx)
        }
        .value()
        .to_string();
        if value.trim().is_empty() {
            self.modal_result = "Enter a workspace name".into();
        } else {
            self.saved_workspace = value.trim().into();
            if modal {
                let value = self.saved_workspace.clone();
                self.workspace_name
                    .update(cx, |state, cx| state.set_value(value, window, cx));
            }
            self.modal_result = format!("Saved “{}”", self.saved_workspace);
        }
        cx.notify();
    }

    fn reset_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.saved_workspace = "Personal workspace".into();
        self.workspace_name.update(cx, |state, cx| {
            state.set_value("Personal workspace", window, cx)
        });
        self.modal_result = "Workspace defaults restored".into();
        cx.notify();
    }

    fn workspace_form(
        &self,
        modal: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let state = if modal {
            &self.workspace_draft
        } else {
            &self.workspace_name
        };
        let valid = !state.read(cx).value().trim().is_empty();
        div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(dialog_title("Workspace settings", cx))
                    .child(dialog_description(
                        "Give this workspace a name you can recognize.",
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child("Name")
                    .child(input(
                        if modal { "modal-name" } else { "specimen-name" },
                        state,
                        window,
                        cx,
                    ))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.secondary)
                            .child(if valid {
                                "Shown in the workspace switcher."
                            } else {
                                "Enter a workspace name"
                            })
                            .text_color(if valid { t.secondary } else { t.danger }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .py(px(8.))
                    .child(icon(IconName::Settings).text_color(t.secondary))
                    .child(
                        div()
                            .text_color(t.secondary)
                            .child(format!("Current workspace: {}", self.saved_workspace)),
                    ),
            )
            .child(separator(cx))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        dialog_button(
                            if modal {
                                "modal-cancel"
                            } else {
                                "specimen-cancel"
                            },
                            "Cancel",
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                if modal {
                                    window
                                        .dispatch_action(Box::new(gpui_base::actions::Cancel), cx);
                                } else {
                                    let value = this.saved_workspace.clone();
                                    this.workspace_name
                                        .update(cx, |state, cx| state.set_value(value, window, cx));
                                    this.modal_result = "Changes discarded".into();
                                    cx.notify();
                                }
                            },
                        )),
                    )
                    .child(
                        dialog_button(
                            if modal {
                                "modal-confirm"
                            } else {
                                "specimen-confirm"
                            },
                            "Save",
                            ButtonVariant::Primary,
                            cx,
                        )
                        .disabled(!valid)
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                if modal {
                                    window.dispatch_action(
                                        Box::new(gpui_base::actions::Confirm { secondary: false }),
                                        cx,
                                    );
                                } else {
                                    this.save_workspace(false, window, cx);
                                }
                            },
                        )),
                    ),
            )
    }

    fn reset_form(&self, modal: bool, cx: &mut Context<Self>) -> gpui::Div {
        let t = cx.omarchy();
        div().flex().flex_col().gap(px(18.))
            .child(div().flex().items_center().gap(px(10.))
                .child(icon(IconName::TriangleAlert).size(px(20.)).text_color(t.danger))
                .child(dialog_title("Reset workspace settings?", cx)))
            .child(dialog_description(format!("The custom name “{}” will be replaced with “Personal workspace”. Your files will stay in place.", self.saved_workspace), cx))
            .child(div().flex().justify_end().gap(px(8.))
                .child(dialog_button(if modal { "modal-cancel" } else { "specimen-cancel" }, "Cancel", ButtonVariant::Secondary, cx)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if modal { window.dispatch_action(Box::new(gpui_base::actions::Cancel), cx); }
                        else { this.modal_result = "Workspace unchanged".into(); cx.notify(); }
                    })))
                .child(dialog_button(if modal { "modal-confirm" } else { "specimen-confirm" }, "Reset", ButtonVariant::Danger, cx)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if modal { window.dispatch_action(Box::new(gpui_base::actions::Confirm { secondary: false }), cx); }
                        else { this.reset_workspace(window, cx); }
                    }))))
    }
}

impl Gallery {
    fn render_toggle_group(&self, mut content: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        let t = cx.omarchy().clone();

        let labels = ["Draft", "In review", "Published"];
        let mut filters = toggle_group("article-status", cx).aria_label("Article status filters");
        for (index, label) in labels.into_iter().enumerate() {
            filters = filters.child(
                toggle(index, label, self.article_filters[index], cx)
                    .debug_selector(move || {
                        ["article-filter-0", "article-filter-1", "article-filter-2"][index].into()
                    })
                    .on_change(change(cx.listener(move |this, next, _, cx| {
                        this.article_filters[index] = *next;
                        cx.notify();
                    }))),
            );
        }
        content = content.child("Article status").child(filters).child(
            div()
                .text_color(t.secondary)
                .child("Select any combination. Turn every filter off to show no articles."),
        );
        let mut count = 0;
        for (index, (title, status)) in [
            ("Autumn release notes", 0),
            ("Getting started", 1),
            ("Keyboard shortcuts", 2),
            ("Workspace migration", 0),
        ]
        .into_iter()
        .enumerate()
        {
            if !self.article_filters[status] {
                continue;
            }
            count += 1;
            content = content.child(
                div()
                    .debug_selector(move || format!("filtered-article-{index}"))
                    .flex()
                    .flex_wrap()
                    .justify_between()
                    .gap(px(8.))
                    .py(px(10.))
                    .border_b_1()
                    .border_color(t.divider())
                    .child(title)
                    .child(div().text_color(t.secondary).child(labels[status])),
            );
        }
        content = content.child(
            div()
                .text_color(t.secondary)
                .child(format!("{count} articles shown")),
        );
        if count == 0 {
            content = content.child(
                button(
                    "show-all-articles",
                    "Show all statuses",
                    ButtonVariant::Outline,
                    cx,
                )
                .debug_selector(|| "show-all-articles".into())
                .on_click(cx.listener(|this, _, _, cx| {
                    this.article_filters = [true; 3];
                    cx.notify();
                })),
            );
        }

        content
    }
}

// Separate page builders keep the gallery render stack bounded in debug builds.
impl Gallery {
    fn render_overview_example(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let form = self.workspace_form(false, window, cx);
        content = content
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .gap(px(24.))
                    .child(div().w(px(420.)).max_w(gpui::relative(1.)).child(form))
                    .child(
                        div()
                            .w(px(220.))
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Preferences"),
                            )
                            .child(
                                checkbox(
                                    "overview-hidden",
                                    "Show hidden files",
                                    if self.checked {
                                        CheckboxState::Checked
                                    } else {
                                        CheckboxState::Unchecked
                                    },
                                    cx,
                                )
                                .on_change(change(cx.listener(
                                    |this, state, _, cx| {
                                        this.checked = *state == CheckboxState::Checked;
                                        cx.notify();
                                    },
                                ))),
                            )
                            .child(
                                switch("overview-sync", "Sync enabled", self.enabled, cx)
                                    .on_change(change(cx.listener(|this, value, _, cx| {
                                        this.enabled = *value;
                                        cx.notify();
                                    }))),
                            )
                            .child(separator(cx))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Theme"),
                            )
                            .child(div().text_color(t.secondary).child(t.name.clone()))
                            .child(
                                div()
                                    .text_color(t.secondary)
                                    .child("Settings in this gallery stay in memory."),
                            ),
                    ),
            )
            .child(div().mt(px(18.)).w_full().child(separator(cx)))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::BOLD)
                    .child("Explore components"),
            )
            .child(
                div().flex().flex_wrap().gap(px(8.)).children(
                    [
                        ("button", "Buttons"),
                        ("input", "Text fields"),
                        ("menu", "Menus"),
                        ("dialog", "Dialogs"),
                    ]
                    .into_iter()
                    .map(|(page, label)| {
                        dialog_button(
                            (gpui::ElementId::from("overview-open"), page),
                            label,
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.page = page;
                            cx.notify();
                        }))
                    }),
                ),
            );
        content
    }
    fn render_popover_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let target = cx.entity();
        content = content
            .child(
                div()
                    .w(px(360.))
                    .max_w(gpui::relative(1.))
                    .p(px(14.))
                    .bg(t.normal_fill())
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(self.saved_workspace.clone())
                    .child("Documents")
                    .child("Projects")
                    .when(self.checked, |preview| {
                        preview.child(div().text_color(t.secondary).child(".config"))
                    }),
            )
            .child(popover(
                "display-options",
                button(
                    "display-options-trigger",
                    "Display options",
                    ButtonVariant::Secondary,
                    cx,
                )
                .child(icon(IconName::ChevronDown).size(px(14.))),
                move |_, _, cx| {
                    let checked = target.read(cx).checked;
                    let enabled = target.read(cx).enabled;
                    let hidden_target = target.clone();
                    let sync_target = target.clone();
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .child(div().font_weight(FontWeight::BOLD).child("Display options"))
                        .child(
                            checkbox(
                                "popover-hidden",
                                "Show hidden files",
                                if checked {
                                    CheckboxState::Checked
                                } else {
                                    CheckboxState::Unchecked
                                },
                                cx,
                            )
                            .on_change(move |value, _, _, cx| {
                                hidden_target.update(cx, |this, cx| {
                                    this.checked = value == CheckboxState::Checked;
                                    cx.notify();
                                })
                            }),
                        )
                        .child(
                            switch("popover-sync", "Sync workspace", enabled, cx).on_change(
                                move |value, _, _, cx| {
                                    sync_target.update(cx, |this, cx| {
                                        this.enabled = value;
                                        cx.notify();
                                    })
                                },
                            ),
                        )
                },
            ))
            .child(div().text_color(t.secondary).child(if self.enabled {
                "Workspace sync is enabled."
            } else {
                "Workspace sync is paused."
            }));
        content
    }
    fn render_tooltip_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let action = if self.pressed {
            "Remove from favorites"
        } else {
            "Add to favorites"
        };
        content = content
            .child(
                div()
                    .w(px(360.))
                    .max_w(gpui::relative(1.))
                    .p(px(14.))
                    .bg(t.normal_fill())
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(self.saved_workspace.clone())
                            .child(div().text_color(t.secondary).child(if self.pressed {
                                "In favorites"
                            } else {
                                "Not in favorites"
                            })),
                    )
                    .child(with_tooltip(
                        button("tooltip-favorite", "", ButtonVariant::Secondary, cx)
                            .accessibility_label(action)
                            .selected(self.pressed)
                            .child(icon(IconName::Star))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pressed = !this.pressed;
                                cx.notify();
                            })),
                        action,
                    )),
            )
            .child(
                div()
                    .text_color(t.secondary)
                    .child("Hover the star for its action. Tab and Return also activate it."),
            )
            .child(div().mt(px(14.)).child("Tooltip appearance"))
            .child(tooltip("Add to favorites", cx));
        content
    }
    fn render_menu_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let target = cx.entity();
        content = content
            .child(menu(
                "workspace-menu",
                button(
                    "menu-trigger",
                    "Workspace actions",
                    ButtonVariant::Secondary,
                    cx,
                )
                .styles(|styles| styles.selected(|style| style.bg(t.border)))
                .child(icon(IconName::ChevronDown).text_color(t.foreground)),
                vec![
                    MenuItem::new("New workspace").icon(IconName::Plus),
                    MenuItem::new("Favorite workspace").icon(IconName::Star),
                    MenuItem::new("Unavailable action").disabled(true),
                ],
                move |index, _, cx| {
                    target.update(cx, |this, cx| {
                        this.menu_result =
                            ["New workspace", "Favorite workspace", "Unavailable action"][index]
                                .into();
                        cx.notify();
                    })
                },
            ))
            .child(self.menu_result.clone());
        content
    }
    fn render_table_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let records = [
            (
                "Website refresh",
                "Active",
                Status::Success,
                "Alex Lee",
                "Sep 7",
                "12 files",
            ),
            (
                "Design system",
                "Active",
                Status::Success,
                "Morgan Kim",
                "Sep 6",
                "28 files",
            ),
            (
                "Release notes",
                "Review",
                Status::Warning,
                "Sam Rivera",
                "Sep 5",
                "4 files",
            ),
            (
                "Desktop client",
                "Active",
                Status::Success,
                "Alex Lee",
                "Sep 4",
                "36 files",
            ),
            (
                "Onboarding",
                "Review",
                Status::Warning,
                "Morgan Kim",
                "Sep 3",
                "9 files",
            ),
            (
                "Research archive",
                "Archived",
                Status::Neutral,
                "Sam Rivera",
                "Aug 28",
                "42 files",
            ),
            (
                "API reference",
                "Active",
                Status::Success,
                "Alex Lee",
                "Aug 26",
                "17 files",
            ),
            (
                "Brand assets",
                "Archived",
                Status::Neutral,
                "Morgan Kim",
                "Aug 21",
                "24 files",
            ),
        ];
        let mut heading = table_row("heading", 1, cx);
        for (index, title) in ["Project", "Status", "Owner", "Updated", "Files"]
            .into_iter()
            .enumerate()
        {
            heading = heading.child(table_head(index, index + 1, cx).child(title));
        }
        content = content
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child("Workspace projects")
                    .child(
                        div()
                            .text_color(t.secondary)
                            .child("8 projects · Sample data"),
                    ),
            )
            .child(
                div()
                    .id("project-table-scroll")
                    .w_full()
                    .overflow_x_scroll()
                    .child(
                        table("projects", cx)
                            .min_w(px(640.))
                            .child(gpui_base::TableHeader::new("head").child(heading))
                            .child(gpui_base::TableBody::new("body").children(
                                records.into_iter().enumerate().map(
                                    |(index, (name, label, status, owner, updated, files))| {
                                        table_row(index, index + 2, cx)
                                            .child(
                                                table_cell("name", 1, cx)
                                                    .child(div().truncate().child(name)),
                                            )
                                            .child(
                                                table_cell("status", 2, cx).child(
                                                    div()
                                                        .text_color(match status {
                                                            Status::Neutral => t.secondary,
                                                            Status::Success => t.success,
                                                            Status::Warning => t.warning,
                                                            Status::Error => t.danger,
                                                        })
                                                        .child(label),
                                                ),
                                            )
                                            .child(table_cell("owner", 3, cx).child(owner))
                                            .child(table_cell("updated", 4, cx).child(updated))
                                            .child(table_cell("files", 5, cx).child(files))
                                    },
                                ),
                            )),
                    ),
            );
        content
    }
    fn render_button_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content.child(div().text_color(t.secondary).child("Appearance"));
        for (key, label, variant) in [
            ("primary", "Primary", ButtonVariant::Primary),
            ("outline", "Outline", ButtonVariant::Outline),
            ("secondary", "Secondary", ButtonVariant::Secondary),
            ("danger", "Danger", ButtonVariant::Danger),
        ] {
            content = content.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(div().w(px(90.)).text_color(t.secondary).child(label))
                    .child(button(key, "Apply", variant, cx).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.count += 1;
                            cx.notify();
                        },
                    )))
                    .child(
                        button(
                            (gpui::ElementId::from(key), "disabled"),
                            "Unavailable",
                            variant,
                            cx,
                        )
                        .disabled(true),
                    ),
            );
        }
        content = content
            .child(div().text_color(t.secondary).child("Icons and actions"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        button("icon-action", "", ButtonVariant::Outline, cx)
                            .accessibility_label("Add workspace")
                            .child(icon(IconName::Plus).size(px(14.)))
                            .child("Add workspace")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            })),
                    )
                    .child(with_tooltip(
                        button("icon-only", "", ButtonVariant::Outline, cx)
                            .accessibility_label("Add workspace")
                            .child(icon(IconName::Plus).size(px(14.)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            })),
                        "Add workspace",
                    ))
                    .child(
                        button("icon-disabled", "", ButtonVariant::Outline, cx)
                            .accessibility_label("Add workspace unavailable")
                            .child(icon(IconName::Plus).size(px(14.)))
                            .disabled(true),
                    )
                    .child(
                        button("reset-count", "Reset", ButtonVariant::Secondary, cx).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.count = 0;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .text_color(t.secondary)
                    .child(format!("Activations: {}", self.count)),
            );
        content
    }
    fn render_tabs_example(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let labels = ["Overview", "Activity", "Settings"];
        let target = cx.entity();
        let strip = tab_list(
            "workspace-tabs",
            labels
                .iter()
                .map(|&label| ChoiceItem::new(label, label))
                .collect(),
            Some(self.tab),
            move |index, _, cx| {
                target.update(cx, |this, cx| {
                    this.tab = index;
                    cx.notify();
                })
            },
            window,
            cx,
        );
        let mut body = div()
            .id("workspace-tab-panel")
            .role(gpui::Role::TabPanel)
            .flex()
            .flex_col()
            .gap(px(14.))
            .p(px(14.))
            .w_full()
            .min_h(px(240.))
            .bg(t.normal_fill());
        match self.tab {
            0 => {
                body = body
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Personal workspace"),
                    )
                    .child(
                        div()
                            .text_color(t.secondary)
                            .child("Your files and projects, organized in one place."),
                    );
                for (name, detail) in [
                    ("Documents", "Notes and reference material"),
                    ("Projects", "Active work and experiments"),
                    ("Archive", "Completed projects"),
                ] {
                    body = body.child(
                        div()
                            .flex()
                            .justify_between()
                            .gap(px(14.))
                            .border_b_1()
                            .border_color(t.divider())
                            .pb(px(10.))
                            .child(name)
                            .child(div().text_color(t.secondary).child(detail)),
                    );
                }
            }
            1 => {
                body = body
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Recent activity"),
                    )
                    .child(
                        div()
                            .text_color(t.secondary)
                            .child("Sample workspace history"),
                    );
                for (event, time) in [
                    ("Updated project notes", "Today · 09:42"),
                    ("Added a reference document", "Today · 09:15"),
                    ("Archived a completed project", "Yesterday · 16:30"),
                ] {
                    body = body.child(
                        div()
                            .flex()
                            .justify_between()
                            .gap(px(14.))
                            .border_b_1()
                            .border_color(t.divider())
                            .pb(px(10.))
                            .child(event)
                            .child(div().text_color(t.secondary).child(time)),
                    );
                }
            }
            _ => {
                body = body
                    .child("Workspace name")
                    .child(
                        input("tab-workspace-name", &self.workspace_name, window, cx)
                            .max_w(px(360.)),
                    )
                    .child(
                        switch("tab-sync", "Sync workspace", self.enabled, cx)
                            .debug_selector(|| "tab-sync".into())
                            .on_change(change(cx.listener(|this, next, _, cx| {
                                this.enabled = *next;
                                cx.notify();
                            }))),
                    )
                    .child(div().text_color(t.secondary).child(
                        "Changes apply immediately and remain available when you switch tabs.",
                    ));
            }
        }
        content = content
            .child(strip.aria_label("Workspace pages"))
            .child(body);
        content
    }
    fn render_resizable_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content.child("Horizontal").child(div().w_full().h(px(300.)).border_1().border_color(t.border)
                    .child(resizable("workspace-panes", gpui::Axis::Horizontal, cx)
                        .child(resizable_panel().size(px(220.)).size_range(px(140.)..px(420.))
                            .child(div().size_full().p(px(14.)).flex().flex_col().gap(px(10.))
                                .child("Documents").child("Project brief.md").child("Meeting notes.md")))
                        .child(resizable_panel().size_range(px(180.)..px(1600.))
                            .child(div().size_full().p(px(14.)).flex().flex_col().gap(px(14.))
                                .child(div().font_weight(FontWeight::BOLD).child("Project brief"))
                                .child("A focused desktop workspace for files, notes and project activity.")
                                .child(div().text_color(t.secondary).child("Drag the divider to give this preview more room."))))));
        content = content.child("Vertical").child(
            div()
                .w_full()
                .h(px(240.))
                .border_1()
                .border_color(t.border)
                .child(
                    resizable("preview-console", gpui::Axis::Vertical, cx)
                        .child(
                            resizable_panel()
                                .size(px(140.))
                                .size_range(px(80.)..px(180.))
                                .child(
                                    div()
                                        .size_full()
                                        .p(px(14.))
                                        .flex()
                                        .flex_col()
                                        .gap(px(10.))
                                        .child("Preview")
                                        .child("The workspace is ready for review."),
                                ),
                        )
                        .child(
                            resizable_panel().size_range(px(60.)..px(160.)).child(
                                div()
                                    .size_full()
                                    .p(px(14.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(10.))
                                    .child("Activity")
                                    .child(
                                        div()
                                            .text_color(t.secondary)
                                            .child("All changes saved · No pending tasks"),
                                    ),
                            ),
                        ),
                ),
        );
        content
    }
    fn render_avatar_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content.child(
            div()
                .font_weight(FontWeight::BOLD)
                .child("Workspace members"),
        );
        for (initials, name, role) in [
            ("HL", "huacnlee", "Owner"),
            ("MK", "Morgan Kim", "Editor"),
            ("SR", "Sam Rivera", "Viewer"),
        ] {
            content = content.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .pb(px(10.))
                    .border_b_1()
                    .border_color(t.divider())
                    .child(avatar(initials, cx).when(initials == "HL", |avatar| {
                        avatar.image(avatar_image(gallery_avatar()))
                    }))
                    .child(div().flex_1().child(name))
                    .child(div().text_color(t.secondary).child(role)),
            );
        }
        content = content
            .child(div().text_color(t.secondary).child("Sizes"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .child(
                        avatar("HL", cx)
                            .image(avatar_image(gallery_avatar()))
                            .size(px(24.))
                            .text_size(px(10.)),
                    )
                    .child(avatar("HL", cx).image(avatar_image(gallery_avatar())))
                    .child(
                        avatar("HL", cx)
                            .image(avatar_image(gallery_avatar()))
                            .size(px(48.))
                            .text_size(px(16.)),
                    ),
            );
        content
    }
    fn render_nav_stack_example(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                button("nav-back", "Back", ButtonVariant::Outline, cx)
                                    .debug_selector(|| "nav-back".into())
                                    .disabled(self.nav_state.read(cx).depth() <= 1)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.nav_state.update(cx, |state, cx| {
                                            state.pop(gpui_base::NavMotion::Immediate, cx);
                                        });
                                    })),
                            )
                            .child(
                                button("nav-forward", "Forward", ButtonVariant::Outline, cx)
                                    .debug_selector(|| "nav-forward".into())
                                    .disabled(self.nav_state.read(cx).forward_views().len() == 0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.nav_state.update(cx, |state, cx| {
                                            state.forward(gpui_base::NavMotion::Immediate, cx);
                                        });
                                    })),
                            )
                            .child(div().text_color(t.secondary).child(
                                match self.nav_state.read(cx).depth() {
                                    1 => "Projects",
                                    2 => "Projects / Website refresh",
                                    _ => "Projects / Website refresh / Review homepage",
                                },
                            )),
                    )
                    .child(nav_stack(&self.nav_state, cx).h(px(340.)))
                    .child(div().text_color(t.secondary).child(
                        "Try it: open Website refresh, open its task, edit the note, then go Back and Forward. Your note stays on the task page.",
                    ));
        content
    }
}

impl Gallery {
    fn render_progress_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content
            .child(format!("Progress: {:.0}%", self.progress))
            .child(progress("progress", self.progress, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        button("advance", "Advance 10%", ButtonVariant::Primary, cx).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.progress = (this.progress + 10.).min(100.);
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        button("reset", "Reset", ButtonVariant::Secondary, cx).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.progress = 0.;
                                cx.notify();
                            }),
                        ),
                    ),
            );
        content
    }
    fn render_empty_state_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = if self.count == 0 {
            content.child(
                empty_state(
                    "No workspaces",
                    "Create a workspace to collect your projects.",
                    cx,
                )
                .child(
                    button("create", "Create workspace", ButtonVariant::Primary, cx).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.count = 1;
                            cx.notify();
                        }),
                    ),
                ),
            )
        } else {
            content.child("Workspace created").child(
                button("reset-empty", "Reset example", ButtonVariant::Secondary, cx).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.count = 0;
                        cx.notify();
                    }),
                ),
            )
        };
        content
    }
    fn render_badge_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        for (label, status) in [
            ("Idle", Status::Neutral),
            ("Applied", Status::Success),
            ("Needs attention", Status::Warning),
            ("Failed", Status::Error),
        ] {
            content = content.child(badge(label, status, cx));
        }
        content
    }
    fn render_keycap_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(keycap("Tab", cx))
                .child("Next control")
                .child(keycap("Return", cx))
                .child("Activate"),
        );
        content
    }
    fn render_separator_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
            .child(div().text_color(t.secondary).child("Horizontal"))
            .child("Appearance")
            .child(separator(cx))
            .child("Keyboard")
            .child(div().mt(px(14.)).text_color(t.secondary).child("Vertical"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        button("separator-add", "Add", ButtonVariant::Secondary, cx).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(vertical_separator(cx))
                    .child(
                        button("separator-reset", "Reset", ButtonVariant::Secondary, cx).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.count = 0;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(vertical_separator(cx))
                    .child(format!("{} items", self.count)),
            );
        content
    }
    fn render_panel_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content.child(
            panel("Workspace", cx)
                .child("Theme-aware surface with a title and composable children")
                .child(badge("Ready", Status::Success, cx)),
        );
        content
    }
    fn render_calendar_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
            .child("Schedule a workspace review")
            .child(calendar("review-calendar", &self.calendar_state, cx))
            .child(
                div()
                    .text_color(t.secondary)
                    .child("Choose a weekday. Weekends are unavailable."),
            )
            .child(
                self.calendar_state
                    .read(cx)
                    .date()
                    .format("%A, %B %e, %Y")
                    .unwrap_or_else(|| "No date selected".into()),
            )
            .child(
                button("clear-date", "Clear date", ButtonVariant::Outline, cx).on_click(
                    cx.listener(|this, _, window, cx| {
                        this.calendar_state.update(cx, |state, cx| {
                            state.set_date(gpui_base::Date::Single(None), window, cx)
                        });
                    }),
                ),
            );
        content
    }
    fn render_date_picker_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
            .child("Review date")
            .child(date_picker("review-date", &self.date_picker_state, cx))
            .child(
                div()
                    .text_color(t.secondary)
                    .child("Select a day to confirm. Escape cancels the calendar."),
            );
        content
    }
    fn render_color_picker_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let color = self.color_state.read(cx).value().unwrap_or(t.accent);
        content = content.child("Project label color")
                    .child(color_picker("project-color", &self.color_state, window, cx))
                    .child(div().flex().items_center().gap(px(10.))
                        .child(div().size(px(20.)).bg(color))
                        .child("Website refresh"))
                    .child(div().text_color(t.secondary).child("The swatch shows the committed label color. Escape discards an unconfirmed Hex edit."));
        content
    }
    fn render_tree_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
            .child("Workspace files")
            .child(tree(&self.tree_state, cx).max_w(px(440.)))
            .child(
                div().text_color(t.secondary).child(
                    self.tree_state
                        .read(cx)
                        .selected_item()
                        .map(|item| format!("Selected: {}", item.label))
                        .unwrap_or_else(|| "Select a file or folder".into()),
                ),
            )
            .child(
                div()
                    .text_color(t.secondary)
                    .child("Arrow keys navigate · Left / Right collapse and expand"),
            );
        content
    }
    fn render_dock_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
                    .child(button("reset-dock", "Reset layout", ButtonVariant::Outline, cx)
                        .on_click(cx.listener(|this, _, window, cx| {
                            let layout = demo_dock_layout(cx);
                            this.dock_state.update(cx, |state, cx| state.set_center(layout, window, cx));
                        })))
                    .child(div().w_full().h(px(360.)).child(self.dock_state.clone()))
                    .child(div().text_color(t.secondary).child("Drag tabs to merge or split panes. Drag a divider to resize. Reset layout restores all three panels."));
        content
    }
    fn render_hover_card_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
            .child("Workspace owner")
            .child(hover_card(
                "member-preview",
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(avatar("AL", cx))
                    .child("Alex Lee"),
                |_, _, cx| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(div().font_weight(FontWeight::BOLD).child("Alex Lee"))
                        .child("Design engineer · Workspace owner")
                        .child(
                            div()
                                .text_color(cx.omarchy().secondary)
                                .child("Maintains the design system and reviews desktop releases."),
                        )
                },
            ))
            .child(
                div()
                    .text_color(t.secondary)
                    .child("Hover over the member to preview their profile."),
            );
        content
    }
    fn render_otp_input_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
            .child("Verification code")
            .child(otp_input(&self.otp_state, window, cx).disabled(self.otp_disabled))
            .child(div().text_color(t.secondary).child(
                "Demo only — no code is sent. Type or paste six digits; Backspace removes a digit.",
            ))
            .child(if self.otp_state.read(cx).value().len() == 6 {
                "Code complete"
            } else {
                "Waiting for six digits"
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        button("reset-otp", "Reset code", ButtonVariant::Outline, cx)
                            .disabled(self.otp_disabled)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.otp_state.update(cx, |state, cx| {
                                    state.set_value("", window, cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        button(
                            "toggle-otp-disabled",
                            if self.otp_disabled {
                                "Enable input"
                            } else {
                                "Disable input"
                            },
                            ButtonVariant::Outline,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.otp_disabled = !this.otp_disabled;
                            cx.notify();
                        })),
                    ),
            );
        content
    }
    fn render_button_group_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let labels = ["Top", "Right", "Bottom", "Left"];
        let target = cx.entity();
        let group = button_group(
            "toolbar-position",
            labels
                .iter()
                .map(|&label| ChoiceItem::new(label, label))
                .collect(),
            Some(self.toolbar_position),
            move |index, _, cx| {
                target.update(cx, |this, cx| {
                    this.toolbar_position = index;
                    cx.notify();
                })
            },
            window,
            cx,
        );
        content = content
            .child("Toolbar position")
            .child(group.aria_label("Toolbar position"))
            .child(div().text_color(t.secondary).child(format!(
                "Toolbar is placed at the {}.",
                labels[self.toolbar_position].to_lowercase()
            )))
            .child(
                div()
                    .text_color(t.secondary)
                    .child("Left / Right move the cursor · Return / Space choose"),
            );
        content
    }
    fn render_link_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content
            .child(link(
                "manual",
                "Open Omarchy manual",
                "https://omarchy.org/manual",
                cx,
            ))
            .child(
                link(
                    "disabled-link",
                    "Unavailable link",
                    "https://omarchy.org",
                    cx,
                )
                .disabled(true),
            );
        content
    }
    fn render_toggle_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content.child(
            toggle("toggle", "Favorite panel", self.pressed, cx)
                .child(icon(IconName::Star))
                .on_change(change(cx.listener(|this, next, _, cx| {
                    this.pressed = *next;
                    cx.notify();
                }))),
        );
        content
    }
    fn render_radio_page(
        &mut self,
        content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = cx.omarchy().clone();
        let mut options = gpui_base::RadioGroup::new("list-density")
            .aria_label("List density")
            .track_focus(&self.radio_focus.clone().tab_stop(true))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                this.choice = match event.keystroke.key.as_str() {
                    "down" | "right" | "j" | "l" => (this.choice + 1) % 3,
                    "up" | "left" | "k" | "h" => (this.choice + 2) % 3,
                    _ => return,
                };
                cx.stop_propagation();
                cx.notify();
            }))
            .flex()
            .flex_col()
            .gap(px(4.));
        for (index, label) in ["Compact", "Comfortable", "Spacious"]
            .into_iter()
            .enumerate()
        {
            options = options.child(
                radio(index, label, self.choice == index, cx)
                    .set_position(index + 1, 3)
                    .tab_stop(false)
                    .when(
                        self.radio_focus.is_focused(window) && self.choice == index,
                        |row| {
                            row.aria_active_descendant()
                                .bg(theme.hover_fill())
                                .border_color(theme.focus_border())
                        },
                    )
                    .debug_selector(move || format!("density-option-{index}"))
                    .on_change(change(cx.listener(move |this, _, window, cx| {
                        this.choice = index;
                        this.radio_focus.focus(window, cx);
                        cx.notify();
                    }))),
            );
        }
        let row_height = [28., 36., 44.][self.choice];
        let mut preview = div()
            .w_full()
            .max_w(px(420.))
            .border_1()
            .border_color(theme.border)
            .flex()
            .flex_col();
        for (index, (name, status)) in [
            ("Project brief", "Updated today"),
            ("Meeting notes", "Updated yesterday"),
            ("Release checklist", "Draft"),
        ]
        .into_iter()
        .enumerate()
        {
            preview = preview.child(
                div()
                    .debug_selector(move || format!("density-preview-{index}"))
                    .h(px(row_height))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .when(index > 0, |row| row.border_t_1().border_color(theme.border))
                    .child(name)
                    .child(div().text_color(theme.secondary).child(status)),
            );
        }
        content
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child("List density")
                    .child(options),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child("Preview")
                    .child(preview),
            )
    }
    fn render_switch_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content
            .child(
                switch("switch", "Show metadata", self.enabled, cx).on_change(change(cx.listener(
                    |this, next, _, cx| {
                        this.enabled = *next;
                        cx.notify();
                    },
                ))),
            )
            .child(if self.enabled {
                "Metadata visible"
            } else {
                "Metadata hidden"
            });
        content
    }
    fn render_checkbox_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let state = if self.mixed {
            CheckboxState::Indeterminate
        } else if self.checked {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        content = content
            .child(
                checkbox("check", "Include hidden files", state, cx).on_change(change(
                    cx.listener(|this, next, _, cx| {
                        this.checked = *next == CheckboxState::Checked;
                        this.mixed = false;
                        cx.notify();
                    }),
                )),
            )
            .child(
                checkbox(
                    "check-disabled",
                    "Managed by policy",
                    CheckboxState::Checked,
                    cx,
                )
                .disabled(true),
            )
            .child(
                button("mixed", "Set mixed state", ButtonVariant::Secondary, cx).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.mixed = true;
                        cx.notify();
                    }),
                ),
            );
        content
    }
    fn render_textarea_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content
            .child("Workspace notes")
            .child(textarea("notes", &self.textarea, window, cx).max_w(px(520.)))
            .child("Supports multiple lines, selection, clipboard and IME");
        content
    }
    fn render_input_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content
            .child("Workspace name")
            .child(input("workspace-name", &self.input, window, cx).max_w(px(380.)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        button("submit-input", "Read value", ButtonVariant::Primary, cx).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.count = this.input.read(cx).value().chars().count();
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        button("reset-input", "Reset value", ButtonVariant::Outline, cx)
                            .debug_selector(|| "reset-input".into())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.input.update(cx, |state, cx| {
                                    state.set_value("", window, cx);
                                    state.focus(window, cx);
                                });
                                this.count = 0;
                                cx.notify();
                            })),
                    ),
            )
            .child(format!("Submitted length: {} characters", self.count));
        content
    }
    fn render_number_input_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content
            .child("Quantity")
            .child(number_input(&self.number, cx))
            .child("Arrow Up / Arrow Down  adjust by 1");
        content
    }
    fn render_pagination_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let listener = cx.listener(|this, page, _, cx| {
            this.current_page = *page;
            cx.notify();
        });
        let state = gpui_base::PaginationState::new(self.current_page, 12)
            .on_change(move |page, window, cx| listener(&page, window, cx));
        content = content
            .child(format!("Page {} of 12", self.current_page))
            .child(pagination("pages", state, cx));
        content
    }
    fn render_accordion_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut sections = accordion("sections", cx);
        for (index, (title, description)) in [
                    ("Appearance", "Uses the current Omarchy system theme, with Tokyo Night as the fallback. Change the theme from the application menu."),
                    ("Keyboard navigation", "Tab moves between controls. Return or Space activates the focused control. Escape closes an open menu or dialog."),
                    ("Workspace data", "Changes in this gallery stay in memory for this session. Resetting an example does not remove files on disk."),
                ].into_iter().enumerate() {
                    sections = sections.child(gpui_base::AccordionItem::new().open(self.expanded[index])
                        .header(gpui_base::AccordionHeader::new(accordion_trigger(("section", index), title, self.expanded[index], cx)
                            .debug_selector(move || format!("accordion-trigger-{index}"))
                            .on_change(change(cx.listener(move |this, next, _, cx| { this.expanded[index] = *next; cx.notify(); })))))
                        .panel(accordion_panel(cx).child(div().debug_selector(move || format!("accordion-panel-{index}")).child(description))));
                }
        content = content.child(sections);
        content
    }
    fn advance_toast(&mut self, now: GalleryInstant, cx: &mut Context<Self>) {
        let change = self
            .toast_lifecycle
            .advance(now, self.toast_hovered || self.toast_focused);
        if change.changed {
            self.toast_message = self
                .toast_lifecycle
                .iter()
                .next_back()
                .map(|(_, message, _)| *message);
            cx.notify();
        }
    }

    fn dismiss_toast_id(&mut self, id: u8, cx: &mut Context<Self>) {
        let now = GalleryInstant::now();
        self.toast_lifecycle.dismiss(&id, now);
        self.toast_lifecycle.advance(now, false);
        self.toast_message = self
            .toast_lifecycle
            .iter()
            .next_back()
            .map(|(_, message, _)| *message);
        if self.toast_message.is_none() {
            self.toast_timer = None;
            self.toast_hovered = false;
            self.toast_focused = false;
        }
        cx.notify();
    }

    fn show_toast(&mut self, message: &'static str, cx: &mut Context<Self>) {
        let now = GalleryInstant::now();
        self.toast_lifecycle.push(
            if message == "Could not sync workspace" {
                1
            } else {
                0
            },
            message,
            gpui_base::ToastOptions {
                timeout: (message != "Could not sync workspace")
                    .then_some(std::time::Duration::from_secs(6)),
            },
            now,
        );
        self.toast_lifecycle.advance(now, false);
        self.toast_message = Some(message);
        if !self
            .toast_lifecycle
            .iter()
            .any(|(_, message, _)| *message != "Could not sync workspace")
        {
            self.toast_timer = None;
            cx.notify();
            return;
        }
        self.toast_timer = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
                let Ok(keep_running) = view.update(cx, |this, cx| {
                    if this.toast_message.is_none() {
                        return false;
                    }
                    this.advance_toast(GalleryInstant::now(), cx);
                    this.toast_lifecycle
                        .iter()
                        .any(|(_, message, _)| *message != "Could not sync workspace")
                }) else {
                    break;
                };
                if !keep_running {
                    break;
                }
            }
        }));
        cx.notify();
    }

    fn render_notifications(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .absolute()
            .right(px(14.))
            .bottom(px(48.))
            .w(px(320.))
            .id("notification-stack")
            .track_focus(&self.toast_focus)
            .flex()
            .flex_col()
            .gap(px(8.))
            .occlude()
            .on_hover(cx.listener(|this, hovered, _, _| this.toast_hovered = *hovered))
            .children(
                self.toast_lifecycle
                    .iter()
                    .map(|(id, message, _)| {
                        let id = *id;
                        let message = *message;
                        toast(("gallery-toast", id as usize), cx)
                            .debug_selector(move || {
                                if id == 0 {
                                    "gallery-toast"
                                } else {
                                    "sync-error-toast"
                                }
                                .into()
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        icon(if id == 1 {
                                            IconName::TriangleAlert
                                        } else {
                                            IconName::Check
                                        })
                                        .size(px(16.)),
                                    )
                                    .child(div().flex_1().child(message))
                                    .child(
                                        button(
                                            ("dismiss-toast", id as usize),
                                            "",
                                            ButtonVariant::Secondary,
                                            cx,
                                        )
                                        .size(px(18.))
                                        .p(px(0.))
                                        .flex_shrink_0()
                                        .debug_selector(|| "dismiss-toast".into())
                                        .accessibility_label("Dismiss notification")
                                        .child(icon(IconName::Close).size(px(14.)))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.dismiss_toast_id(id, cx)
                                            }),
                                        ),
                                    ),
                            )
                            .child(
                                button(
                                    ("toast-action", id as usize),
                                    match message {
                                        "Workspace saved" => "Undo",
                                        "Could not sync workspace" => "Retry",
                                        _ => "Dismiss",
                                    },
                                    ButtonVariant::Outline,
                                    cx,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        match message {
                                            "Workspace saved" => {
                                                this.toast_saved = false;
                                                this.show_toast("Save undone", cx);
                                            }
                                            "Could not sync workspace" => {
                                                this.dismiss_toast_id(id, cx);
                                                this.toast_saved = true;
                                                this.show_toast("Workspace saved", cx);
                                            }
                                            _ => this.dismiss_toast_id(id, cx),
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                    })
                    .collect::<Vec<_>>(),
            )
    }

    fn render_toast_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content = content
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .child(
                        button("show-toast", "Save workspace", ButtonVariant::Outline, cx)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toast_saved = true;
                                this.show_toast("Workspace saved", cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        button(
                            "show-error-toast",
                            "Show error",
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_toast("Could not sync workspace", cx);
                            cx.notify();
                        })),
                    ),
            )
            .child(if self.toast_saved {
                "Example workspace: saved"
            } else {
                "Example workspace: unsaved"
            })
            .child(div().text_color(t.secondary).child(
                "Save and sync notifications appear together. Saved notifications close after six seconds; hover or focus pauses the timer. Errors remain until dismissed.",
            ));
        content
    }
    fn render_collapsible_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        content =
            content.child(
                collapsible(self.collapse_open, cx)
                    .child(
                        button("collapse-trigger", "", ButtonVariant::Outline, cx)
                            .debug_selector(|| "collapse-trigger".into())
                            .accessibility_label("Advanced settings")
                            .aria_expanded(self.collapse_open)
                            .child(
                                icon(if self.collapse_open {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .size(px(14.)),
                            )
                            .child("Advanced settings")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.collapse_open = !this.collapse_open;
                                cx.notify();
                            })),
                    )
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(14.))
                            .p(px(14.))
                            .bg(t.normal_fill())
                            .child("Workspace synchronization")
                            .child(
                                switch("collapse-sync", "Sync workspace", self.enabled, cx)
                                    .debug_selector(|| "collapse-sync".into())
                                    .on_change(change(cx.listener(|this, next, _, cx| {
                                        this.enabled = *next;
                                        cx.notify();
                                    }))),
                            )
                            .child(div().text_color(t.secondary).child(
                                "Your selection is preserved when this section is collapsed.",
                            )),
                    ),
            );
        content
    }
    fn render_icon_page(
        &mut self,
        mut content: gpui::Div,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> gpui::Div {
        for (name, glyph) in [
            ("Check", IconName::Check),
            ("Minus", IconName::Minus),
            ("Plus", IconName::Plus),
            ("Chevron down", IconName::ChevronDown),
            ("Chevron right", IconName::ChevronRight),
            ("Star", IconName::Star),
            ("External link", IconName::ExternalLink),
            ("Close", IconName::Close),
            ("Search", IconName::Search),
            ("Menu", IconName::Menu),
            ("Settings", IconName::Settings),
            ("Alert", IconName::TriangleAlert),
        ] {
            content = content.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon(glyph))
                    .child(name),
            );
        }
        content
    }
    fn render_slider_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content
            .child(format!("Volume: {}", self.slider_value.read(cx).value()))
            .child(slider(&self.slider_value, false, window, cx))
            .child(format!("Range: {}", self.slider_range.read(cx).value()))
            .child(slider(&self.slider_range, false, window, cx))
            .child("Disabled")
            .child(slider(&self.slider_disabled, true, window, cx))
            .child("Arrow keys / h l  adjust · Home / End  bounds · Tab  next thumb");
        content
    }
    fn render_dialog_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let alert = self.page == "alert_dialog";
        let specimen = if alert {
            self.reset_form(false, cx)
        } else {
            self.workspace_form(false, window, cx)
        };
        content = content
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::BOLD)
                    .child(if alert {
                        "Destructive confirmation"
                    } else {
                        "Form dialog"
                    }),
            )
            .child(div().text_color(t.secondary).child(if alert {
                "A named consequence, a clear way back, and a distinct destructive action."
            } else {
                "A short task with visible labels and outline actions."
            }))
            .child(dialog_popup(cx).w(px(460.)).child(specimen))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .mt(px(6.))
                    .child(
                        dialog_button(
                            "open-modal",
                            if alert {
                                "Open alert dialog…"
                            } else {
                                "Open dialog…"
                            },
                            ButtonVariant::Secondary,
                            cx,
                        )
                        .track_focus(&self.modal_trigger)
                        .on_click(cx.listener(|this, _, window, cx| {
                            let value = this.saved_workspace.clone();
                            this.workspace_draft
                                .update(cx, |state, cx| state.set_value(value, window, cx));
                            this.modal_open = true;
                            this.modal_focus.focus(window, cx);
                            cx.notify();
                        })),
                    )
                    .child(div().text_color(t.secondary).child(if alert {
                        "Escape cancels · Backdrop keeps the dialog open"
                    } else {
                        "Escape cancels · Tab moves between controls"
                    })),
            )
            .when(!self.modal_result.is_empty(), |content| {
                content.child(
                    div()
                        .text_color(t.secondary)
                        .child(self.modal_result.clone()),
                )
            });
        if self.modal_open {
            let body = if alert {
                self.reset_form(true, cx)
            } else {
                self.workspace_form(true, window, cx)
            };
            let popup = div()
                .id("modal-surface")
                .occlude()
                .max_w(gpui::relative(0.9))
                .child(dialog_popup(cx).w(px(460.)).child(body));
            let close = cx.listener(|this, confirmed: &bool, window, cx| {
                this.modal_open = false;
                if *confirmed {
                    if this.page == "alert_dialog" {
                        this.reset_workspace(window, cx);
                    } else {
                        this.save_workspace(true, window, cx);
                    }
                } else {
                    this.modal_result = "Changes discarded".into();
                }
                this.modal_trigger.focus(window, cx);
                cx.notify();
            });
            if alert {
                content = content.child(
                    alert_dialog(&self.modal_focus, cx)
                        .popup(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(popup),
                        )
                        .request_close(move |confirmed, window, cx| close(&confirmed, window, cx)),
                );
            } else {
                content = content.child(
                    dialog(&self.modal_focus, cx)
                        .on_ok({
                            let draft = self.workspace_draft.clone();
                            move |_, _, cx| !draft.read(cx).value().trim().is_empty()
                        })
                        .popup(popup)
                        .request_close(move |confirmed, window, cx| close(&confirmed, window, cx)),
                );
            }
        };
        content
    }
    fn render_select_page(
        &mut self,
        mut content: gpui::Div,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = cx.omarchy().clone();
        let searchable = self.page == "combobox";
        let state = if searchable {
            &self.combo_choice
        } else {
            &self.select_choice
        };
        let selected = state
            .read(cx)
            .selected()
            .map(|item| item.label.to_string())
            .unwrap_or_else(|| "None".into());
        let control = if searchable {
            combobox("workspace-choice", state, window, cx).into_any_element()
        } else {
            select("workspace-choice", state, window, cx).into_any_element()
        };
        let disabled = if searchable {
            combobox("managed-choice", &self.disabled_choice, window, cx).into_any_element()
        } else {
            select("managed-choice", &self.disabled_choice, window, cx).into_any_element()
        };
        content = content
            .child(
                div()
                    .w(px(360.))
                    .max_w(gpui::relative(1.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(if searchable {
                        "Find a workspace"
                    } else {
                        "Default workspace"
                    })
                    .child(control)
                    .child(
                        div()
                            .text_color(t.secondary)
                            .child("Archived workspaces cannot be selected."),
                    )
                    .child(div().mt(px(12.)).child(format!("Selected: {selected}"))),
            )
            .child(div().mt(px(18.)).w_full().child(separator(cx)))
            .child(
                div()
                    .w(px(360.))
                    .max_w(gpui::relative(1.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child("Managed workspace")
                    .child(disabled)
                    .child(
                        div()
                            .text_color(t.secondary)
                            .child("This setting is managed by your organization."),
                    ),
            );
        content
    }
}

impl Gallery {
    fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sheet_open = false;
        self.sheet_trigger.focus(window, cx);
        cx.notify();
    }

    fn render_project_sheet(&self, cx: &mut Context<Self>) -> gpui_base::Sheet {
        let surface = sheet_surface(cx)
            .debug_selector(|| "project-sheet".into())
            .child(dialog_title("Website refresh", cx))
            .child(dialog_description("Project details", cx))
            .child(separator(cx))
            .child("In review · Due Friday")
            .child("Owner: Alex Lee")
            .child("Review the homepage layout, navigation and images before the autumn release.")
            .child(separator(cx))
            .child("Next steps")
            .child("Check narrow windows")
            .child("Review keyboard navigation")
            .child("Approve release notes")
            .child(div().flex_1())
            .child(
                button("close-sheet", "Close", ButtonVariant::Outline, cx)
                    .debug_selector(|| "close-sheet".into())
                    .on_click(cx.listener(|this, _, window, cx| this.close_sheet(window, cx))),
            );
        let target = cx.entity();
        sheet(&self.sheet_focus, cx)
            .surface(surface)
            .request_close(move |window, cx| {
                target.update(cx, |this, cx| this.close_sheet(window, cx))
            })
    }
}

impl Gallery {
    fn render_text_view(&self, content: gpui::Div, cx: &App) -> gpui::Div {
        content.child(markdown("project-brief", "## Website refresh\n\nA clearer home for our **project documentation**. Keep navigation simple and preserve the reading experience.\n\n### Before launch\n\n- Review the introduction and installation steps.\n- Verify keyboard navigation in both themes.\n- Publish the release notes.\n\n> Changes should make the next step easier to understand.\n\nRun the gallery locally:\n\n```sh\ncargo run --example gallery\n```\n\nRead the [project source](https://github.com/huacnlee/gpui-omarchy) for usage examples.", cx))
            .child(separator(cx))
            .child(html("release-summary", "<h3>Release summary</h3><p><strong>Ready for review.</strong> The documentation is complete; launch follows the final keyboard check.</p><p><em>Updated by Alex Lee</em></p>", cx))
    }

    fn render_horizontal_scrollbar(&self, content: gpui::Div, cx: &App) -> gpui::Div {
        let t = cx.omarchy();
        content
            .child("Project timeline · Scroll horizontally")
            .child(
                div()
                    .relative()
                    .w_full()
                    .border_1()
                    .border_color(t.border)
                    .debug_selector(|| "timeline-viewport".into())
                    .child(
                        div()
                            .id("timeline-scroll")
                            .w_full()
                            .h(px(76.))
                            .overflow_x_scroll()
                            .track_scroll(&self.timeline_scroll)
                            .child(
                                div().flex().w(px(1440.)).h(px(64.)).children(
                                    [
                                        "Brief",
                                        "Research",
                                        "Design",
                                        "Prototype",
                                        "Review",
                                        "Build",
                                        "Test",
                                        "Launch",
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, name)| {
                                        div()
                                            .w(px(180.))
                                            .flex_shrink_0()
                                            .h_full()
                                            .p(px(10.))
                                            .border_r_1()
                                            .border_color(t.divider())
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.))
                                            .child(name)
                                            .child(
                                                div()
                                                    .text_color(t.secondary)
                                                    .child(format!("Week {}", index + 1)),
                                            )
                                    }),
                                ),
                            ),
                    )
                    .child(scrollbar(
                        "timeline-scrollbar",
                        gpui_base::ScrollbarAxis::Horizontal,
                        &self.timeline_scroll,
                        cx,
                    )),
            )
    }

    fn render_activity_list(
        &mut self,
        mut content: gpui::Div,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        content = content.child("Sample activity · 1,000 events").child(
            div().flex().gap(px(8.)).children(
                [
                    ("activity-first", "First event", 0),
                    ("activity-last", "Last event", 999),
                ]
                .into_iter()
                .map(|(id, label, index)| {
                    button(id, label, ButtonVariant::Outline, cx)
                        .debug_selector(move || id.into())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.activity_scroll
                                .scroll_to_item(index, gpui::ScrollStrategy::Top);
                            cx.notify();
                        }))
                }),
            ),
        );
        let list = virtual_list(
            cx.entity(),
            "activity-list",
            self.activity_sizes.clone(),
            |this, range, _, cx| {
                this.activity_rendered = range.len();
                let t = cx.omarchy();
                range
                    .map(|index| {
                        div()
                            .debug_selector(move || format!("activity-row-{index}"))
                            .h(this.activity_sizes[index].height)
                            .w_full()
                            .px(px(10.))
                            .flex()
                            .flex_col()
                            .justify_center()
                            .child(format!("Event {:04} · Updated project notes", index + 1))
                            .when(index % 5 == 0, |row| {
                                row.child(
                                    div()
                                        .text_color(t.secondary)
                                        .child("Review requested by Alex Lee"),
                                )
                            })
                    })
                    .collect::<Vec<_>>()
            },
            cx,
        )
        .track_scroll(&self.activity_scroll);
        content.child(div().relative().w_full().border_1().border_color(cx.omarchy().border)
            .debug_selector(|| "activity-viewport".into()).child(list)
            .child(scrollbar("activity-scrollbar", gpui_base::ScrollbarAxis::Vertical, &self.activity_scroll, cx)))
            .child(div().text_color(cx.omarchy().secondary).child("Scroll through the log or jump to either end. Longer events keep their full description."))
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.toast_focused = self.toast_focus.contains_focused(window, cx);
        let t = cx.omarchy().clone();
        let mut content = div()
            .flex()
            .flex_col()
            .items_start()
            .w_full()
            .gap(px(14.))
            .min_w_0();
        match self.page {
            "overview" => {
                content = self.render_overview_example(content, window, cx);
            }
            "popover" => {
                content = self.render_popover_example(content, window, cx);
            }
            "tooltip" => {
                content = self.render_tooltip_example(content, window, cx);
            }
            "select" | "combobox" => {
                content = self.render_select_page(content, window, cx);
            }
            "menu" => {
                content = self.render_menu_example(content, window, cx);
            }
            "dialog" | "alert_dialog" => {
                content = self.render_dialog_page(content, window, cx);
            }
            "slider" => {
                content = self.render_slider_page(content, window, cx);
            }
            "icon" => {
                content = self.render_icon_page(content, window, cx);
            }
            "collapsible" => {
                content = self.render_collapsible_page(content, window, cx);
            }
            "toast" => {
                content = self.render_toast_page(content, window, cx);
            }
            "accordion" => {
                content = self.render_accordion_page(content, window, cx);
            }
            "pagination" => {
                content = self.render_pagination_page(content, window, cx);
            }
            "table" => {
                content = self.render_table_example(content, window, cx);
            }
            "number_input" => {
                content = self.render_number_input_page(content, window, cx);
            }
            "input" => {
                content = self.render_input_page(content, window, cx);
            }
            "textarea" => {
                content = self.render_textarea_page(content, window, cx);
            }
            "button" => {
                content = self.render_button_example(content, window, cx);
            }
            "checkbox" => {
                content = self.render_checkbox_page(content, window, cx);
            }
            "switch" => {
                content = self.render_switch_page(content, window, cx);
            }
            "radio" => {
                content = self.render_radio_page(content, window, cx);
            }
            "toggle_group" => {
                content = self.render_toggle_group(content, cx);
            }
            "toggle" => {
                content = self.render_toggle_page(content, window, cx);
            }
            "link" => {
                content = self.render_link_page(content, window, cx);
            }
            "button_group" => {
                content = self.render_button_group_page(content, window, cx);
            }
            "tabs" => {
                content = self.render_tabs_example(content, window, cx);
            }
            "nav_stack" => {
                content = self.render_nav_stack_example(content, window, cx);
            }
            "otp_input" => {
                content = self.render_otp_input_page(content, window, cx);
            }
            "hover_card" => {
                content = self.render_hover_card_page(content, window, cx);
            }
            "dock" => {
                content = self.render_dock_page(content, window, cx);
            }
            "tree" => {
                content = self.render_tree_page(content, window, cx);
            }
            "resizable" => {
                content = self.render_resizable_example(content, window, cx);
            }
            "color_picker" => {
                content = self.render_color_picker_page(content, window, cx);
            }
            "date_picker" => {
                content = self.render_date_picker_page(content, window, cx);
            }
            "calendar" => {
                content = self.render_calendar_page(content, window, cx);
            }
            "avatar" => {
                content = self.render_avatar_example(content, window, cx);
            }
            "panel" => {
                content = self.render_panel_page(content, window, cx);
            }
            "separator" => {
                content = self.render_separator_page(content, window, cx);
            }
            "keycap" => {
                content = self.render_keycap_page(content, window, cx);
            }
            "badge" => {
                content = self.render_badge_page(content, window, cx);
            }
            "empty_state" => {
                content = self.render_empty_state_page(content, window, cx);
            }
            "sheet" => {
                content = content.child("Website refresh · In review")
                    .child(button("open-project-sheet", "Project details…", ButtonVariant::Outline, cx)
                        .track_focus(&self.sheet_trigger)
                        .debug_selector(|| "open-project-sheet".into())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.sheet_open = true;
                            this.sheet_focus.focus(window, cx);
                            cx.notify();
                        })))
                    .child(div().text_color(t.secondary).child("Inspect the project without leaving this page. Close the panel or press Escape to return."));
            }
            "text_view" => {
                content = self.render_text_view(content, cx);
            }
            "virtual_list" | "scrollbar" => {
                content = self.render_activity_list(content, cx);
                if self.page == "scrollbar" {
                    content = self.render_horizontal_scrollbar(content, cx);
                }
            }
            "progress" => {
                content = self.render_progress_page(content, window, cx);
            }
            _ => unreachable!(),
        }
        let sidebar_width = if window.viewport_size().width < px(700.) {
            148.
        } else {
            196.
        };
        let navigation = div()
            .id("component-sidebar")
            .track_focus(&self.navigation_focus)
            .w(px(sidebar_width))
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .border_r_1()
            .border_color(t.divider())
            .bg(t.background)
            .p(px(8.))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                let items: Vec<_> = components().collect();
                let current = items
                    .iter()
                    .position(|&item| item == this.page)
                    .unwrap_or(0);
                let next = match event.keystroke.key.as_str() {
                    "down" | "j" => (current + 1).min(items.len() - 1),
                    "up" | "k" => current.saturating_sub(1),
                    "home" => 0,
                    "end" => items.len() - 1,
                    _ => return,
                };
                this.page = items[next];
                if let Some(index) = this
                    .navigation_rows
                    .iter()
                    .position(|&(header, name)| !header && name == this.page)
                {
                    this.navigation_list.scroll_to_reveal_item(index);
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                gpui::list(self.navigation_list.clone(), {
                    let view = cx.entity();
                    move |index, _, cx| {
                        view.update(cx, |this, cx| {
                            let (header, name) = this.navigation_rows[index];
                            let t = cx.omarchy().clone();
                            if header {
                                return div()
                                    .mt(px(if index == 0 { 0. } else { 12. }))
                                    .mb(px(2.))
                                    .px(px(8.))
                                    .py(px(6.))
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(t.secondary)
                                    .child(name)
                                    .into_any_element();
                            }
                            div()
                                .pb(px(2.))
                                .child(
                                    button(name, display_name(name), ButtonVariant::Secondary, cx)
                                        .w_full()
                                        .justify_start()
                                        .px(px(8.))
                                        .py(px(5.))
                                        .border_color(t.foreground.opacity(0.))
                                        .selected(this.page == name)
                                        .focusable(false)
                                        .styles(|styles| {
                                            styles.selected(|style| {
                                                style
                                                    .bg(t.hover_fill())
                                                    .text_color(t.accent)
                                                    .font_weight(FontWeight::NORMAL)
                                            })
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.page = name;
                                            window.focus(&this.navigation_focus, cx);
                                            cx.notify();
                                        })),
                                )
                                .into_any_element()
                        })
                    }
                })
                .size_full(),
            );
        focus_scope("gallery")
            .relative()
            .debug_selector(|| "gallery-root".into())
            .size_full()
            .flex()
            .flex_col()
            .bg(t.background)
            .text_color(t.foreground)
            .font_family(t.font.clone())
            .text_size(px(12.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .px(px(14.))
                    .py(px(10.))
                    .border_b_1()
                    .border_color(t.divider())
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .flex_1()
                            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                                window.start_window_move()
                            })
                            .child("GPUI Omarchy"),
                    )
                    .child(
                        menu(
                            "application-menu",
                            with_tooltip(
                                button(
                                    "application-menu-trigger",
                                    "Menu",
                                    ButtonVariant::Secondary,
                                    cx,
                                )
                                .styles(|styles| styles.selected(|style| style.bg(t.border)))
                                .child(icon(IconName::ChevronDown).size(px(14.))),
                                "Appearance and application commands",
                            ),
                            vec![
                                MenuItem::new("System theme").checked(self.theme_mode == 0),
                                MenuItem::new("Tokyo Night").checked(self.theme_mode == 1),
                                MenuItem::new("Flexoki Light").checked(self.theme_mode == 2),
                                MenuItem::new("Exit")
                                    .separator_before()
                                    .disabled(cfg!(target_family = "wasm")),
                            ],
                            {
                                let target = cx.entity();
                                move |index, _, cx| {
                                    target.update(cx, |this, cx| {
                                        match index {
                                            0 => {
                                                #[cfg(not(target_family = "wasm"))]
                                                Theme::follow_system(cx);
                                                #[cfg(target_family = "wasm")]
                                                apply_gallery_theme(Theme::system_or_default(), cx);
                                            }
                                            1 => apply_gallery_theme(Theme::tokyo_night(), cx),
                                            2 => apply_gallery_theme(Theme::flexoki_light(), cx),
                                            _ => {
                                                cx.quit();
                                                return;
                                            }
                                        }
                                        this.theme_mode = index;
                                        cx.notify();
                                    })
                                }
                            },
                        )
                        .anchor(gpui::Anchor::TopRight),
                    ),
            )
            .child(
                div().flex().flex_1().min_h_0().child(navigation).child(
                    div()
                        .id("component-content")
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .overflow_y_scroll()
                        .p(px(28.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(14.))
                                .max_w(px(760.))
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(FontWeight::BOLD)
                                        .child(display_name(self.page)),
                                )
                                .child(div().text_color(t.secondary).child(description(self.page)))
                                .child(div().mt(px(14.)).child(content)),
                        ),
                ),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex_shrink_0()
                    .px(px(14.))
                    .py(px(8.))
                    .border_t_1()
                    .border_color(t.divider())
                    .text_color(t.secondary)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().child(
                        "↑↓ / j k  component · Tab / Shift + Tab  focus · Return / Space  activate",
                    ))
                    .child(
                        div().flex().flex_1().min_w(px(320.)).justify_end().child(
                            link(
                                "repository",
                                "https://github.com/huacnlee/gpui-omarchy",
                                "https://github.com/huacnlee/gpui-omarchy",
                                cx,
                            )
                            .py(px(0.))
                            .px(px(0.))
                            .flex_shrink_0()
                            .debug_selector(|| "gallery-repository".into())
                            .text_color(t.secondary),
                        ),
                    ),
            )
            .child(gpui_base::TextSelectionLayer)
            .when(self.sheet_open, |root| {
                root.child(self.render_project_sheet(cx))
            })
            .when(self.toast_message.is_some(), |root| {
                root.child(self.render_notifications(cx))
            })
    }
}

fn demo_dock_layout(cx: &mut App) -> gpui_base::dock::DockLayout {
    use gpui_base::dock::DockLayout;
    let files = cx.new(|cx| DemoDockPanel {
        title: "Files",
        focus: cx.focus_handle(),
    });
    let notes = cx.new(|cx| DemoDockPanel {
        title: "Notes",
        focus: cx.focus_handle(),
    });
    let preview = cx.new(|cx| DemoDockPanel {
        title: "Preview",
        focus: cx.focus_handle(),
    });
    DockLayout::h_split()
        .child(DockLayout::tabs().panel(files).panel(notes), Some(px(280.)))
        .child(DockLayout::tabs().panel(preview), None)
}

struct DemoDockPanel {
    title: &'static str,
    focus: FocusHandle,
}
impl gpui::EventEmitter<gpui_base::dock::PanelEvent> for DemoDockPanel {}
impl gpui::Focusable for DemoDockPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl gpui_base::dock::Panel for DemoDockPanel {
    fn panel_name(&self) -> &'static str {
        self.title
    }
}
impl gpui::Render for DemoDockPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .track_focus(&self.focus)
            .child(div().font_weight(FontWeight::BOLD).child(self.title))
            .children(
                match self.title {
                    "Files" => vec![
                        "Project brief.md",
                        "Meeting notes.md",
                        "Release checklist.md",
                    ],
                    "Notes" => vec![
                        "Workspace review",
                        "Refine the navigation and preview layout.",
                        "Prepare release documentation.",
                    ],
                    _ => vec![
                        "Project brief",
                        "A focused desktop workspace for files, notes and project activity.",
                    ],
                }
                .into_iter()
                .map(|text| div().text_color(cx.omarchy().secondary).child(text)),
            )
    }
}

fn gallery_avatar() -> std::sync::Arc<gpui::Image> {
    static IMAGE: std::sync::OnceLock<std::sync::Arc<gpui::Image>> = std::sync::OnceLock::new();
    IMAGE
        .get_or_init(|| {
            std::sync::Arc::new(gpui::Image::from_bytes(
                gpui::ImageFormat::Png,
                include_bytes!("../assets/huacnlee.png").to_vec(),
            ))
        })
        .clone()
}

struct NavigationPage {
    level: usize,
    next: Option<Entity<NavigationPage>>,
    navigation: gpui::WeakEntity<gpui_base::NavStackState>,
    notes: Entity<gpui_base::input::InputState>,
}
impl gpui::Render for NavigationPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.omarchy().clone();
        let (title, description) = match self.level {
            0 => ("Projects", "Choose a project to see its tasks."),
            1 => (
                "Website refresh",
                "Update the homepage for the autumn release.",
            ),
            _ => ("Review homepage", "Website refresh · Alex Lee · Due Friday"),
        };
        let mut page = div()
            .id("navigation-page")
            .size_full()
            .overflow_y_scroll()
            .p(px(18.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(div().text_color(t.secondary).child(description));
        if self.level < 2 {
            let (name, detail) = if self.level == 0 {
                (
                    "Website refresh",
                    "Alex Lee · 1 task to review · Updated today",
                )
            } else {
                ("Review homepage", "In review · Due Friday")
            };
            page = page.child(
                button("nav-open", "", ButtonVariant::Outline, cx)
                    .debug_selector(|| "nav-open".to_string())
                    .w_full()
                    .justify_between()
                    .py(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap(px(4.))
                            .child(name)
                            .child(div().text_color(t.secondary).child(detail)),
                    )
                    .child(icon(IconName::ChevronRight))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let (Some(navigation), Some(next)) =
                            (this.navigation.upgrade(), this.next.clone())
                        {
                            navigation.update(cx, |state, cx| {
                                state.push(next, gpui_base::NavMotion::Immediate, cx)
                            });
                        }
                    })),
            );
            page = page.child(div().text_color(t.secondary).child(if self.level == 0 {
                "1 active project"
            } else {
                "Review the layout and leave a note before approving the release."
            }));
        } else {
            page =
                page.child(div().child("Review checklist"))
                    .child(div().text_color(t.secondary).child(
                        "Confirm the heading, navigation and images work in a narrow window.",
                    ))
                    .child("Review note")
                    .child(
                        input("nav-note", &self.notes, window, cx)
                            .debug_selector(|| "nav-note".to_string())
                            .w_full(),
                    )
                    .child(div().text_color(t.secondary).child(
                        "Kept in this demo while you navigate. Nothing is sent or saved to disk.",
                    ));
        }
        page
    }
}

#[cfg(not(target_family = "wasm"))]
fn install_panic_report() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!(
            "gpui-omarchy-gallery-{}.panic.log",
            std::process::id(),
        ));
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(
                file,
                "{info}\n{}",
                std::backtrace::Backtrace::force_capture()
            );
            let _ = file.flush();
            eprintln!("Gallery panic report: {}", path.display());
        }
        previous(info);
    }));
}

#[cfg(not(target_family = "wasm"))]
pub fn run() {
    install_panic_report();
    gpui::platform::application().run(move |cx| {
        gpui_omarchy::init(cx);
        cx.open_window(
            WindowOptions {
                titlebar: None,
                window_min_size: Some(size(px(680.), px(520.))),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1060.), px(760.)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Gallery::new(window, cx)),
        )
        .expect("open component gallery");
        cx.activate(true);
    });
}

fn initial_page() -> &'static str {
    #[cfg(not(target_family = "wasm"))]
    if let Some(page) = components().find(|name| std::env::args().nth(1).as_deref() == Some(*name))
    {
        return page;
    }
    "overview"
}

fn apply_gallery_theme(theme: Theme, cx: &mut App) {
    #[cfg(target_family = "wasm")]
    let theme = Theme {
        font: "Inter Variable".into(),
        ..theme
    };
    theme.apply(cx);
}

#[cfg(target_family = "wasm")]
pub fn open_web_gallery(theme: Theme, cx: &mut App) {
    gpui_omarchy::init(cx);
    apply_gallery_theme(theme, cx);
    cx.open_window(WindowOptions::default(), |window, cx| {
        cx.new(|cx| Gallery::new(window, cx))
    })
    .expect("open web component gallery");
    cx.activate(true);
}

fn change<T>(
    listener: impl Fn(&T, &mut Window, &mut App) + 'static,
) -> impl Fn(T, &ClickEvent, &mut Window, &mut App) {
    move |value, _, window, cx| listener(&value, window, cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn dialogs_cancel_confirm_and_restore_focus(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        for page in ["dialog", "alert_dialog"] {
            for (key, result) in [("escape", "Cancelled"), ("enter", "Confirmed")] {
                view.update(cx, |this, cx| {
                    this.page = page;
                    this.modal_open = true;
                    cx.notify();
                });
                cx.update(|window, cx| {
                    view.read(cx).modal_focus.clone().focus(window, cx);
                    window.draw(cx).clear(cx);
                });
                cx.simulate_keystrokes(key);
                cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    let this = view.read(cx);
                    assert!(!this.modal_open, "{page}: {key}");
                    assert_eq!(
                        this.modal_result,
                        if result == "Cancelled" {
                            "Changes discarded"
                        } else if page == "alert_dialog" {
                            "Workspace defaults restored"
                        } else {
                            "Saved “Personal workspace”"
                        }
                    );
                    assert!(this.modal_trigger.is_focused(window));
                });
            }
        }
    }

    #[gpui::test]
    fn dialog_rejects_blank_name_without_losing_draft(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.page = "dialog";
                this.modal_open = true;
                this.workspace_draft
                    .update(cx, |state, cx| state.set_value("   ", window, cx));
                this.modal_focus.focus(window, cx);
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let this = view.read(cx);
            assert!(this.modal_open);
            assert_eq!(this.saved_workspace, "Personal workspace");
            assert_eq!(this.workspace_draft.read(cx).value().as_ref(), "   ");
        });
    }

    #[gpui::test]
    fn sidebar_keys_switch_components_without_intercepting_text_input(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        cx.update(|window, cx| view.read(cx).navigation_focus.clone().focus(window, cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("j");
        cx.update(|_, cx| assert_eq!(view.read(cx).page, "button"));
        cx.simulate_keystrokes("end");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).page, "progress");
            assert!(view.read(cx).navigation_list.logical_scroll_top().item_ix > 0);
        });
        cx.simulate_keystrokes("home");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).page, "overview");
            assert!(view.read(cx).navigation_list.logical_scroll_top().item_ix <= 1);
        });
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.page = "input";
                this.input.update(cx, |state, cx| state.focus(window, cx));
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_input("jk");
        cx.update(|_, cx| assert_eq!(view.read(cx).page, "input"));
        cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "jk"));
    }

    #[gpui::test]
    fn toast_timeout_pauses_and_replacement_restarts_the_deadline(cx: &mut TestAppContext) {
        use std::time::{Duration, Instant};
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.show_toast("Workspace saved", cx);
            this.toast_timer = None;
            let now = Instant::now();
            this.toast_hovered = true;
            this.advance_toast(now + Duration::from_secs(20), cx);
            assert_eq!(this.toast_message, Some("Workspace saved"));
            this.toast_hovered = false;
            this.toast_focused = true;
            this.advance_toast(now + Duration::from_secs(40), cx);
            assert!(this.toast_message.is_some());
            this.toast_focused = false;
            this.advance_toast(now + Duration::from_secs(47), cx);
            assert_eq!(this.toast_message, None);

            this.show_toast("Workspace saved", cx);
            this.toast_timer = None;
            this.advance_toast(Instant::now() + Duration::from_secs(5), cx);
            assert!(this.toast_message.is_some());
            this.show_toast("Save undone", cx);
            this.toast_timer = None;
            assert_eq!(this.toast_lifecycle.len(), 1);
            let replaced = Instant::now();
            this.advance_toast(replaced + Duration::from_secs(2), cx);
            assert_eq!(this.toast_message, Some("Save undone"));
            this.advance_toast(replaced + Duration::from_secs(7), cx);
            assert_eq!(this.toast_message, None);

            this.show_toast("Could not sync workspace", cx);
            assert!(this.toast_timer.is_none());
            this.advance_toast(Instant::now() + Duration::from_secs(600), cx);
            assert_eq!(this.toast_message, Some("Could not sync workspace"));
        });
    }

    #[gpui::test]
    fn toast_stack_keeps_independent_notifications(cx: &mut TestAppContext) {
        use std::time::{Duration, Instant};
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.show_toast("Workspace saved", cx);
            this.show_toast("Could not sync workspace", cx);
            this.toast_timer = None;
            assert_eq!(this.toast_lifecycle.len(), 2);
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let saved = cx.debug_bounds("gallery-toast").unwrap();
        let error = cx.debug_bounds("sync-error-toast").unwrap();
        assert!(
            saved.bottom_right().y < error.origin.y,
            "cards must not overlap"
        );
        assert_eq!(saved.origin.x, error.origin.x);
        view.update(cx, |this, cx| {
            this.dismiss_toast_id(0, cx);
            assert_eq!(this.toast_lifecycle.len(), 1);
            assert_eq!(this.toast_message, Some("Could not sync workspace"));
            this.show_toast("Workspace saved", cx);
            this.toast_timer = None;
            this.advance_toast(Instant::now() + Duration::from_secs(7), cx);
            assert_eq!(this.toast_lifecycle.len(), 1);
            assert_eq!(this.toast_message, Some("Could not sync workspace"));
            this.dismiss_toast_id(1, cx);
            assert!(this.toast_lifecycle.is_empty());
        });
    }

    #[gpui::test]
    fn dismissed_toast_does_not_pause_the_next_notification(cx: &mut TestAppContext) {
        use std::time::{Duration, Instant};
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.show_toast("Workspace saved", cx);
            this.toast_hovered = true;
            this.toast_focused = true;
            this.dismiss_toast_id(0, cx);
            assert!(this.toast_lifecycle.is_empty());
            assert!(this.toast_timer.is_none());
            assert!(!this.toast_hovered && !this.toast_focused);
            this.show_toast("Workspace saved", cx);
            this.toast_timer = None;
            this.advance_toast(Instant::now() + Duration::from_secs(7), cx);
            assert!(this.toast_message.is_none());
        });
    }

    #[gpui::test]
    fn toast_is_bottom_right_and_dismissible_over_page_content(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "toast";
            this.show_toast("Workspace saved", cx);
            this.toast_saved = true;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let root = cx.debug_bounds("gallery-root").unwrap();
        let notification = cx.debug_bounds("gallery-toast").unwrap();
        assert_eq!(root.right() - notification.right(), px(14.));
        assert_eq!(root.bottom() - notification.bottom(), px(48.));
        let close = cx.debug_bounds("dismiss-toast").unwrap();
        cx.simulate_click(close.center(), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("gallery-toast").is_none());
        cx.update(|_, cx| {
            assert!(
                view.read(cx).toast_saved,
                "dismissing does not undo the save"
            )
        });
    }

    #[gpui::test]
    fn disclosure_examples_preserve_independent_state(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "accordion";
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("accordion-panel-0").is_some());
        assert!(cx.debug_bounds("accordion-panel-1").is_none());
        for selector in ["accordion-trigger-1", "accordion-trigger-2"] {
            let bounds = cx.debug_bounds(selector).unwrap();
            cx.simulate_click(bounds.center(), Default::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        for selector in [
            "accordion-panel-0",
            "accordion-panel-1",
            "accordion-panel-2",
        ] {
            assert!(cx.debug_bounds(selector).is_some());
        }
        let first = cx.debug_bounds("accordion-trigger-0").unwrap();
        cx.simulate_click(first.center(), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("accordion-panel-0").is_none());
        assert!(cx.debug_bounds("accordion-panel-1").is_some());
        assert!(cx.debug_bounds("accordion-panel-2").is_some());

        view.update(cx, |this, cx| {
            this.page = "collapsible";
            this.enabled = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("collapse-sync").is_none());
        let trigger = cx.debug_bounds("collapse-trigger").unwrap().center();
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let control = cx.debug_bounds("collapse-sync").unwrap().center();
        cx.simulate_click(control, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|_, cx| assert!(view.read(cx).enabled));
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("collapse-sync").is_none());
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("collapse-sync").is_some());
        cx.update(|_, cx| assert!(view.read(cx).enabled));
    }

    #[gpui::test]
    fn tab_settings_survive_switching_pages(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "tabs";
            this.enabled = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let settings = cx.debug_bounds("omarchy-tab-2").unwrap().center();
        cx.simulate_click(settings, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let sync = cx.debug_bounds("tab-sync").unwrap().center();
        cx.simulate_click(sync, Default::default());
        cx.update(|window, cx| {
            let input = view.read(cx).workspace_name.clone();
            input.update(cx, |state, cx| state.focus(window, cx));
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        });
        cx.simulate_input("Studio");
        let overview = cx.debug_bounds("omarchy-tab-0").unwrap().center();
        cx.simulate_click(overview, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("tab-sync").is_none());
        cx.simulate_click(settings, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("tab-sync").is_some());
        cx.update(|_, cx| {
            assert!(view.read(cx).enabled);
            assert_eq!(
                view.read(cx).workspace_name.read(cx).value().as_ref(),
                "Studio"
            );
        });
    }

    #[gpui::test]
    fn radio_density_updates_the_list_preview(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "radio";
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let compact = cx.debug_bounds("density-preview-0").unwrap().size.height;
        let spacious = cx.debug_bounds("density-option-2").unwrap().center();
        cx.simulate_click(spacious, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).choice, 2);
        });
        assert!(cx.debug_bounds("density-preview-0").unwrap().size.height > compact);
        let compact_option = cx.debug_bounds("density-option-0").unwrap().center();
        cx.simulate_click(compact_option, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            cx.debug_bounds("density-preview-0").unwrap().size.height,
            compact
        );
        cx.simulate_keystrokes("up");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                view.read(cx).choice,
                2,
                "arrows wrap and select immediately"
            );
            assert!(view.read(cx).radio_focus.is_focused(window));
        });
        cx.simulate_keystrokes("j");
        cx.update(|_, cx| assert_eq!(view.read(cx).choice, 0));
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| assert!(!view.read(cx).radio_focus.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        cx.update(|window, cx| {
            assert!(
                view.read(cx).radio_focus.is_focused(window),
                "one Tab stop for the group"
            )
        });
    }

    #[gpui::test]
    fn input_reset_clears_value_and_returns_focus(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.page = "input";
                this.count = 6;
                this.input
                    .update(cx, |input, cx| input.set_value("Studio", window, cx));
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        let reset = cx.debug_bounds("reset-input").unwrap().center();
        cx.simulate_click(reset, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let this = view.read(cx);
            assert!(this.input.read(cx).value().is_empty());
            assert_eq!(this.count, 0);
            use gpui::Focusable;
            assert!(this.input.read(cx).focus_handle(cx).is_focused(window));
        });
        cx.simulate_input("New name");
        cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "New name"));
    }

    #[gpui::test]
    fn date_picker_opens_and_escape_preserves_selection(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "date_picker";
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let trigger = cx.debug_bounds("date-picker-trigger").unwrap().center();
        cx.simulate_click(trigger, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(view.read(cx).date_picker_state.read(cx).is_open());
        });
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(!view.read(cx).date_picker_state.read(cx).is_open());
            assert!(
                !view
                    .read(cx)
                    .date_picker_state
                    .read(cx)
                    .calendar
                    .read(cx)
                    .date()
                    .is_some()
            );
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(
                view.read(cx).date_picker_state.read(cx).is_open(),
                "focus returns to trigger for reopening"
            );
        });
    }

    #[gpui::test]
    fn repository_link_is_visible_inside_the_window(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (_, cx) = cx.add_window_view(Gallery::new);
        for width in [680., 1060., 1440.] {
            cx.simulate_resize(size(px(width), px(760.)));
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let root = cx.debug_bounds("gallery-root").unwrap();
            let link = cx.debug_bounds("gallery-repository").unwrap();
            assert!(link.right() <= root.right(), "{link:?} vs {root:?}");
            assert!(link.left() >= root.left());
            assert!(
                link.bottom() <= root.bottom(),
                "footer link is clipped below the window: {link:?} vs {root:?}"
            );
            assert!(link.size.width > px(100.));
        }
    }

    #[gpui::test]
    fn dock_drag_merges_tabs_without_losing_panels(cx: &mut TestAppContext) {
        use gpui::MouseButton;
        use gpui_base::dock::DockPlacement;
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "dock";
            cx.notify();
        });
        for _ in 0..2 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        let source = cx.debug_bounds("dock-tab-Notes").unwrap().center();
        let destination = cx.debug_bounds("dock-tab-Preview").unwrap().center();
        cx.simulate_mouse_down(source, MouseButton::Left, Default::default());
        cx.simulate_mouse_move(
            source + gpui::point(px(12.), px(0.)),
            Some(MouseButton::Left),
            Default::default(),
        );
        cx.simulate_mouse_move(destination, Some(MouseButton::Left), Default::default());
        cx.simulate_mouse_up(destination, MouseButton::Left, Default::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let dock = view.read(cx).dock_state.read(cx);
            let layout = dock.layout(DockPlacement::Center).unwrap();
            assert_eq!(layout.panels().count(), 3);
            let find = |name| {
                layout
                    .panels()
                    .find(|id| dock.panel(*id).unwrap().panel_name(cx) == name)
                    .unwrap()
            };
            assert_eq!(
                layout.find_panel_node(find("Notes")),
                layout.find_panel_node(find("Preview"))
            );
            assert_ne!(
                layout.find_panel_node(find("Files")),
                layout.find_panel_node(find("Preview"))
            );
        });
        assert!(cx.debug_bounds("dock-tab-Notes").is_some());
    }

    #[gpui::test]
    fn navigation_task_note_survives_back_and_forward(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "nav_stack";
            cx.notify();
        });
        for _ in 0..2 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let target = cx.debug_bounds("nav-open").unwrap().center();
            cx.simulate_click(target, Default::default());
        }
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let notes = cx.update(|_, cx| {
            let nav = view.read(cx).nav_state.read(cx);
            assert_eq!(nav.depth(), 3);
            nav.current()
                .unwrap()
                .clone()
                .downcast::<NavigationPage>()
                .unwrap()
                .read(cx)
                .notes
                .clone()
        });
        cx.update(|window, cx| {
            notes.update(cx, |state, cx| {
                state.set_value("", window, cx);
                state.focus(window, cx);
            })
        });
        cx.simulate_input("Ready for review");
        for selector in ["nav-back", "nav-back", "nav-forward", "nav-forward"] {
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let target = cx.debug_bounds(selector).unwrap().center();
            cx.simulate_click(target, Default::default());
        }
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let nav = view.read(cx).nav_state.read(cx);
            assert_eq!(nav.depth(), 3);
            assert_eq!(
                nav.current()
                    .unwrap()
                    .clone()
                    .downcast::<NavigationPage>()
                    .unwrap()
                    .read(cx)
                    .notes,
                notes
            );
            assert_eq!(notes.read(cx).value().as_ref(), "Ready for review");
        });
    }

    #[gpui::test]
    fn color_picker_cancels_preview_and_commits_hex(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "color_picker";
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let original = cx.update(|_, cx| view.read(cx).color_state.read(cx).value());
        for commit in [false, true] {
            let trigger = cx.debug_bounds("color-picker-trigger").unwrap().center();
            cx.simulate_click(trigger, Default::default());
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let state = view.read(cx).color_state.clone();
                assert!(state.read(cx).is_open());
                let hex = state.read(cx).hex_input().clone();
                hex.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                    input.focus(window, cx);
                });
            });
            cx.simulate_input("#FF0000");
            cx.simulate_keystrokes(if commit { "enter" } else { "escape" });
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let state = view.read(cx).color_state.read(cx);
                assert!(!state.is_open());
                assert!(gpui::Focusable::focus_handle(state, cx).is_focused(window));
                if commit {
                    assert_eq!(state.value(), Some(gpui::hsla(0., 1., 0.5, 1.)));
                } else {
                    assert_eq!(state.value(), original);
                }
            });
        }
    }

    #[gpui::test]
    fn toggle_group_combines_filters_and_recovers_from_empty(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "toggle_group";
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("filtered-article-0").is_some());
        assert!(cx.debug_bounds("filtered-article-1").is_some());
        assert!(cx.debug_bounds("filtered-article-2").is_none());
        for index in 0..2 {
            let target = cx
                .debug_bounds(["article-filter-0", "article-filter-1"][index])
                .unwrap()
                .center();
            cx.simulate_click(target, Default::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        for index in 0..4 {
            assert!(
                cx.debug_bounds(
                    [
                        "filtered-article-0",
                        "filtered-article-1",
                        "filtered-article-2",
                        "filtered-article-3"
                    ][index]
                )
                .is_none()
            );
        }
        let reset = cx.debug_bounds("show-all-articles").unwrap().center();
        cx.simulate_click(reset, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        for index in 0..4 {
            assert!(
                cx.debug_bounds(
                    [
                        "filtered-article-0",
                        "filtered-article-1",
                        "filtered-article-2",
                        "filtered-article-3"
                    ][index]
                )
                .is_some()
            );
        }
    }

    #[gpui::test]
    fn sheet_geometry_and_dismissal_restore_focus(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "sheet";
            cx.notify();
        });
        for dismissal in ["escape", "close", "backdrop"] {
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let trigger = cx.debug_bounds("open-project-sheet").unwrap().center();
            cx.simulate_click(trigger, Default::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let root = cx.debug_bounds("gallery-root").unwrap();
            let panel = cx.debug_bounds("project-sheet").unwrap();
            assert_eq!(panel.right(), root.right());
            assert_eq!(panel.top(), root.top());
            assert_eq!(panel.bottom(), root.bottom());
            cx.simulate_click(panel.center(), Default::default());
            cx.update(|_, cx| assert!(view.read(cx).sheet_open));
            match dismissal {
                "escape" => cx.simulate_keystrokes("escape"),
                "close" => {
                    let close = cx.debug_bounds("close-sheet").unwrap().center();
                    cx.simulate_click(close, Default::default());
                }
                _ => cx.simulate_click(
                    root.origin + gpui::point(px(10.), px(10.)),
                    Default::default(),
                ),
            }
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(!view.read(cx).sheet_open, "{dismissal}");
                assert!(view.read(cx).sheet_trigger.is_focused(window));
            });
        }
    }

    #[gpui::test]
    fn virtual_list_jumps_to_ends_without_building_all_rows(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "virtual_list";
            cx.notify();
        });
        for _ in 0..2 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        assert!(cx.debug_bounds("activity-row-0").is_some());
        assert!(cx.debug_bounds("activity-row-999").is_none());
        for (button, row, hidden) in [
            ("activity-last", "activity-row-999", "activity-row-0"),
            ("activity-first", "activity-row-0", "activity-row-999"),
        ] {
            let target = cx.debug_bounds(button).unwrap().center();
            cx.simulate_click(target, Default::default());
            for _ in 0..2 {
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
            assert!(cx.debug_bounds(row).is_some());
            assert!(cx.debug_bounds(hidden).is_none());
            cx.update(|_, cx| {
                let rendered = view.read(cx).activity_rendered;
                assert!(rendered > 0 && rendered < 30, "rendered {rendered} rows");
            });
        }
    }

    #[gpui::test]
    fn scrollbar_drag_changes_the_virtual_list_viewport(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "scrollbar";
            cx.notify();
        });
        for _ in 0..2 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        let bounds = cx.debug_bounds("activity-viewport").unwrap();
        let start = gpui::point(bounds.right() - px(5.), bounds.top() + px(6.));
        let end = gpui::point(start.x, bounds.bottom() - px(12.));
        cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
        cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
        for _ in 0..2 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        cx.update(|_, cx| {
            assert!(view.read(cx).activity_scroll.offset().y < -px(1000.));
            assert!(view.read(cx).activity_rendered < 30);
        });
        assert!(cx.debug_bounds("activity-row-0").is_none());
    }

    #[gpui::test]
    fn horizontal_scrollbar_drag_moves_the_timeline(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        view.update(cx, |this, cx| {
            this.page = "scrollbar";
            cx.notify();
        });
        for _ in 0..2 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        let bounds = cx.debug_bounds("timeline-viewport").unwrap();
        let start = gpui::point(bounds.left() + px(20.), bounds.bottom() - px(5.));
        let end = gpui::point(bounds.right() - px(12.), start.y);
        cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
        cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
        for _ in 0..2 {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        cx.update(|_, cx| {
            assert!(view.read(cx).timeline_scroll.offset().x < -px(200.));
        });
    }

    #[gpui::test]
    fn every_component_renders_in_both_themes(cx: &mut TestAppContext) {
        cx.update(gpui_omarchy::init);
        let (view, cx) = cx.add_window_view(Gallery::new);
        for theme in [Theme::tokyo_night(), Theme::flexoki_light()] {
            cx.update(|_, cx| theme.apply(cx));
            for page in components() {
                cx.update(|window, cx| {
                    view.update(cx, |this, cx| {
                        this.page = page;
                        cx.notify();
                    });
                    window.draw(cx).clear(cx);
                });
            }
        }
    }
}
