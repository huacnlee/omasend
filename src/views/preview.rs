//! Previews retain their SendItem, including clipboard temporary-file ownership.
use super::appearance::theme;
use super::{control, session::Session};
use nsh_core::ui::prelude::*;
use omasend::model::SendItem;

#[derive(Clone, Debug)]
pub(super) enum Preview {
    Image(SendItem),
    Text(String),
}

pub(super) fn overlay(session: Session) -> impl IntoElement {
    let selected = session.preview;
    let closer = session.clone();
    when(
        move || selected.get().is_some(),
        move || {
            let close = closer.clone();
            let title = closer.translated("action.preview");
            let content = match selected.get_untracked().expect("preview is open") {
                Preview::Text(text) => super::editor()
                    .value(text)
                    .readonly(true)
                    .multiline(true)
                    .flex_1()
                    .label(closer.translated("message.title"))
                    .into_element(),
                Preview::Image(item) => {
                    let path = item.path().expect("image has a path");
                    let uri = url::Url::from_file_path(path)
                        .map(|uri| uri.to_string())
                        .unwrap_or_else(|_| path.to_string_lossy().into_owned());
                    let loaded = animated_value(if reduce_motion() { 1.0 } else { 0.0 });
                    let fade = loaded;
                    let error = signal(String::new());
                    // The modal's signal holds `item` until the preview closes.
                    view()
                        .flex_1()
                        .gap_2()
                        .child(
                            image(uri)
                                .resize_mode(nsh_core::ResizeMode::Contain)
                                .flex_1()
                                .bind_opacity(&loaded)
                                .on_load(move |_: &nsh_core::Size<nsh_core::Points>| {
                                    fade.timing(
                                        1.0,
                                        if reduce_motion() { 0.0 } else { 140.0 },
                                        Easing::Linear,
                                    );
                                })
                                .on_error(move |reason: &str| error.set(reason.to_owned())),
                        )
                        .child(text(move || error.get()).text_color(move || theme().danger()))
                        .into_element()
                }
            };
            nsh_base::dialog()
                .animation(if reduce_motion() {
                    ModalAnimation::None
                } else {
                    ModalAnimation::Fade
                })
                .on_cancel(move |_: &()| selected.set(None))
                .child(
                    view()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .p_6()
                        .bg(AdaptiveColor::from(rgb(0)).opacity(0.6))
                        .child(
                            view()
                                .flex_1()
                                .w_full()
                                .max_w(px(720.0))
                                .gap_4()
                                .p_4()
                                .border_1()
                                .border_color(move || theme().border())
                                .bg(move || theme().background())
                                .child(
                                    view()
                                        .flex_row()
                                        .gap_2()
                                        .items_center()
                                        .child(text(title).flex_1())
                                        .child(control(
                                            closer.translated("action.close"),
                                            false,
                                            move || close.preview.set(None),
                                        )),
                                )
                                .child(content),
                        ),
                )
        },
    )
}
