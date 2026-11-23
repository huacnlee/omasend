pub mod wayland;

use crate::model::SendItem;
use anyhow::{Context, Result, bail, ensure};
use std::{io::Write, path::Path, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardKind {
    Files,
    Image(String),
    Video(String),
    Text(String),
}

impl ClipboardKind {
    pub fn mime(&self) -> &str {
        match self {
            Self::Files => "text/uri-list",
            Self::Image(mime) | Self::Video(mime) | Self::Text(mime) => mime,
        }
    }

    pub(crate) fn extension(&self) -> Option<&'static str> {
        match self.mime() {
            "image/png" => Some("png"),
            "image/jpeg" => Some("jpg"),
            "image/webp" => Some("webp"),
            "image/gif" => Some("gif"),
            "image/bmp" => Some("bmp"),
            "image/tiff" => Some("tiff"),
            "video/mp4" => Some("mp4"),
            "video/webm" => Some("webm"),
            "video/quicktime" => Some("mov"),
            "video/x-matroska" => Some("mkv"),
            _ => None,
        }
    }
}

pub fn select_mime(types: &[&str]) -> Result<ClipboardKind> {
    if types.contains(&"text/uri-list") {
        return Ok(ClipboardKind::Files);
    }
    for mime in [
        "image/png",
        "image/jpeg",
        "image/webp",
        "image/gif",
        "image/bmp",
        "image/tiff",
    ] {
        if types.contains(&mime) {
            return Ok(ClipboardKind::Image(mime.into()));
        }
    }
    for mime in [
        "video/mp4",
        "video/webm",
        "video/quicktime",
        "video/x-matroska",
    ] {
        if types.contains(&mime) {
            return Ok(ClipboardKind::Video(mime.into()));
        }
    }
    for mime in ["text/plain;charset=utf-8", "text/plain", "UTF8_STRING"] {
        if types.contains(&mime) {
            return Ok(ClipboardKind::Text(mime.into()));
        }
    }
    bail!("Clipboard has no supported files, images, videos or text")
}

pub fn read_offer(kind: ClipboardKind, data: &[u8], runtime: &Path) -> Result<Vec<SendItem>> {
    ensure!(
        !data.is_empty(),
        "Clipboard is empty; copy the content again"
    );
    match &kind {
        ClipboardKind::Files => {
            let value = std::str::from_utf8(data).context("File clipboard is not UTF-8")?;
            let mut items = Vec::new();
            for line in value
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
            {
                let uri = url::Url::parse(line).context("Invalid clipboard file URI")?;
                ensure!(
                    uri.scheme() == "file" && uri.host_str().is_none_or(|host| host == "localhost"),
                    "Clipboard URI is not a local file"
                );
                ensure!(
                    uri.query().is_none() && uri.fragment().is_none(),
                    "File URI has a query or fragment"
                );
                let path = uri
                    .to_file_path()
                    .map_err(|_| anyhow::anyhow!("Invalid clipboard file path"))?;
                ensure!(
                    path.exists(),
                    "Clipboard file no longer exists: {}",
                    path.display()
                );
                items.push(SendItem::File(path));
            }
            ensure!(!items.is_empty(), "Clipboard contains no local files");
            Ok(items)
        }
        ClipboardKind::Text(_) => Ok(vec![SendItem::Text(
            std::str::from_utf8(data)
                .context("Clipboard text is not UTF-8")?
                .to_owned(),
        )]),
        ClipboardKind::Image(_) | ClipboardKind::Video(_) => {
            let mut file = temporary_file(&kind, runtime)?;
            file.write_all(data)?;
            Ok(vec![SendItem::TemporaryFile {
                path: Arc::new(file.into_temp_path()),
                mime: kind.mime().into(),
            }])
        }
    }
}

pub(crate) fn temporary_file(
    kind: &ClipboardKind,
    runtime: &Path,
) -> Result<tempfile::NamedTempFile> {
    let extension = kind
        .extension()
        .context("Unsupported clipboard media type")?;
    let prefix = if matches!(kind, ClipboardKind::Image(_)) {
        "screenshot-"
    } else {
        "video-"
    };
    Ok(tempfile::Builder::new()
        .prefix(prefix)
        .suffix(&format!(".{extension}"))
        .tempfile_in(runtime)?)
}
