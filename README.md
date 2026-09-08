# OmaSend

A native GPUI + gpui-omarchy LocalSend client for Omarchy and Wayland.

Send files, folders, text, clipboard images and recordings to nearby LocalSend
clients. Incoming transfers require acceptance and save to the XDG Downloads
folder, with numbered names when a file already exists. History lasts for the
current session. Exit stops networking. On macOS, closing the window keeps the
session running; clicking the Dock icon restores it.

The interface supports English and Simplified Chinese. It initially follows
`LC_ALL`, `LC_MESSAGES`, then `LANG`; macOS falls back to its system locale.
Choose English or 简体中文 in the menu to switch the current session. Device
identity comes from the operating system, rather than the interface language.

## Install and releases

See [installation instructions](docs/install.md) for macOS, Linux and Windows
installers. Releases provide macOS Apple Silicon/Intel `.tar.gz` bundles,
Linux x86_64 `.tar.gz`, Windows x86_64 `.zip`, and SHA-256 checksums.
The installers require a published GitHub release.

macOS / Linux:

```sh
curl -fsSL https://github.com/huacnlee/omasend/raw/refs/heads/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://github.com/huacnlee/omasend/raw/refs/heads/main/install.ps1 | iex
```

`.github/workflows/release.yml` builds and packages all platforms for pull
requests; a tag matching `v<Cargo.toml version>` publishes the release after
all builds pass. Modern macOS icon compilation requires Xcode 26 or newer.

The [website](website/README.md) uses Astro + Bun and GitHub Pages, following
gpui-omarchy's setup. Its PR workflow builds and runs browser checks; deployment
runs from `main` after Pages is configured to use GitHub Actions.

## Build and run

Use a recent stable Rust toolchain with edition 2024 support. Development checks
use Rust 1.98 on macOS and Rust 1.95 in the Linux verification container.

Linux needs a C/C++ compiler, CMake, pkg-config, Fontconfig/FreeType, xkbcommon
(including X11 support), Wayland and Vulkan development libraries. Runtime
requires a working graphics driver, `wl-clipboard` for paste, and a desktop
portal/file manager for native file selection and opening received files.

```sh
cargo build --release --locked
./target/release/omasend
```

Only one LocalSend server can normally use TCP port 53317 on the same machine.
Discovery uses UDP 53317 multicast and periodically probes local private IPv4
subnets through the official core. Small LANs through /22 are covered fully;
for larger networks the fallback is bounded to the interface's own /24.
HTTPS and certificate fingerprint verification remain enabled.

Install for the current Linux user after building:

```sh
install -Dm755 target/release/omasend "$HOME/.local/bin/omasend"
install -Dm644 packaging/omasend.desktop "$HOME/.local/share/applications/omasend.desktop"
install -Dm644 assets/omasend.png "$HOME/.local/share/icons/hicolor/1024x1024/apps/omasend.png"
```

Ensure `~/.local/bin` is on PATH. On Omarchy, use
`omarchy-launch-or-focus omasend` to launch or focus the window.

## Controls

- `ctrl+v`: paste files, images, video or text.
- `ctrl+o`: choose files or folders.
- Arrow keys: choose a nearby device.
- `enter`: send / confirm; `esc`: cancel / go back.
- `tab` / `shift+tab`: move keyboard focus.
- Logs… in the menu opens the latest 500 session log events, with copy and
  clear actions. Ordinary background discovery timeouts stay in the logs.
- Exit is in the menu. Omarchy's compositor owns `super+w` window closing;
  OmaSend does not override it or bind `ctrl+q`.
- macOS also accepts `cmd+v`, `cmd+o`, and `cmd+q`. `cmd+w` closes the
  window while keeping the session available from the Dock.

Folders expand to relative file paths; empty directories are not transferred.
Clipboard file URIs use the original file. Clipboard media is streamed to a
private temporary file under `$XDG_RUNTIME_DIR/omasend` and removed after its
last composer/transfer owner releases it. Text and file references are not
materialized into media copies.

## Verification

```sh
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --no-default-features --locked
cargo test --features ui-tests --bin omasend --locked
```

The default feature is the desktop app. Disabling it exposes no separate CLI;
it allows filesystem, clipboard-process, state and real HTTPS integration
tests to run without a display. The ignored `native_window` fixture requires
an open application and manual interaction; it does not run in the default
suite.

All-target desktop compilation and 22 headless tests pass on macOS and in a
Linux Debian container. macOS native UI and bidirectional transfers with a
LocalSend desktop peer have been exercised. Real iPhone discovery/interoperability and a real Linux
Wayland desktop session remain unverified. Receiver PIN entry is not yet
exposed in the interface. Language selection and history are session-only.

## Dependencies and assets

`vendor/localsend` pins the official Rust core to the commit recorded in its
`UPSTREAM.md`, with a local discovery-diagnostic event patch; TLS verification
is unchanged. WebRTC and web sharing are disabled.
`vendor/gpui-omarchy` pins version 0.1.0 with menu shortcuts rendered by its
existing `keycap` component and keyboard traversal respecting modal focus traps.
See its `UPSTREAM.txt`.

The original OmaSend emblem is inspired by LocalSend's local-discovery shape
and Omarchy's pixel geometry; it is not either project's official mark. Linux
uses the square PNG; macOS combines Icon Composer assets for modern systems
with a rounded, transparent-margin ICNS for older releases.
The header logo is static; the empty send area animates its outer pixel segments.
Both render a transparent vector emblem in the current theme's accent color.
GPUI's reduced-motion setting renders a static logo.

The menu includes About… with the current version and Check for update.
OmaSend checks GitHub's latest stable release at startup and every six hours.
Click Install in the status bar to download and install that version with
[self_update](https://github.com/jaemk/self_update). The updater requires the
release's SHA256SUMS checksum before replacing files. macOS updates the entire
signed OmaSend.app bundle; Linux and Windows replace the executable in place.
The installation directory must be writable by the current user.

Download progress, installation status, retry, and Restart to update appear in
the status bar. Restart is available after transfers finish and Outbox is empty;
network services shut down before the new process starts. Update failures leave
details in Logs. On macOS, run the installed .app to update, rather than a bare
development binary. Routine check results disappear after five seconds.
