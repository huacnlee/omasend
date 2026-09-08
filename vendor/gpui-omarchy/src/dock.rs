//! Dock presentation. Layout, reconciliation and drag operations stay in base.
use crate::{ActiveTheme, ButtonVariant, button};
use gpui_kit::base::dock::*;
use gpui_kit::{
    AnyElement, App, Context, Div, SharedString, Stateful, Window, div, prelude::*, px,
};
use std::rc::Rc;

pub fn dock_area(
    id: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut Context<DockArea>,
) -> DockArea {
    DockArea::new(id, None, window, cx).with_renderer(Rc::new(OmarchyDock))
}

struct OmarchyDock;
impl DockAreaRenderer for OmarchyDock {
    fn frame(&self, _: &mut Window, cx: &mut App) -> Stateful<Div> {
        let t = cx.omarchy();
        div()
            .id("omarchy-dock")
            .border_1()
            .border_color(t.border)
            .bg(t.background)
            .font_family(t.font.clone())
            .text_size(px(12.))
            .text_color(t.foreground)
    }
    fn tiles_renderer(&self) -> Rc<dyn TilesRenderer> {
        Rc::new(OmarchyDock)
    }
    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(OmarchyDock)
    }
}

impl TabGroupRenderer for OmarchyDock {
    fn render_tab_bar(&self, group: &TabGroupContext, _: &mut Window, cx: &mut App) -> AnyElement {
        let t = cx.omarchy();
        let mut bar = div()
            .id("dock-tab-bar")
            .flex()
            .items_center()
            .gap(px(6.))
            .p(px(6.))
            .border_b_1()
            .border_color(t.border)
            .bg(t.normal_fill());
        for (index, panel) in group.panels().iter().enumerate() {
            let target = group.clone();
            let drop_target = group.clone();
            let label = panel.panel_name(cx);
            let tab = button(index, label, ButtonVariant::Outline, cx)
                .debug_selector(move || format!("dock-tab-{label}"))
                .selected(index == group.active_ix())
                .on_click(move |_, window, cx| target.select_tab(index, window, cx))
                .when(group.is_draggable(), |tab| {
                    tab.when_some(group.drag_panel(index, cx), |tab, drag| {
                        tab.on_drag(drag, move |_, _, _, cx| cx.new(|_| DockDragLabel(label)))
                    })
                })
                .when(group.is_droppable(), |tab| {
                    tab.on_drop(move |drag: &DragPanel, window, cx| {
                        drop_target.drop_panel(drag.clone(), Some(index), true, window, cx);
                    })
                });
            bar = bar.child(tab);
        }
        if let Some(panel) = group.active_panel() {
            let target = group.clone();
            let id = panel.panel_id(cx);
            if group.is_closable() && panel.closable(cx) {
                bar = bar.child(
                    button("dock-close", "Close", ButtonVariant::Secondary, cx)
                        .ml_auto()
                        .on_click(move |_, window, cx| target.close(id, window, cx)),
                );
            }
        }
        bar.into_any_element()
    }
    fn render_drop_indicator(
        &self,
        indicator: DropIndicator,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let hint = div()
            .absolute()
            .inset_0()
            .bg(cx.omarchy().accent.opacity(0.15))
            .border_1()
            .border_color(cx.omarchy().accent);
        Some(
            match indicator.placement() {
                Some(gpui_kit::base::Placement::Left) => hint.w(gpui_kit::relative(0.5)),
                Some(gpui_kit::base::Placement::Right) => hint.left(gpui_kit::relative(0.5)),
                Some(gpui_kit::base::Placement::Top) => hint.h(gpui_kit::relative(0.5)),
                Some(gpui_kit::base::Placement::Bottom) => hint.top(gpui_kit::relative(0.5)),
                _ => hint,
            }
            .into_any_element(),
        )
    }
}

struct DockDragLabel(&'static str);
impl gpui_kit::Render for DockDragLabel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(10.))
            .py(px(6.))
            .border_1()
            .border_color(cx.omarchy().accent)
            .bg(cx.omarchy().background)
            .text_color(cx.omarchy().foreground)
            .child(self.0)
    }
}

// Required by base's renderer contract; this skin offers tabbed/split docks only.
impl TilesRenderer for OmarchyDock {
    fn render_drag_bar(&self, _: &TileContext, _: &mut Window, _: &mut App) -> AnyElement {
        gpui_kit::Empty.into_any_element()
    }
}
