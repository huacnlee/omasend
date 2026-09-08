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
    let asset_name = release_asset_name(&parsed, target)?;
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

fn release_asset_name(version: &semver::Version, target: &str) -> Result<String> {
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
    Ok(format!("omasend-{version}-{target}.{extension}"))
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

    #[test]
    fn release_assets_match_published_platform_names() {
        let version = semver::Version::new(9, 0, 0);
        for (target, expected) in [
            (
                "aarch64-apple-darwin",
                "omasend-9.0.0-aarch64-apple-darwin.tar.gz",
            ),
            (
                "x86_64-apple-darwin",
                "omasend-9.0.0-x86_64-apple-darwin.tar.gz",
            ),
            (
                "x86_64-unknown-linux-gnu",
                "omasend-9.0.0-x86_64-unknown-linux-gnu.tar.gz",
            ),
            (
                "aarch64-unknown-linux-gnu",
                "omasend-9.0.0-aarch64-unknown-linux-gnu.tar.gz",
            ),
            (
                "x86_64-pc-windows-msvc",
                "omasend-9.0.0-x86_64-pc-windows-msvc.zip",
            ),
        ] {
            assert_eq!(release_asset_name(&version, target).unwrap(), expected);
        }
        assert!(release_asset_name(&version, "aarch64-pc-windows-msvc").is_err());
        assert!(release_asset_name(&version, "x86_64-unknown-linux-musl").is_err());
    }

    #[test]
    fn accepts_newer_stable_versions_with_optional_tag_prefix() {
        assert!(builder("9.0.0").is_ok());
        assert!(builder("v9.0.0").is_ok());
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
