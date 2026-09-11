//! Bounded, session-only diagnostics for the in-app log viewer.
use std::{
    collections::VecDeque,
    io::Write,
    sync::{Mutex, OnceLock},
};

const MAX_ENTRIES: usize = 500;
static LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

fn logs() -> &'static Mutex<VecDeque<String>> {
    LOGS.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub fn snapshot() -> String {
    logs()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn clear() {
    logs()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
}

fn append(message: &str) {
    let mut entries = logs().lock().unwrap_or_else(|error| error.into_inner());
    if entries.len() == MAX_ENTRIES {
        entries.pop_front();
    }
    entries.push_back(message.trim_end().chars().take(16_384).collect());
}

struct LogWriter;
struct EventWriter(Vec<u8>);

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = EventWriter;
    fn make_writer(&'a self) -> Self::Writer {
        EventWriter(Vec::new())
    }
}

impl Write for EventWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for EventWriter {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            let _ = std::io::stderr().write_all(&self.0);
            append(&String::from_utf8_lossy(&self.0));
        }
    }
}

fn default_filter(debug: bool) -> &'static str {
    if debug {
        "warn,omasend=debug,localsend::discovery=debug"
    } else {
        "warn,omasend=info"
    }
}

pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter(cfg!(debug_assertions)).into()),
        )
        .with_writer(LogWriter)
        .with_ansi(false)
        .init();
    tracing::info!("Omasend session started");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logs_keep_complete_events_and_bound_session_memory() {
        clear();
        {
            let mut writer = EventWriter(Vec::new());
            writer.write_all(b"first ").unwrap();
            writer.write_all(b"event\n").unwrap();
        }
        assert_eq!(snapshot(), "first event");
        for index in 0..MAX_ENTRIES {
            append(&format!("event {index}"));
        }
        let text = snapshot();
        assert_eq!(text.lines().count(), MAX_ENTRIES);
        assert!(text.starts_with("event 0\n"));
        assert!(text.ends_with("event 499"));
        clear();
        assert!(snapshot().is_empty());
    }

    #[test]
    fn release_default_suppresses_debug_events() {
        #[derive(Clone)]
        struct Capture(std::sync::Arc<Mutex<Vec<u8>>>);
        struct CaptureWriter(std::sync::Arc<Mutex<Vec<u8>>>);
        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
            type Writer = CaptureWriter;
            fn make_writer(&'a self) -> Self::Writer {
                CaptureWriter(self.0.clone())
            }
        }
        impl Write for CaptureWriter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let output = std::sync::Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(default_filter(false)))
            .with_writer(Capture(output.clone()))
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("discovery probe");
            tracing::warn!("connection failed");
        });
        let text = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(!text.contains("discovery probe"));
        assert!(text.contains("connection failed"));
    }
}
