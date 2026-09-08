use crate::{ActiveTheme, IconName, icon};
use gpui_kit::base::{Tree, TreeState};
use gpui_kit::{App, Entity, div, prelude::*, px};

/// Virtualized, keyboard-navigable tree with application-owned base state.
pub fn tree(state: &Entity<TreeState>, cx: &App) -> Tree {
    let t = cx.omarchy().clone();
    Tree::new(state)
        .h(px(280.))
        .w_full()
        .list_style(div().size_full().style().clone())
        .item(move |_, entry, state, _, _| {
            let id = entry.item().id.clone();
            div()
                .debug_selector(move || format!("omarchy-tree-{id}"))
                .flex()
                .items_center()
                .gap(px(6.))
                .h(px(28.))
                .pl(px(6. + entry.depth() as f32 * 20.))
                .pr(px(8.))
                .font_family(t.font.clone())
                .text_size(px(12.))
                .text_color(t.foreground)
                .bg(if state.is_selected() {
                    t.selected_fill()
                } else {
                    t.foreground.opacity(0.)
                })
                .when(entry.is_disabled(), |row| row.opacity(0.45))
                .when(!entry.is_disabled(), |row| {
                    row.hover(|s| s.bg(t.hover_fill()))
                })
                .when(entry.is_folder(), |row| {
                    row.child(
                        icon(if entry.is_expanded() {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(px(14.))
                        .flex_shrink_0(),
                    )
                })
                .child(div().min_w_0().truncate().child(entry.item().label.clone()))
                .into_any_element()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::base::TreeItem;
    use gpui_kit::gpui;
    use gpui_kit::{Context, Render, TestAppContext, Window};
    struct Harness {
        state: Entity<TreeState>,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            crate::focus_scope("tree-test")
                .size_full()
                .child(tree(&self.state, cx))
        }
    }
    #[gpui::test]
    fn pointer_selection_hands_off_to_keyboard_navigation(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|_, cx| Harness {
            state: cx.new(|cx| {
                TreeState::new(cx).items(vec![
                    TreeItem::new("folder", "Documents")
                        .expanded(true)
                        .child(TreeItem::new("first", "Brief.md"))
                        .child(TreeItem::new("second", "Notes.md")),
                ])
            }),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let first = cx.debug_bounds("omarchy-tree-first").unwrap().center();
        cx.simulate_click(first, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("down");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx)
                    .state
                    .read(cx)
                    .selected_item()
                    .unwrap()
                    .id
                    .as_ref(),
                "second"
            )
        });
        let folder = cx.debug_bounds("omarchy-tree-folder").unwrap().center();
        cx.simulate_click(folder, Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("omarchy-tree-first").is_none());
        cx.simulate_keystrokes("right");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("omarchy-tree-first").is_some());
    }
}
