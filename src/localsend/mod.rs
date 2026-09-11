mod adapter;
mod receive;
mod send;

pub use adapter::{Handle, Node, NodeConfig};
pub use send::Upload;

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Device {
    pub fingerprint: String,
    pub alias: String,
    pub model: String,
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug)]
pub struct OfferedFile {
    pub name: String,
    pub size: u64,
    pub mime: String,
}

#[derive(Clone, Debug)]
pub enum TransferEvent {
    DiscoveryActive(bool),
    DeviceFound(Device),
    DeviceLost(String),
    NetworkError(String),
    IncomingRequest {
        id: String,
        peer: Device,
        files: Vec<OfferedFile>,
    },
    Started {
        id: String,
        peer: String,
        sending: bool,
        files: Vec<OfferedFile>,
        total: u64,
    },
    Accepted {
        id: String,
    },
    Progress {
        id: String,
        transferred: u64,
        total: u64,
    },
    ReceivedText {
        id: String,
        text: String,
    },
    Completed {
        id: String,
        paths: Vec<PathBuf>,
    },
    Cancelled {
        id: String,
    },
    Failed {
        id: String,
        error: String,
    },
}

pub(crate) type Events = async_channel::Sender<TransferEvent>;

pub(crate) fn emit(events: &Events, event: TransferEvent) {
    // Terminal and decision events must never be dropped. Progress producers
    // throttle before reaching this channel; the GUI receives without polling.
    let _ = events.try_send(event);
}

pub fn downloads_directory() -> anyhow::Result<PathBuf> {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
        .ok_or_else(|| anyhow::anyhow!("Cannot locate your Downloads folder"))
}
