//! GitHub asset selection and platform policy; self_update owns verification and replacement.
use anyhow::{Context, Result, ensure};
use self_update::backends::github::{Update, UpdateBuilder};
use std::{path::Path, time::Duration};

pub fn builder(version: &str) -> Result<UpdateBuilder> {
    let parsed = semver::Version::parse(version.strip_prefix('v').unwrap_or(version))?;
    ensure!(parsed.pre.is_empty(), "Only stable updates are supported");
    ensure!(
        parsed > semver::Version::parse(env!("CARGO_PKG_VERSION"))?,
        "Update must be newer than this application"
    );
    let target = self_update::get_target();
    ensure!(
        matches!(
            target,
            "aarch64-apple-darwin"
                | "x86_64-apple-darwin"
                | "x86_64-unknown-linux-gnu"
                | "aarch64-unknown-linux-gnu"
                | "x86_64-pc-windows-msvc"
        ),
        "No release package for {target}"
    );
    let extension = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let asset_name = format!("omasend-{parsed}-{target}.{extension}");
    let mut builder = Update::configure();
    builder
        .repo_owner("huacnlee")
        .repo_name("omasend")
        .bin_name("omasend")
        .current_version(env!("CARGO_PKG_VERSION"))
        .release_tag(format!("v{parsed}"))
        .target(target)
        .asset_matcher(move |assets| {
            assets
                .iter()
                .find(|asset| asset.name() == asset_name)
                .cloned()
        })
        .checksum_from_asset("SHA256SUMS")
        .check_install_path_writable(true)
        .no_confirm(true)
        .show_output(false)
        .show_download_progress(false)
        .timeout(Duration::from_secs(15 * 60));
    Ok(builder)
}

