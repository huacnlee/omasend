# Install a release

Releases are published at <https://github.com/huacnlee/omasend/releases>.
The installers download the latest stable release and verify its `SHA256SUMS`
before installing. They run as the current user and do not require administrator
access. A release must have been published before these commands can succeed.
Published builds support Apple Silicon and Intel macOS, x86_64 and ARM64 Linux, and
x86_64 Windows, Android 9+ on ARM64 and x86_64, and iOS 15+ on ARM64.
The terminal installers below are for desktop platforms.

## macOS and Linux

```sh
curl -fsSL https://huacnlee.github.io/omasend/install.sh | sh
```

Use `--version 0.1.0` for a specific release. Use `--dir DIRECTORY` to select
the destination: an Applications directory on macOS, an installation prefix
on Linux. The defaults are `~/Applications` and `~/.local`, respectively.
`OMASEND_VERSION` and `OMASEND_INSTALL_DIR` provide the same defaults.
For example, append `sh -s -- --version 0.1.0` in place of `sh` in the command
above. The same POSIX shell installer detects macOS and Linux automatically.

The native macOS application requires macOS 14 or later.
macOS releases are `.tar.gz` archives containing `OmaSend.app`. Open the
installed app in Finder. The installer preserves the release's signing and
quarantine behavior; it does not disable Gatekeeper.

On Linux, put `~/.local/bin` on `PATH`. The installer adds the desktop entry
and icon under the prefix's `share` directory. A custom prefix also needs its
`share` directory on `XDG_DATA_DIRS` for launcher discovery. Install your
distribution's GTK 4.14 or later and `xdg-desktop-portal` runtime packages, plus
`wl-clipboard` for Wayland paste.
The installer does not modify package-manager or shell configuration.

Run the installer again to upgrade. Close Omasend before upgrading.

## Windows

Download and run in PowerShell:

```powershell
Invoke-WebRequest https://huacnlee.github.io/omasend/install.ps1 -OutFile "$env:TEMP\omasend-install.ps1"
& "$env:TEMP\omasend-install.ps1"
```

Use `-Version 0.1.0` for a specific release or `-InstallDir PATH` for a custom
destination. The default is `%LOCALAPPDATA%\Programs\OmaSend`; the installer
creates a Start menu shortcut. Close the app before running the installer
again to upgrade. If local PowerShell policy prevents script execution,
download and extract the Windows ZIP from the release page instead.

## Android

1. Open the [latest release](https://github.com/huacnlee/omasend/releases/latest)
   on your phone and download `omasend-VERSION-android.apk` from **Assets**.
   One APK supports ARM64 phones and x86_64 devices; 32-bit Android is not included.
2. Open the downloaded APK. If Android blocks installation, open the prompted
   **Settings**, select **Install unknown apps**, and enable **Allow from this source**
   for the browser or file manager opening the APK. Labels vary by manufacturer;
   see [Google's installation instructions](https://support.google.com/pixelphone/answer/7391672?hl=en).
3. Return to the APK and tap **Install**, then open Omasend.

To update, download the newer APK and install it over the existing app. Releases
use the same signing key, so there is no need to uninstall first. Received files
are stored privately by Omasend; use **Transfer history → Open** to preview or
export them through Android's share sheet. Uninstalling removes app-private data,
so export any received files you want to keep before removing the app.

## iOS / iPhone

The release provides `omasend-VERSION-aarch64-apple-ios-unsigned.ipa` for iOS 15+.
It is an **unsigned IPA** that must be signed with your Apple account before
installation. Downloading or opening the IPA in Files alone does not install it.

### Install with Sideloadly (macOS or Windows)

1. Download the IPA from the [latest release](https://github.com/huacnlee/omasend/releases/latest)
   and install [Sideloadly from its official website](https://sideloadly.io/).
   Follow its prerequisites for your computer.
2. Connect your iPhone by USB, unlock it, and accept **Trust This Computer** if asked.
3. In Sideloadly, select your iPhone, add the downloaded IPA, enter your Apple
   account, and start installation. Follow its authentication prompts to sign
   the app for your device.
4. On iOS 16+, enable **Settings → Privacy & Security → Developer Mode** when
   requested, restart the phone, and confirm the setting. If iOS asks you to
   trust the developer, open **Settings → General → VPN & Device Management**,
   select your developer account, and trust it.
5. Open Omasend and allow **Local Network** access when prompted.

A free Apple developer account's signature lasts seven days. Refresh it before
expiry with the signing tool; Sideloadly supports automatic refreshing while your
computer can reach the phone. See its [official FAQ](https://sideloadly.io/faq)
for current setup requirements and signing limits. To update, import the newer
IPA using the same Apple account and bundle ID.

### Install with AltStore Classic

If you use [AltStore Classic](https://faq.altstore.io/altstore-classic), complete
its official setup first, then import the downloaded IPA from **My Apps → +**.
Keep the app's signature refreshed; free accounts require a refresh every seven
days. See [AltStore's getting-started guide](https://faq.altstore.io/altstore-classic/your-altstore).

Received files are available in **Files → On My iPhone → Omasend → Received**,
and can also be opened or shared from transfer history. For a developer-signed
build installed directly through Xcode's device tools, see the
[development instructions](../README.md#development-and-build).

## First transfer on a phone

Connect both devices to the same local network, keep Omasend open on both, and
allow Local Network access on iOS. Choose **Add files… → Photos and videos** or
**Browse files**, select a nearby device, and tap **Send**. Accept the transfer
on the receiving device. Install mobile updates using the APK or IPA workflow
above; the in-app updater is for desktop builds.

## mise

[mise](https://mise.jdx.dev) can install the release binary from GitHub:

```sh
mise use -g github:huacnlee/omasend
```

On macOS, point mise at the binary inside the app bundle:

```sh
mise use -g 'github:huacnlee/omasend[bin_path=OmaSend.app/Contents/MacOS]'
```

mise installs only the executable. It does not add the Linux desktop entry and
icon, or put `OmaSend.app` in Applications. Upgrade with `mise upgrade`, not
the in-app updater, so mise keeps track of the installed version.

## Manual installation and removal

Every archive is portable. Download the archive for your platform and CPU,
verify its SHA-256 against `SHA256SUMS`, and extract it. macOS users can move
`OmaSend.app` into Applications; Windows users can run `omasend.exe`; Linux
users can run `omasend` or install the bundled desktop entry and icon.

To uninstall, close the app and remove its installation files. On macOS,
remove `~/Applications/OmaSend.app`. On Windows, remove the install directory
and the Omasend Start menu shortcut. On Linux, remove `bin/omasend`,
`share/applications/omasend.desktop`, and
`share/icons/hicolor/1024x1024/apps/omasend.png` under the chosen prefix.
Received files in Downloads are retained.
