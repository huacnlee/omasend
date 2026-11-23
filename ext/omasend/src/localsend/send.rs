use super::{Device, Events, OfferedFile, TransferEvent, emit};
use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use localsend::{
    crypto::cert::SelfSignedCert,
    http::{
        client::LsHttpClientV2,
        dto_v2::{PrepareUploadRequestDtoV2, RegisterDtoV2},
    },
    model::{
        discovery::ProtocolType,
        transfer::{FileDto, FileMetadata},
    },
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_util::{io::ReaderStream, sync::CancellationToken};

#[derive(Clone, Debug)]
pub enum UploadContent {
    File(PathBuf),
    Text(String),
}

#[derive(Clone, Debug)]
pub struct Upload {
    pub offer: OfferedFile,
    pub content: UploadContent,
    // Keep a clipboard file alive even if its composer item is removed mid-send.
    pub keep_alive: Option<Arc<tempfile::TempPath>>,
}

impl Upload {
    pub fn file(path: PathBuf, name: String) -> Result<Self> {
        super::receive::relative_path(&name)?;
        let metadata =
            std::fs::metadata(&path).with_context(|| format!("Cannot read {}", path.display()))?;
        ensure!(
            metadata.is_file(),
            "{} is not a regular file",
            path.display()
        );
        Ok(Self {
            offer: OfferedFile {
                name,
                size: metadata.len(),
                mime: mime_guess::from_path(&path)
                    .first_or_octet_stream()
                    .to_string(),
            },
            content: UploadContent::File(path),
            keep_alive: None,
        })
    }

    pub fn text(text: String) -> Self {
        Self {
            offer: OfferedFile {
                name: "Message.txt".into(),
                size: text.len() as u64,
                mime: "text/plain".into(),
            },
            content: UploadContent::Text(text),
            keep_alive: None,
        }
    }

    fn dto(&self, id: String) -> FileDto {
        FileDto {
            id,
            file_name: self.offer.name.clone(),
            size: self.offer.size,
            file_type: self.offer.mime.clone(),
            sha256: None,
            preview: match &self.content {
                UploadContent::Text(text) => Some(text.chars().take(500).collect()),
                _ => None,
            },
            metadata: match &self.content {
                UploadContent::File(path) => FileMetadata::from_path(path),
                _ => None,
            },
        }
    }
}

pub(crate) struct SendTransfer {
    pub id: String,
    pub peer: Device,
    pub identity: Arc<SelfSignedCert>,
    pub info: RegisterDtoV2,
    pub uploads: Vec<Upload>,
    pub pin: Option<String>,
    pub cancel: CancellationToken,
    pub remote_session: Arc<Mutex<Option<String>>>,
    pub events: Events,
}

impl SendTransfer {
    pub async fn run(self) -> Result<()> {
        ensure!(!self.uploads.is_empty(), "Add something to send first");
        let client = LsHttpClientV2::try_new(
            &self.identity.private_key_pem,
            &self.identity.certificate_pem,
            Some(self.peer.fingerprint.clone()),
            None,
        )?;
        let files: HashMap<_, _> = self
            .uploads
            .iter()
            .enumerate()
            .map(|(i, upload)| (i.to_string(), upload.dto(i.to_string())))
            .collect();
        let prepared = tokio::time::timeout(
            Duration::from_secs(300),
            client.prepare_upload(
                ProtocolType::Https,
                &self.peer.host,
                self.peer.port,
                None,
                PrepareUploadRequestDtoV2 {
                    info: self.info.clone(),
                    files,
                },
                self.pin.as_deref(),
                self.cancel.clone(),
            ),
        )
        .await
        .context("The receiver did not respond")??;
        let response = prepared
            .response
            .context("The receiver accepted no files")?;
        *self.remote_session.lock().unwrap() = Some(response.session_id.clone());
        let result = self
            .upload_all(&client, &response.session_id, response.files)
            .await;
        if result.is_err() {
            let _ = tokio::time::timeout(
                Duration::from_secs(3),
                client.cancel(
                    ProtocolType::Https,
                    &self.peer.host,
                    self.peer.port,
                    &response.session_id,
                ),
            )
            .await;
        }
        result
    }

    async fn upload_all(
        &self,
        client: &LsHttpClientV2,
        session: &str,
        accepted: HashMap<String, String>,
    ) -> Result<()> {
        ensure!(!accepted.is_empty(), "The receiver accepted no files");
        ensure!(
            accepted
                .keys()
                .all(|id| id.parse::<usize>().is_ok_and(|i| i < self.uploads.len())),
            "Receiver returned an unknown file ID"
        );
        let total = self
            .uploads
            .iter()
            .enumerate()
            .filter(|(i, _)| accepted.contains_key(&i.to_string()))
            .try_fold(0u64, |sum, (_, upload)| {
                sum.checked_add(upload.offer.size)
                    .context("Transfer is too large")
            })?;
        emit(
            &self.events,
            TransferEvent::Accepted {
                id: self.id.clone(),
            },
        );
        let mut transferred = 0u64;
        for (index, upload) in self.uploads.iter().enumerate() {
            let Some(token) = accepted.get(&index.to_string()) else {
                continue;
            };
            ensure!(!self.cancel.is_cancelled(), "Transfer cancelled");
            let body = match &upload.content {
                UploadContent::Text(text) => localsend::reqwest::Body::from(text.clone()),
                UploadContent::File(path) => {
                    let file = tokio::fs::File::open(path).await?;
                    ensure!(
                        file.metadata().await?.len() == upload.offer.size,
                        "{} changed since it was added",
                        upload.offer.name
                    );
                    let mut read = transferred;
                    let events = self.events.clone();
                    let id = self.id.clone();
                    let mut last = Instant::now();
                    let stream =
                        ReaderStream::with_capacity(file, 256 * 1024).inspect_ok(move |chunk| {
                            read += chunk.len() as u64;
                            if last.elapsed() >= Duration::from_millis(100) {
                                emit(
                                    &events,
                                    TransferEvent::Progress {
                                        id: id.clone(),
                                        transferred: read,
                                        total,
                                    },
                                );
                                last = Instant::now();
                            }
                        });
                    localsend::reqwest::Body::wrap_stream(stream)
                }
            };
            client
                .upload(
                    ProtocolType::Https,
                    &self.peer.host,
                    self.peer.port,
                    None,
                    session,
                    &index.to_string(),
                    token,
                    body,
                    self.cancel.clone(),
                )
                .await?;
            transferred += upload.offer.size;
            emit(
                &self.events,
                TransferEvent::Progress {
                    id: self.id.clone(),
                    transferred,
                    total,
                },
            );
        }
        // A partially accepted selection is visible as a failure instead of
        // silently deleting unsent composer items under a success event.
        ensure!(
            accepted.len() == self.uploads.len(),
            "Receiver accepted only {} of {} items",
            accepted.len(),
            self.uploads.len()
        );
        Ok(())
    }
}
