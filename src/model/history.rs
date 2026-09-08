//! Transfer history that outlives the session.
//!
//! Only finished transfers are stored: an interrupted one cannot be resumed
//! after a restart, so recording it would only leave a row nobody can act on.
//! Received paths are checked when the file is read, so a record never offers
//! to open something that has since been deleted.
use super::{Transfer, TransferStatus};
use crate::localsend::OfferedFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

/// Enough to look back over weeks of use without letting the file grow without
/// bound. The newest records are the ones worth keeping.
const LIMIT: usize = 100;

#[derive(Serialize, Deserialize)]
struct Record {
    id: String,
    peer: String,
    sending: bool,
    files: Vec<File>,
    total: u64,
    paths: Vec<PathBuf>,
    outcome: Outcome,
    #[serde(default)]
    error: Option<String>,
    /// Seconds since the Unix epoch.
    when: u64,
}

#[derive(Serialize, Deserialize)]
struct File {
    name: String,
    size: u64,
    #[serde(default)]
    mime: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Outcome {
    Completed,
    Cancelled,
    Failed,
}

fn path() -> Option<PathBuf> {
    dirs::data_dir().map(|path| path.join("omasend/history.json"))
}

/// The stored records, oldest first, as the application's own transfers.
pub fn load() -> Vec<Transfer> {
    let Some(path) = path() else {
        return Vec::new();
    };
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => {
            tracing::warn!(%error, "Could not read the transfer history");
            return Vec::new();
        }
    };
    let records: Vec<Record> = match serde_json::from_slice(&data) {
        Ok(records) => records,
        Err(error) => {
            tracing::warn!(%error, "Could not parse the transfer history");
            return Vec::new();
        }
    };
    records.into_iter().map(Record::into_transfer).collect()
}

/// Replaces the stored history with the finished transfers of this session.
pub fn save(transfers: &[Transfer]) {
    let Some(path) = path() else {
        return;
    };
    let records: Vec<Record> = transfers
        .iter()
        .filter_map(Record::of)
        .rev()
        .take(LIMIT)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let result = (|| -> anyhow::Result<()> {
        let parent = path.parent().expect("history path has a parent");
        std::fs::create_dir_all(parent)?;
        // Replace in one step, so an interrupted write cannot leave a partial
        // file where the next start expects records.
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut file, &records)?;
        file.persist(&path)?;
        Ok(())
    })();
    if let Err(error) = result {
        tracing::warn!(%error, "Could not save the transfer history");
    }
}

impl Record {
    fn of(transfer: &Transfer) -> Option<Self> {
        let (outcome, error) = match &transfer.status {
            TransferStatus::Completed => (Outcome::Completed, None),
            TransferStatus::Cancelled => (Outcome::Cancelled, None),
            TransferStatus::Failed(error) => (Outcome::Failed, Some(error.clone())),
            TransferStatus::Active => return None,
        };
        Some(Self {
            id: transfer.id.clone(),
            peer: transfer.peer.clone(),
            sending: transfer.sending,
            files: transfer
                .files
                .iter()
                .map(|file| File {
                    name: file.name.clone(),
                    size: file.size,
                    mime: file.mime.clone(),
                })
                .collect(),
            total: transfer.total,
            paths: transfer.paths.clone(),
            outcome,
            error,
            when: transfer
                .when
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }

    fn into_transfer(self) -> Transfer {
        let status = match self.outcome {
            Outcome::Completed => TransferStatus::Completed,
            Outcome::Cancelled => TransferStatus::Cancelled,
            Outcome::Failed => TransferStatus::Failed(self.error.unwrap_or_default()),
        };
        Transfer::restored(
            self.id,
            self.peer,
            self.sending,
            self.files
                .into_iter()
                .map(|file| OfferedFile {
                    name: file.name,
                    size: file.size,
                    mime: file.mime,
                })
                .collect(),
            self.total,
            // A record must never offer to open a file that is no longer there.
            self.paths
                .into_iter()
                .filter(|path| path.exists())
                .collect(),
            status,
            UNIX_EPOCH + Duration::from_secs(self.when),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn transfer(id: &str, status: TransferStatus) -> Transfer {
        Transfer::restored(
            id.into(),
            "peer".into(),
            true,
            vec![OfferedFile {
                name: "a.txt".into(),
                size: 3,
                mime: "text/plain".into(),
            }],
            3,
            Vec::new(),
            status,
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        )
    }

    #[test]
    fn only_finished_transfers_round_trip() {
        let records: Vec<Record> = [
            transfer("done", TransferStatus::Completed),
            transfer("running", TransferStatus::Active),
            transfer("broken", TransferStatus::Failed("boom".into())),
        ]
        .iter()
        .filter_map(Record::of)
        .collect();
        assert_eq!(records.len(), 2);
        let restored: Vec<Transfer> = records.into_iter().map(Record::into_transfer).collect();
        assert_eq!(restored[0].id, "done");
        assert_eq!(restored[0].status, TransferStatus::Completed);
        assert_eq!(restored[0].files[0].name, "a.txt");
        assert_eq!(restored[1].status, TransferStatus::Failed("boom".into()));
        assert_eq!(
            restored[1]
                .when
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            1_700_000_000
        );
    }

    #[test]
    fn a_missing_received_file_loses_its_actions() {
        let mut record = Record::of(&transfer("done", TransferStatus::Completed)).unwrap();
        let present = tempfile::NamedTempFile::new().unwrap();
        record.paths = vec![present.path().to_path_buf(), "/nonexistent/gone.txt".into()];
        let restored = record.into_transfer();
        assert_eq!(restored.paths, vec![present.path().to_path_buf()]);
    }

    #[test]
    fn stored_json_keeps_the_newest_records_within_the_limit() {
        let transfers: Vec<Transfer> = (0..LIMIT + 20)
            .map(|index| transfer(&format!("t{index}"), TransferStatus::Completed))
            .collect();
        let records: Vec<Record> = transfers
            .iter()
            .filter_map(Record::of)
            .rev()
            .take(LIMIT)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        assert_eq!(records.len(), LIMIT);
        assert_eq!(records.first().unwrap().id, "t20");
        assert_eq!(records.last().unwrap().id, format!("t{}", LIMIT + 19));
    }
}