pub fn install(
    version: &str,
    progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
) -> Result<()> {
    let mut builder = builder(version)?;
    builder.progress_callback(progress);
    #[cfg(target_os = "macos")]
    {
        // Replacing just Contents/MacOS/omasend would invalidate the signed bundle.
        let executable = std::env::current_exe()?;
        let app = executable
            .ancestors()
            .find(|path| path.extension().is_some_and(|ext| ext == "app"))
            .context("Install OmaSend.app in Applications before updating")?;
        builder
            .bundle_path_in_archive("OmaSend.app")
            .bundle_install_path(app);
        builder.verify_binary(verify_bundle);
    }
    let status = builder
        .build()?
        .update()
        .context("Could not install update")?;
    ensure!(
        status.is_updated(),
        "The selected release was not installed"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_bundle(path: &Path) -> self_update::Result<()> {
    if !path.join("Contents/MacOS/omasend").is_file() || !path.join("Contents/Info.plist").is_file()
    {
        return Err(self_update::Error::verification_rejected(
            "Release is missing OmaSend.app contents",
        ));
    }
    let result = std::process::Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(path)
        .output()?;
    if !result.status.success() {
        return Err(self_update::Error::verification_rejected(
            "The downloaded app's code signature is invalid",
        ));
    }
    Ok(())
}

// Keep the original path: Linux current_exe() can point to the old, renamed inode
// after self-replacement. Relaunch only after GPUI and the network runtime stop.
pub fn restart(executable: &Path) -> Result<()> {
    let mut command = std::process::Command::new(executable);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(command.exec()).context("Could not restart Omasend")
    }
    #[cfg(windows)]
    {
        command.spawn().context("Could not restart Omasend")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};

    fn archive() -> Vec<u8> {
        if cfg!(windows) {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            zip.start_file("omasend.exe", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"new binary").unwrap();
            zip.finish().unwrap().into_inner()
        } else {
            let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            let mut tar = tar::Builder::new(gzip);
            for (path, data) in [
                ("omasend", b"new binary".as_slice()),
                (
                    "OmaSend.app/Contents/MacOS/omasend",
                    b"new binary".as_slice(),
                ),
                (
                    "OmaSend.app/Contents/Resources/icon",
                    b"new icon".as_slice(),
                ),
            ] {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                tar.append_data(&mut header, path, data).unwrap();
            }
            tar.into_inner().unwrap().finish().unwrap()
        }
    }

    // Exercise the real backend against a local GitHub-shaped API and disposable
    // installation. No test can replace the test runner or the user's application.
    fn update_fixture(bad_checksum: bool, reject_binary: bool, bundle: bool) {
        let archive = archive();
        let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
        let asset = format!("omasend-9.0.0-{}.{extension}", self_update::get_target());
        let digest = if bad_checksum {
            "0".repeat(64)
        } else {
            format!("{:x}", Sha256::digest(&archive))
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let release = serde_json::json!({
            "tag_name": "v9.0.0", "created_at": "2026-09-08T00:00:00Z",
            "assets": [
                {"name": asset, "url": format!("{base}/archive")},
                {"name": "SHA256SUMS", "url": format!("{base}/sums")}
            ]
        })
        .to_string();
        let sums = format!("{digest}  {asset}\n");
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        let server = std::thread::spawn(move || {
            while matches!(
                stop_rx.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Empty)
            ) {
                let (mut socket, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        // Wake promptly on shutdown; this is idle polling, not a
                        // startup delay that assumes the client is ready.
                        let _ = stop_rx.recv_timeout(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => panic!("Fixture failed to accept a connection: {error}"),
                };
                // Windows inherits the listener's nonblocking mode. Request reads
                // must wait for incoming bytes, bounded by the timeout below.
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 2048];
                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                    let count = match socket.read(&mut chunk) {
                        Ok(count) => count,
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(error) => panic!("Fixture failed to read request headers: {error}"),
                    };
                    assert!(count > 0, "Client closed before sending complete headers");
                    assert!(request.len() + count <= 16384, "Request headers too large");
                    request.extend_from_slice(&chunk[..count]);
                }
                let path = std::str::from_utf8(&request)
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                let body = match path {
                    "/repos/huacnlee/omasend/releases/tags/v9.0.0" => release.as_bytes(),
                    "/sums" => sums.as_bytes(),
                    "/archive" => &archive,
                    _ => panic!("Unexpected request: {path}"),
                };
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                socket.write_all(body).unwrap();
            }
        });
        let temp = tempfile::tempdir().unwrap();
        let destination = temp
            .path()
            .join(if bundle { "OmaSend.app" } else { "omasend" });
        let installed_binary = if bundle {
            destination.join("Contents/MacOS/omasend")
        } else {
            destination.clone()
        };
        std::fs::create_dir_all(installed_binary.parent().unwrap()).unwrap();
        std::fs::write(&installed_binary, "old binary").unwrap();
        if bundle {
            std::fs::create_dir_all(destination.join("Contents/Resources")).unwrap();
            std::fs::write(
                destination.join("Contents/Resources/obsolete"),
                "old resource",
            )
            .unwrap();
        }
        let mut config = builder("v9.0.0").unwrap();
        config.api_base_url(base).timeout(Duration::from_secs(5));
        if bundle {
            config
                .bundle_path_in_archive("OmaSend.app")
                .bundle_install_path(&destination);
        } else {
            config.bin_install_path(&destination);
        }
        if reject_binary {
            config.verify_binary(|_| {
                Err(self_update::Error::verification_rejected(
                    "invalid executable",
                ))
            });
        }
        let result = config.build().unwrap().update();
        drop(stop_tx);
        server.join().unwrap();
        if bad_checksum || reject_binary {
            let error = result.unwrap_err();
            if bad_checksum {
                assert!(
                    matches!(error, self_update::Error::ChecksumMismatch { .. }),
                    "Expected checksum rejection, got {error:?}"
                );
            } else {
                assert!(
                    matches!(error, self_update::Error::VerificationRejected { .. }),
                    "Expected executable rejection, got {error:?}"
                );
            }
            assert_eq!(
                std::fs::read_to_string(installed_binary).unwrap(),
                "old binary"
            );
        } else {
            assert!(result.unwrap().is_updated());
            assert_eq!(
                std::fs::read_to_string(installed_binary).unwrap(),
                "new binary"
            );
            if bundle {
                assert_eq!(
                    std::fs::read_to_string(destination.join("Contents/Resources/icon")).unwrap(),
                    "new icon"
                );
                assert!(!destination.join("Contents/Resources/obsolete").exists());
            }
        }
    }

    #[test]
    fn verified_release_replaces_only_the_fixture() {
        update_fixture(false, false, false);
    }
    #[test]
    fn checksum_mismatch_keeps_installed_version() {
        update_fixture(true, false, false);
    }
    #[test]
    fn rejected_executable_keeps_installed_version() {
        update_fixture(false, true, false);
    }
    #[cfg(unix)]
    #[test]
    fn bundle_update_replaces_resources_together() {
        update_fixture(false, false, true);
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn verifies_bundle_signature_and_rejects_modified_resources() {
        let temp = tempfile::tempdir().unwrap();
        let app = temp.path().join("OmaSend.app");
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        std::fs::copy("/usr/bin/true", app.join("Contents/MacOS/omasend")).unwrap();
        std::fs::write(app.join("Contents/Resources/icon"), "original").unwrap();
        std::fs::write(app.join("Contents/Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.omasend.update-test</string><key>CFBundleExecutable</key><string>omasend</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#).unwrap();
        let signed = std::process::Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(&app)
            .output()
            .unwrap();
        assert!(
            signed.status.success(),
            "{}",
            String::from_utf8_lossy(&signed.stderr)
        );
        verify_bundle(&app).unwrap();
        std::fs::write(app.join("Contents/Resources/icon"), "modified").unwrap();
        assert!(verify_bundle(&app).is_err());
    }

    #[test]
    fn rejects_downgrades_and_prereleases() {
        assert!(builder(env!("CARGO_PKG_VERSION")).is_err());
        assert!(builder("0.0.0").is_err());
        assert!(builder("9.0.0-beta.1").is_err());
        assert!(builder("../../bad").is_err());
    }
}
