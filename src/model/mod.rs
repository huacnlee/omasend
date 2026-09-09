use crate::localsend::Upload;
use anyhow::{Context, Result, ensure};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
pub mod history;
mod state;
pub use state::{AppState, ComposerItem, IncomingRequest, Transfer, TransferStatus};

#[derive(Clone, Debug)]
pub enum SendItem {
    File(PathBuf),
    TemporaryFile {
        path: Arc<tempfile::TempPath>,
        mime: String,
    },
    Text(String),
}

impl SendItem {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::TemporaryFile { path, .. } => Some(path.as_ref().as_ref()),
            Self::Text(_) => None,
        }
    }

    pub fn uploads(&self) -> Result<Vec<Upload>> {
        match self {
            Self::Text(text) => {
                ensure!(!text.is_empty(), "Clipboard text is empty");
                Ok(vec![Upload::text(text.clone())])
            }
            Self::TemporaryFile { path, mime } => {
                let file: &Path = path.as_ref().as_ref();
                let name = file
                    .file_name()
                    .context("Missing file name")?
                    .to_string_lossy()
                    .into_owned();
                let mut upload = Upload::file(file.into(), name)?;
                upload.offer.mime = mime.clone();
                upload.keep_alive = Some(path.clone());
                Ok(vec![upload])
            }
            Self::File(path) => {
                let metadata = std::fs::symlink_metadata(path)?;
                ensure!(
                    !metadata.file_type().is_symlink(),
                    "Choose the original file instead of a symbolic link"
                );
                if metadata.is_dir() {
                    let mut uploads = Vec::new();
                    expand_folder(path, path, &mut uploads, 0)?;
                    ensure!(
                        !uploads.is_empty(),
                        "This folder contains no files; LocalSend cannot transfer empty folders"
                    );
                    Ok(uploads)
                } else {
                    let name = path
                        .file_name()
                        .context("Missing file name")?
                        .to_str()
                        .context("File name is not valid UTF-8")?
                        .to_owned();
                    Ok(vec![Upload::file(path.clone(), name)?])
                }
            }
        }
    }
}

fn expand_folder(root: &Path, folder: &Path, result: &mut Vec<Upload>, depth: usize) -> Result<()> {
    ensure!(depth < 128, "Folder nesting exceeds 128 levels");
    let mut entries = std::fs::read_dir(folder)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        ensure!(
            result.len() < 10_000,
            "Select at most 10,000 files per transfer"
        );
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "Folder contains a symbolic link: {}",
            entry.path().display()
        );
        if kind.is_dir() {
            expand_folder(root, &entry.path(), result, depth + 1)?;
        } else {
            ensure!(
                kind.is_file(),
                "Folder contains a special file: {}",
                entry.path().display()
            );
            let path = entry.path();
            let name = path
                .strip_prefix(root)?
                .to_str()
                .context("File name is not valid UTF-8")?
                .replace(std::path::MAIN_SEPARATOR, "/");
            result.push(Upload::file(path, name)?);
        }
    }
    Ok(())
}
