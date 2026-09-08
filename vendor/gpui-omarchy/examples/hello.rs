use gpui_kit::{
    AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, WindowOptions,
};
use gpui_omarchy::{ActiveTheme, ButtonVariant, button, focus_scope, panel};

struct Hello {
    clicks: usize,
}

impl Render for Hello {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        focus_scope("hello")
            .size_full()
            .bg(cx.omarchy().background)
            .child(
                panel("Welcome to Omarchy", cx).child(
                    button(
                        "hello",
                        format!("Clicked {} times", self.clicks),
                        ButtonVariant::Primary,
                        cx,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })),
                ),
            )
    }
}

fn main() {
    gpui_omarchy::application().run(|cx| {
        gpui_omarchy::init(cx);
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|_| Hello { clicks: 0 })
        })
        .expect("open window");
        cx.activate(true);
    });
}
