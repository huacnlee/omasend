# Install a release

Releases are published at <https://github.com/huacnlee/omasend/releases>.
The installers download the latest stable release and verify its `SHA256SUMS`
before installing. They run as the current user and do not require administrator
access. A release must have been published before these commands can succeed.
Published builds support Apple Silicon and Intel macOS, x86_64 Linux, and
x86_64 Windows. Other architectures require a source build.

## macOS and Linux

```sh
curl -fsSL https://github.com/huacnlee/omasend/raw/refs/heads/main/install.sh | sh
```

Use `--version 0.1.0` for a specific release. Use `--dir DIRECTORY` to select
the destination: an Applications directory on macOS, an installation prefix
on Linux. The defaults are `~/Applications` and `~/.local`, respectively.
`OMASEND_VERSION` and `OMASEND_INSTALL_DIR` provide the same defaults.
For example, append `sh -s -- --version 0.1.0` in place of `sh` in the command
above. The same POSIX shell installer detects macOS and Linux automatically.

macOS releases are `.tar.gz` archives containing `OmaSend.app`. Open the
installed app in Finder. The installer preserves the release's signing and
quarantine behavior; it does not disable Gatekeeper.

On Linux, put `~/.local/bin` on `PATH`. The installer adds the desktop entry
and icon under the prefix's `share` directory. A custom prefix also needs its
`share` directory on `XDG_DATA_DIRS` for launcher discovery. Install your
distribution's graphics/Wayland runtime libraries and `wl-clipboard` for paste.
The installer does not modify package-manager or shell configuration.

Run the installer again to upgrade. Close OmaSend before upgrading.

## Windows

Download and run in PowerShell:

```powershell
Invoke-WebRequest https://github.com/huacnlee/omasend/raw/refs/heads/main/install.ps1 -OutFile "$env:TEMP\omasend-install.ps1"
& "$env:TEMP\omasend-install.ps1"
```

Use `-Version 0.1.0` for a specific release or `-InstallDir PATH` for a custom
destination. The default is `%LOCALAPPDATA%\Programs\OmaSend`; the installer
creates a Start menu shortcut. Close the app before running the installer
again to upgrade. If local PowerShell policy prevents script execution,
download and extract the Windows ZIP from the release page instead.

## Manual installation and removal

Every archive is portable. Download the archive for your platform and CPU,
verify its SHA-256 against `SHA256SUMS`, and extract it. macOS users can move
`OmaSend.app` into Applications; Windows users can run `omasend.exe`; Linux
users can run `omasend` or install the bundled desktop entry and icon.

To uninstall, close the app and remove its installation files. On macOS,
remove `~/Applications/OmaSend.app`. On Windows, remove the install directory
and the OmaSend Start menu shortcut. On Linux, remove `bin/omasend`,
`share/applications/omasend.desktop`, and
`share/icons/hicolor/1024x1024/apps/omasend.png` under the chosen prefix.
Received files in Downloads are retained.
