use super::{ClipboardKind, read_offer, select_mime, temporary_file};
use crate::model::SendItem;
use anyhow::{Context, Result, ensure};
use std::{path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};

fn runtime_directory() -> Result<PathBuf> {
    let root = std::env::var_os("XDG_RUNTIME_DIR")
        .context("XDG_RUNTIME_DIR is not set; start OmaSend in your Wayland session")?;
    let directory = PathBuf::from(root).join("omasend");
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = std::fs::symlink_metadata(&directory)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "Clipboard temporary directory is not a directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "Clipboard temporary directory must be private (mode 700)"
        );
    }
    Ok(directory)
}

/// Enumerate first; never reinterpret arbitrary clipboard bytes as text.
/// Media goes straight from wl-paste to a private file, so a recording does
/// not consume its entire size in application memory.
pub async fn paste() -> Result<Vec<SendItem>> {
    let types = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("wl-paste")
            .arg("--list-types")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("Clipboard did not respond")?
    .context("Install wl-clipboard to paste from Wayland")?;
    ensure!(
        types.status.success(),
        "Cannot read clipboard; copy something first"
    );
    let types = String::from_utf8(types.stdout)?;
    let kind = select_mime(&types.lines().collect::<Vec<_>>())?;
    let mut command = Command::new("wl-paste");
    command
        .args(["--no-newline", "--type", kind.mime()])
        .kill_on_drop(true)
        .stderr(Stdio::null());
    if matches!(kind, ClipboardKind::Image(_) | ClipboardKind::Video(_)) {
        let runtime = runtime_directory()?;
        let file = temporary_file(&kind, &runtime)?;
        command.stdout(Stdio::from(file.reopen()?));
        let status = tokio::time::timeout(Duration::from_secs(120), command.status())
            .await
            .context("Clipboard media read timed out")??;
        ensure!(
            status.success() && file.as_file().metadata()?.len() > 0,
            "Clipboard media is empty or changed; copy it again"
        );
        Ok(vec![SendItem::TemporaryFile {
            path: Arc::new(file.into_temp_path()),
            mime: kind.mime().into(),
        }])
    } else {
        let mut child = command.stdout(Stdio::piped()).spawn()?;
        let mut output = child
            .stdout
            .take()
            .context("Cannot read clipboard output")?
            .take(16 * 1024 * 1024 + 1);
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(10), output.read_to_end(&mut bytes))
            .await
            .context("Clipboard text read timed out")??;
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "Clipboard text exceeds 16 MB; send it as a file"
        );
        ensure!(
            tokio::time::timeout(Duration::from_secs(5), child.wait())
                .await??
                .success(),
            "Clipboard changed; copy the content again"
        );
        // Text and file URIs never materialize temporary files.
        read_offer(kind, &bytes, &std::env::temp_dir())
    }
}
