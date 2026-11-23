//! Exercise the actual wl-paste process boundary without changing the user's clipboard.
#![cfg(unix)]
use omasend::{clipboard::wayland, model::SendItem};
use std::{os::unix::fs::PermissionsExt, process::Command};

#[test]
fn mime_enumeration_precedes_reads_and_media_is_cleaned_up() {
    if let Ok(kind) = std::env::var("OMASEND_CLIPBOARD_FIXTURE") {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(wayland::paste());
        let root = std::path::PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").unwrap());
        if kind == "empty" {
            assert!(result.is_err());
            assert_eq!(std::fs::read_dir(root.join("omasend")).unwrap().count(), 0);
            return;
        }
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            SendItem::TemporaryFile { path, mime } => {
                assert_eq!(
                    mime,
                    if kind == "video" {
                        "video/mp4"
                    } else {
                        "image/png"
                    }
                );
                assert_eq!(std::fs::read(path.as_ref()).unwrap(), b"binary\0\xff\r\n");
                assert!(path.starts_with(root.join("omasend")));
                let path = path.to_path_buf();
                drop(items);
                assert!(!path.exists());
            }
            SendItem::Text(text) => {
                assert_eq!(text, "first\nsecond\n");
                assert!(!root.join("omasend").exists());
            }
            SendItem::File(path) => {
                assert_eq!(path, &root.join("original.txt"));
                assert!(!root.join("omasend").exists());
            }
        }
        let calls = std::fs::read_to_string(root.join("calls")).unwrap();
        assert!(calls.starts_with("--list-types\n--no-newline --type "));
        assert_eq!(calls.lines().count(), 2);
        return;
    }
    for kind in ["image", "video", "text", "file", "empty"] {
        let root = tempfile::tempdir().unwrap();
        let types = match kind {
            "image" | "empty" => "text/plain\nimage/png\n",
            "video" => "text/plain\nvideo/mp4\n",
            "file" => "text/plain\ntext/uri-list\n",
            _ => "text/plain\n",
        };
        std::fs::write(root.path().join("types"), types).unwrap();
        let data = match kind {
            "text" => b"first\nsecond\n".to_vec(),
            "empty" => Vec::new(),
            "file" => format!(
                "{}\r\n",
                url::Url::from_file_path(root.path().join("original.txt")).unwrap()
            )
            .into_bytes(),
            _ => b"binary\0\xff\r\n".to_vec(),
        };
        std::fs::write(root.path().join("original.txt"), b"original").unwrap();
        std::fs::write(root.path().join("payload"), data).unwrap();
        let executable = root.path().join("wl-paste");
        std::fs::write(
            &executable,
            r#"#!/bin/sh
printf '%s\n' "$*" >> "$XDG_RUNTIME_DIR/calls"
if [ "$1" = "--list-types" ]; then
    /bin/cat "$XDG_RUNTIME_DIR/types"
else
    /bin/cat "$XDG_RUNTIME_DIR/payload"
fi
"#,
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "mime_enumeration_precedes_reads_and_media_is_cleaned_up",
                "--nocapture",
            ])
            .env("OMASEND_CLIPBOARD_FIXTURE", kind)
            .env("PATH", root.path())
            .env("XDG_RUNTIME_DIR", root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{kind}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
