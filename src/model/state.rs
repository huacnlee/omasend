use super::SendItem;
use crate::localsend::{Device, OfferedFile, TransferEvent, Upload};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

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

#[derive(Default)]
struct TransferRate {
    sample: Option<(Instant, u64)>,
    started: Option<Instant>,
    average: Option<u64>,
    bytes_per_second: u64,
}

impl TransferRate {
    fn finish(&mut self, bytes: u64, now: Instant) {
        self.average = self.started.and_then(|started| {
            let elapsed = now.saturating_duration_since(started).as_secs_f64();
            (elapsed > 0.).then(|| (bytes as f64 / elapsed) as u64)
        });
    }

    fn update(&mut self, bytes: u64, now: Instant) {
        self.started.get_or_insert(now);
        let Some((start, previous)) = self.sample else {
            self.sample = Some((now, bytes));
            return;
        };
        let elapsed = now.duration_since(start);
        if elapsed >= Duration::from_millis(500) {
            self.bytes_per_second =
                (bytes.saturating_sub(previous) as f64 / elapsed.as_secs_f64()) as u64;
            self.sample = Some((now, bytes));
        }
    }
}

pub struct Transfer {
    pub id: String,
    pub peer: String,
    peer_id: Option<String>,
    pub sending: bool,
    pub awaiting_acceptance: bool,
    pub files: Vec<OfferedFile>,
    pub transferred: u64,
    pub total: u64,
    pub paths: Vec<PathBuf>,
    pub status: TransferStatus,
    pub when: SystemTime,
    pub composer_ids: HashSet<String>,
    rate: TransferRate,
}

impl Transfer {
    pub fn average_bytes_per_second(&self) -> Option<u64> {
        self.rate.average
    }

    pub fn bytes_per_second(&self) -> u64 {
        self.rate.bytes_per_second
    }
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
    expired_devices: HashSet<String>,
}

impl AppState {
    pub fn clear_transfer_history(&mut self) {
        self.transfers
            .retain(|transfer| transfer.status == TransferStatus::Active);
    }

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
            peer_id: self.selected.clone(),
            sending: true,
            awaiting_acceptance: true,
            files: self
                .composer
                .iter()
                .flat_map(|item| item.uploads.iter().map(|file| file.offer.clone()))
                .collect(),
            transferred: 0,
            rate: TransferRate::default(),
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
                self.expired_devices.remove(&device.fingerprint);
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
                self.expired_devices.insert(id);
                self.remove_expired_devices();
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
                let peer_id = self
                    .incoming
                    .as_ref()
                    .filter(|request| request.id == id)
                    .map(|request| request.peer.fingerprint.clone());
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
                        peer_id,
                        sending,
                        awaiting_acceptance: sending,
                        files,
                        total,
                        transferred: 0,
                        rate: TransferRate {
                            started: (!sending).then(Instant::now),
                            ..Default::default()
                        },
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
                    transfer.rate.update(0, Instant::now());
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
                    transfer.rate.update(transfer.transferred, Instant::now());
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
                tracing::warn!(transfer_id = %id, error = %error, "Transfer failed");
                if !self.transfers.iter().any(|transfer| transfer.id == id) {
                    self.error = Some(error.clone());
                }
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
    fn remove_expired_devices(&mut self) {
        self.devices.retain(|device| {
            !self.expired_devices.contains(&device.fingerprint)
                || self.transfers.iter().any(|transfer| {
                    transfer.status == TransferStatus::Active
                        && transfer.peer_id.as_ref() == Some(&device.fingerprint)
                })
        });
        if !self
            .devices
            .iter()
            .any(|device| Some(&device.fingerprint) == self.selected.as_ref())
        {
            self.selected = self
                .devices
                .first()
                .map(|device| device.fingerprint.clone());
        }
        self.expired_devices
            .retain(|id| self.devices.iter().any(|device| &device.fingerprint == id));
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
                transfer.rate.finish(transfer.total, Instant::now());
                self.composer
                    .retain(|item| !transfer.composer_ids.contains(&item.id));
            }
            transfer.status = status;
            transfer.paths = paths;
            transfer.when = SystemTime::now();
        }
        self.remove_expired_devices();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_rate_averages_the_whole_transfer_and_stays_fixed() {
        let start = Instant::now();
        let mut rate = TransferRate::default();
        assert_eq!(rate.average, None);
        rate.update(0, start);
        rate.update(1024, start + Duration::from_secs(1));
        rate.update(3072, start + Duration::from_secs(2));
        rate.finish(4096, start + Duration::from_secs(4));
        assert_eq!(rate.average, Some(1024));
        rate.update(4096, start + Duration::from_secs(10));
        assert_eq!(rate.average, Some(1024));
        let mut unmeasured = TransferRate::default();
        unmeasured.finish(4096, start);
        assert_eq!(unmeasured.average, None);
    }

    #[test]
    fn transfer_rate_uses_recent_bytes_and_handles_stalls() {
        let start = Instant::now();
        let mut rate = TransferRate::default();
        rate.update(0, start);
        rate.update(1024, start + Duration::from_millis(500));
        assert_eq!(rate.bytes_per_second, 2048);
        rate.update(1536, start + Duration::from_secs(1));
        assert_eq!(rate.bytes_per_second, 1024);
        rate.update(1536, start + Duration::from_secs(2));
        assert_eq!(rate.bytes_per_second, 0);
        rate.update(0, start + Duration::from_secs(3));
        assert_eq!(rate.bytes_per_second, 0);
    }
}
