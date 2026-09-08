use super::SendItem;
use crate::localsend::{Device, OfferedFile, TransferEvent, Upload};
use std::{collections::HashSet, path::PathBuf, time::SystemTime};

pub struct ComposerItem {
    pub id: String,
    pub item: SendItem,
    pub uploads: Vec<Upload>,
}

impl ComposerItem {
    pub fn new(item: SendItem) -> anyhow::Result<Self> {
        let uploads = item.uploads()?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            item,
            uploads,
        })
    }
    pub fn name(&self) -> String {
        match &self.item {
            SendItem::Text(text) => text
                .lines()
                .next()
                .unwrap_or("Text")
                .chars()
                .take(100)
                .collect(),
            _ => self
                .item
                .path()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "File".into()),
        }
    }
    pub fn size(&self) -> u64 {
        self.uploads.iter().map(|file| file.offer.size).sum()
    }
    pub fn is_image(&self) -> bool {
        self.uploads.len() == 1 && self.uploads[0].offer.mime.starts_with("image/")
    }
    pub fn is_video(&self) -> bool {
        self.uploads.len() == 1 && self.uploads[0].offer.mime.starts_with("video/")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransferStatus {
    Active,
    Completed,
    Cancelled,
    Failed(String),
}

pub struct Transfer {
    pub id: String,
    pub peer: String,
    pub sending: bool,
    pub awaiting_acceptance: bool,
    pub files: Vec<OfferedFile>,
    pub transferred: u64,
    pub total: u64,
    pub paths: Vec<PathBuf>,
    pub status: TransferStatus,
    pub when: SystemTime,
    pub composer_ids: HashSet<String>,
}

pub struct IncomingRequest {
    pub id: String,
    pub peer: Device,
    pub files: Vec<OfferedFile>,
}

#[derive(Default)]
pub struct AppState {
    pub discovering: bool,
    pub discovery_revision: u64,
    pub devices: Vec<Device>,
    pub selected: Option<String>,
    pub composer: Vec<ComposerItem>,
    pub transfers: Vec<Transfer>,
    pub incoming: Option<IncomingRequest>,
    pub error: Option<String>,
}

impl AppState {
    pub fn selected_device(&self) -> Option<&Device> {
        self.devices
            .iter()
            .find(|device| Some(&device.fingerprint) == self.selected.as_ref())
    }
    pub fn sending(&self) -> bool {
        self.transfers
            .iter()
            .any(|transfer| transfer.sending && transfer.status == TransferStatus::Active)
    }
    pub fn navigate(&mut self, delta: isize) {
        if self.devices.is_empty() {
            self.selected = None;
            return;
        }
        let index = self
            .devices
            .iter()
            .position(|device| Some(&device.fingerprint) == self.selected.as_ref())
            .unwrap_or(0);
        let index = (index as isize + delta).rem_euclid(self.devices.len() as isize) as usize;
        self.selected = Some(self.devices[index].fingerprint.clone());
    }
    pub fn track_send(&mut self, id: String) {
        self.transfers.push(Transfer {
            id,
            peer: self
                .selected_device()
                .map(|device| device.alias.clone())
                .unwrap_or_default(),
            sending: true,
            awaiting_acceptance: true,
            files: self
                .composer
                .iter()
                .flat_map(|item| item.uploads.iter().map(|file| file.offer.clone()))
                .collect(),
            transferred: 0,
            total: self.composer.iter().map(ComposerItem::size).sum(),
            paths: Vec::new(),
            status: TransferStatus::Active,
            when: SystemTime::now(),
            composer_ids: self.composer.iter().map(|item| item.id.clone()).collect(),
        });
    }
    pub fn apply(&mut self, event: TransferEvent) {
        match event {
            TransferEvent::DiscoveryActive(active) => self.discovering = active,
            TransferEvent::DeviceFound(device) => {
                if let Some(existing) = self
                    .devices
                    .iter_mut()
                    .find(|existing| existing.fingerprint == device.fingerprint)
                {
                    *existing = device
                } else {
                    self.discovery_revision += 1;
                    self.devices.push(device)
                }
                if self.selected.is_none() {
                    self.selected = self
                        .devices
                        .first()
                        .map(|device| device.fingerprint.clone());
                }
            }
            TransferEvent::DeviceLost(id) => {
                self.devices.retain(|device| device.fingerprint != id);
                if self.selected.as_ref() == Some(&id) {
                    self.selected = self
                        .devices
                        .first()
                        .map(|device| device.fingerprint.clone());
                }
            }
            TransferEvent::NetworkError(error) => self.error = Some(error),
            TransferEvent::IncomingRequest { id, peer, files } => {
                self.incoming = Some(IncomingRequest { id, peer, files })
            }
            TransferEvent::Started {
                id,
                peer,
                sending,
                files,
                total,
            } => {
                if self
                    .incoming
                    .as_ref()
                    .is_some_and(|request| request.id == id)
                {
                    self.incoming = None;
                }
                if let Some(transfer) = self.transfers.iter_mut().find(|transfer| transfer.id == id)
                {
                    transfer.peer = peer;
                    transfer.files = files;
                    transfer.total = total;
                } else {
                    self.transfers.push(Transfer {
                        id,
                        peer,
                        sending,
                        awaiting_acceptance: sending,
                        files,
                        total,
                        transferred: 0,
                        paths: Vec::new(),
                        status: TransferStatus::Active,
                        when: SystemTime::now(),
                        composer_ids: HashSet::new(),
                    });
                }
            }
            TransferEvent::Accepted { id } => {
                if let Some(transfer) = self
                    .transfers
                    .iter_mut()
                    .find(|transfer| transfer.id == id && transfer.status == TransferStatus::Active)
                {
                    transfer.awaiting_acceptance = false;
                }
            }
            TransferEvent::Progress {
                id,
                transferred,
                total,
            } => {
                if let Some(transfer) = self
                    .transfers
                    .iter_mut()
                    .find(|transfer| transfer.id == id && transfer.status == TransferStatus::Active)
                {
                    transfer.transferred = transferred.min(total);
                    transfer.total = total;
                }
            }
            TransferEvent::Completed { id, paths } => {
                self.finish(&id, TransferStatus::Completed, paths)
            }
            TransferEvent::Cancelled { id } => {
                self.finish(&id, TransferStatus::Cancelled, Vec::new())
            }
            TransferEvent::Failed { id, error } => {
                self.error = Some(error.clone());
                self.finish(&id, TransferStatus::Failed(error), Vec::new());
            }
        }
        // Session history only. Preserve active work while bounding completed rows.
        if self.transfers.len() > 100
            && let Some(index) = self
                .transfers
                .iter()
                .position(|transfer| transfer.status != TransferStatus::Active)
        {
            self.transfers.remove(index);
        }
    }
    fn finish(&mut self, id: &str, status: TransferStatus, paths: Vec<PathBuf>) {
        if self
            .incoming
            .as_ref()
            .is_some_and(|request| request.id == id)
        {
            self.incoming = None;
        }
        if let Some(transfer) = self.transfers.iter_mut().find(|transfer| transfer.id == id) {
            if status == TransferStatus::Completed {
                transfer.transferred = transfer.total;
                self.composer
                    .retain(|item| !transfer.composer_ids.contains(&item.id));
            }
            transfer.status = status;
            transfer.paths = paths;
            transfer.when = SystemTime::now();
        }
    }
}
