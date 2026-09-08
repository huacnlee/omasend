# OmaSend

A native GPUI + gpui-omarchy LocalSend client for Omarchy and Wayland.

Send files, folders, text, clipboard images and recordings to nearby LocalSend
clients. Incoming transfers require acceptance and save to the XDG Downloads
folder, with numbered names when a file already exists. History lasts for the
current session. Closing the application stops its networking.

The interface supports English and Simplified Chinese. It initially follows
`LC_ALL`, `LC_MESSAGES`, then `LANG`; macOS falls back to its system locale.
Choose English or 简体中文 in the menu to switch the current session. Device
identity comes from the operating system, rather than the interface language.

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
- Exit is in the menu. Omarchy's compositor owns `super+w` window closing;
  OmaSend does not override it or bind `ctrl+q`.
- macOS development builds also accept `cmd+v`, `cmd+o`, and `cmd+q`.

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

`vendor/localsend` is the unmodified official Rust core, pinned to the commit
recorded in its `UPSTREAM.md`; WebRTC and web sharing are disabled.
`vendor/gpui-omarchy` pins version 0.1.0 with one presentation patch: menu
shortcuts render with its existing `keycap` component. See its `UPSTREAM.txt`.

The original OmaSend emblem is inspired by LocalSend's local-discovery shape
and Omarchy's pixel geometry; it is not either project's official mark. Linux
uses the square PNG; macOS uses its own rounded, transparent-margin ICNS.
The titlebar discovery effect crops the same emblem into animated segments.
GPUI's reduced-motion setting renders a static logo.
