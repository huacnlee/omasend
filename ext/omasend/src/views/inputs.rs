//! Native clipboard MIME priority: files, image, then text. Wayland keeps the
//! existing MIME-aware adapter, including video and URI-list support.
use anyhow::{Context, Result, ensure};
use omasend::{
    clipboard::{ClipboardKind, read_offer},
    model::SendItem,
};
use std::io::Cursor;

pub(crate) fn paste() -> Result<Vec<SendItem>> {
    let mut clipboard = arboard::Clipboard::new().context("Cannot open the system clipboard")?;
    if let Ok(paths) = clipboard.get().file_list()
        && !paths.is_empty()
    {
        return Ok(paths.into_iter().map(SendItem::File).collect());
    }
    #[cfg(target_os = "macos")]
    if let Some(items) = mac_media()? {
        return Ok(items);
    }
    if let Ok(image) = clipboard.get_image() {
        let width = u32::try_from(image.width).context("Clipboard image is too wide")?;
        let height = u32::try_from(image.height).context("Clipboard image is too tall")?;
        let rgba = image::RgbaImage::from_raw(width, height, image.bytes.into_owned())
            .context("Clipboard image has an invalid pixel buffer")?;
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(rgba).write_to(&mut png, image::ImageFormat::Png)?;
        return read_offer(
            ClipboardKind::Image("image/png".into()),
            png.get_ref(),
            &std::env::temp_dir(),
        );
    }
    let text = clipboard
        .get_text()
        .context("Clipboard has no supported files, images or text")?;
    ensure!(
        !text.is_empty(),
        "Clipboard is empty; copy the content again"
    );
    Ok(vec![SendItem::Text(text)])
}

// arboard's macOS image reader accepts TIFF only. Read advertised binary
// formats directly before that fallback, keeping PNG/JPEG/video payloads exact.
#[cfg(target_os = "macos")]
fn mac_media() -> Result<Option<Vec<SendItem>>> {
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::NSString;
    let board = NSPasteboard::generalPasteboard();
    let revision = board.changeCount();
    let types = board
        .types()
        .map(|types| {
            types
                .iter()
                .map(|kind| kind.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (kind, mime, video) in [
        ("public.png", "image/png", false),
        ("public.jpeg", "image/jpeg", false),
        ("org.webmproject.webp", "image/webp", false),
        ("com.compuserve.gif", "image/gif", false),
        ("public.tiff", "image/tiff", false),
        ("public.mpeg-4", "video/mp4", true),
        ("com.apple.quicktime-movie", "video/quicktime", true),
        ("org.webmproject.webm", "video/webm", true),
    ] {
        if !types.iter().any(|candidate| candidate == kind) {
            continue;
        }
        let data = board
            .dataForType(&NSString::from_str(kind))
            .context("Clipboard media changed; copy it again")?;
        let bytes = data.to_vec();
        ensure!(
            board.changeCount() == revision,
            "Clipboard changed; copy the content again"
        );
        let kind = if video {
            ClipboardKind::Video(mime.into())
        } else {
            ClipboardKind::Image(mime.into())
        };
        return read_offer(kind, &bytes, &std::env::temp_dir()).map(Some);
    }
    Ok(None)
}
