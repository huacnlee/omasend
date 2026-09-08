use gpui::{
    Context, IntoElement, Modifiers, Render, TestAppContext, Window, div, point, prelude::*, px,
};
use gpui_omarchy::*;
use std::{cell::Cell, rc::Rc};

struct Harness {
    clicks: Rc<Cell<usize>>,
    disabled: bool,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        div().id("root").tab_group().size(px(200.)).child(
            button("apply", "Apply", ButtonVariant::Primary, cx)
                .w(px(120.))
                .disabled(self.disabled)
                .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
        )
    }
}

#[gpui::test]
fn styled_button_accepts_pointer_return_and_space(cx: &mut TestAppContext) {
    cx.update(gpui_omarchy::init);
    let clicks = Rc::new(Cell::new(0));
    let handle = clicks.clone();
    let (_, cx) = cx.add_window_view(move |_, _| Harness {
        clicks: handle,
        disabled: false,
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 1);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for key in ["enter", "space"] {
        let keystroke = gpui::Keystroke::parse(key).unwrap();
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
    }
    assert_eq!(clicks.get(), 3);
}

#[gpui::test]
fn disabled_styled_button_cannot_activate(cx: &mut TestAppContext) {
    cx.update(gpui_omarchy::init);
    let clicks = Rc::new(Cell::new(0));
    let handle = clicks.clone();
    let (_, cx) = cx.add_window_view(move |_, _| Harness {
        clicks: handle,
        disabled: true,
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
    cx.update(|window, cx| window.focus_next(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for key in ["enter", "space"] {
        let keystroke = gpui::Keystroke::parse(key).unwrap();
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
    }
    assert_eq!(clicks.get(), 0);
}

#[gpui::test]
fn applying_theme_updates_base_tokens_and_geometry(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_omarchy::init(cx);
        Theme::flexoki_light().apply(cx);
        let base = gpui_base::Theme::global(cx);
        assert_eq!(base.tokens.colors, cx.omarchy().tokens());
        assert_eq!(base.appearance, gpui_base::ThemeAppearance::Light);
        for radius in [
            base.tokens.radius.none,
            base.tokens.radius.sm,
            base.tokens.radius.md,
            base.tokens.radius.lg,
            base.tokens.radius.xl,
            base.tokens.radius.full,
        ] {
            assert_eq!(radius, px(0.));
        }
        assert_eq!(base.tokens.typography.md.size, px(12.));
        assert_eq!(base.tokens.typography.sans.as_ref(), ".SystemUIFont");
        for theme in [Theme::tokyo_night(), Theme::flexoki_light()] {
            let style = theme
                .input_style()
                .resolved(&gpui_base::SemanticThemeTokens {
                    colors: theme.tokens(),
                    ..Default::default()
                });
            assert_eq!(style.selection, theme.foreground.opacity(0.35));
            assert_eq!(style.selection.a, 0.35);
            assert_ne!(style.selection, style.background);
        }
        Theme::tokyo_night().apply(cx);
        assert_eq!(
            gpui_base::Theme::global(cx).tokens.colors,
            cx.omarchy().tokens()
        );
    });
}

struct MenuHarness(Rc<Cell<usize>>);
impl Render for MenuHarness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.0.clone();
        div().size(px(300.)).child(menu(
            "actions",
            button("trigger", "Actions", ButtonVariant::Secondary, cx),
            vec![
                MenuItem::new("First"),
                MenuItem::new("Disabled").disabled(true),
                MenuItem::new("Last"),
            ],
            move |index, _, _| selected.set(index),
        ))
    }
}
#[gpui::test]
fn menu_keyboard_skips_disabled_and_selects(cx: &mut TestAppContext) {
    cx.update(gpui_omarchy::init);
    let selected = Rc::new(Cell::new(usize::MAX));
    let result = selected.clone();
    let (_, cx) = cx.add_window_view(move |_, _| MenuHarness(result));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        cx.debug_bounds("omarchy-menu-content").is_some(),
        "menu opened"
    );
    cx.simulate_keystrokes("down enter");
    assert_eq!(selected.get(), 2);
}

#[gpui::test]
fn links_open_urls_and_disabled_links_remain_inert(cx: &mut TestAppContext) {
    struct Links;
    impl Render for Links {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().flex_col().children([
                link(
                    "enabled",
                    "Project",
                    "https://github.com/huacnlee/gpui-omarchy",
                    cx,
                )
                .debug_selector(|| "enabled-link".into()),
                link(
                    "disabled",
                    "Unavailable",
                    "https://example.com/disabled",
                    cx,
                )
                .disabled(true)
                .debug_selector(|| "disabled-link".into()),
            ])
        }
    }
    cx.update(gpui_omarchy::init);
    let (_, cx) = cx.add_window_view(|_, _| Links);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let disabled = cx.debug_bounds("disabled-link").unwrap().center();
    cx.simulate_click(disabled, Modifiers::default());
    assert_eq!(cx.opened_url(), None);
    let enabled = cx.debug_bounds("enabled-link").unwrap().center();
    cx.simulate_click(enabled, Modifiers::default());
    assert_eq!(
        cx.opened_url().as_deref(),
        Some("https://github.com/huacnlee/gpui-omarchy")
    );
}
