use super::{Events, TransferEvent, emit};
use anyhow::{Context, Result, bail, ensure};
use bytes::Bytes;
use cap_std::{ambient_authority, fs::Dir};
use localsend::{
    crypto::hash::sha256_file_content,
    model::transfer::{FileContent, FileDto},
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, sync::mpsc};
use tokio_util::sync::CancellationToken;

pub(crate) fn relative_path(name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty() && !name.contains(['\\', ':', '\0']),
        "Invalid file name"
    );
    ensure!(
        name.split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".."),
        "Unsafe relative file path"
    );
    let path = PathBuf::from(name);
    ensure!(!path.is_absolute(), "Absolute file paths are not accepted");
    Ok(path)
}

/// Publish by hard-linking beneath an open directory capability. This is atomic
/// and never replaces an existing name (including a symlink). Capability-based
/// path resolution also prevents a raced parent symlink from escaping Downloads.
fn publish(downloads: &Path, staging: &Path, name: &str) -> Result<PathBuf> {
    let relative = relative_path(name)?;
    let root = Dir::open_ambient_dir(downloads, ambient_authority())?;
    let parent = relative
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    root.create_dir_all(parent)?;
    let target = root.open_dir(parent)?;
    let filename = relative.file_name().context("Missing file name")?;
    let stem = relative
        .file_stem()
        .context("Missing file stem")?
        .to_string_lossy();
    let extension = relative
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    let source = staging.file_name().context("Missing staging name")?;
    for index in 0..100_000 {
        let candidate = if index == 0 {
            filename.to_os_string()
        } else {
            format!("{stem} ({index}){extension}").into()
        };
        match root.hard_link(source, &target, &candidate) {
            Ok(()) => {
                return Ok(downloads
                    .join(relative.parent().unwrap_or(Path::new("")))
                    .join(candidate));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    bail!("Too many files share the name {name}")
}

pub(crate) struct ReceiveFile {
    pub downloads: PathBuf,
    pub session: String,
    pub file: FileDto,
    pub cancel: CancellationToken,
    pub events: Events,
    pub progress_tx: mpsc::UnboundedSender<(String, String, u64)>,
}

impl ReceiveFile {
    pub async fn save(self, mut chunks: mpsc::Receiver<Bytes>) -> Result<PathBuf> {
        relative_path(&self.file.file_name)?;
        let temporary = tempfile::Builder::new()
            .prefix(".omasend-")
            .tempfile_in(&self.downloads)?;
        let mut output = tokio::fs::File::from_std(temporary.reopen()?);
        let mut size = 0u64;
        let mut last = Instant::now();
        loop {
            let chunk = tokio::select! {
                biased;
                _ = self.cancel.cancelled() => bail!("Transfer cancelled"),
                chunk = tokio::time::timeout(Duration::from_secs(60), chunks.recv()) => chunk.context("Sender stopped responding")?,
            };
            let Some(chunk) = chunk else { break };
            size = size
                .checked_add(chunk.len() as u64)
                .context("File size overflow")?;
            ensure!(size <= self.file.size, "Received more data than offered");
            output.write_all(&chunk).await?;
            if last.elapsed() >= Duration::from_millis(100) || size == self.file.size {
                let _ = self
                    .progress_tx
                    .send((self.session.clone(), self.file.id.clone(), size));
                last = Instant::now();
            }
        }
        ensure!(
            size == self.file.size,
            "Incomplete file: received {size} of {} bytes",
            self.file.size
        );
        output.flush().await?;
        output.sync_all().await?;
        drop(output);
        if let Some(expected) = &self.file.sha256 {
            let actual = sha256_file_content(
                FileContent::Path(temporary.path().into()),
                &self.cancel,
                |_| {},
            )
            .await?;
            ensure!(
                actual.eq_ignore_ascii_case(expected),
                "File checksum did not match"
            );
        }
        ensure!(!self.cancel.is_cancelled(), "Transfer cancelled");
        if let Some(metadata) = &self.file.metadata {
            let mut times = std::fs::FileTimes::new();
            if let Some(time) = metadata.modified_time() {
                times = times.set_modified(time);
            }
            if let Some(time) = metadata.accessed_time() {
                times = times.set_accessed(time);
            }
            temporary.as_file().set_times(times)?;
        }
        let result = publish(&self.downloads, temporary.path(), &self.file.file_name);
        if let Err(error) = &result {
            emit(
                &self.events,
                TransferEvent::NetworkError(format!(
                    "Could not save {}: {error}",
                    self.file.file_name
                )),
            );
        }
        result
    }
}
