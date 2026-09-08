# Omasend

Copy, paste, and send to your other devices.

Omasend is a desktop file-sharing app built for [Omarchy](https://omarchy.org),
and a native client for the [LocalSend](https://localsend.org) protocol. Send files,
folders, screenshots, recordings, and text over your local network — to Omasend or
to any LocalSend app — with no account or cloud upload. Available for Linux, macOS,
and Windows.

<img width="2546" height="1402" alt="image" src="https://github.com/user-attachments/assets/d3d942e9-c8b1-4c45-aaf4-1b1508384adf" />

[Website](https://huacnlee.github.io/omasend/) · [Download](https://github.com/huacnlee/omasend/releases) · [Installation guide](docs/install.md)

## Features

- **Share with nearby devices.** Connect to Omasend and [LocalSend](https://localsend.org) clients on the same network.
- **Paste or drop anything.** Add clipboard content, drag in files and folders, or use the file picker. Preview before sending.
- **Keep transfers local.** Content travels directly between devices over an encrypted connection. You choose which incoming transfers to accept.
- **Follow your transfers.** See progress and speed, cancel a transfer, and review the current session’s history. Received files go to Downloads without overwriting existing files.
- **Make it feel at home.** Follow your Omarchy theme, choose System, Light, or Dark appearance, and switch between English and 简体中文.
- **Stay up to date.** Check for updates from the menu, install them in the app, and restart when your transfers are finished.

## Works with LocalSend

Omasend is a native client for the [LocalSend protocol](https://github.com/localsend/protocol),
so [LocalSend](https://localsend.org) on Android, iOS, macOS, Windows, or Linux appears as a nearby
device in Omasend, and Omasend appears in it. There is nothing to pair and no bridge in between.

- **Protocol.** Version 2.2, spoken through the
  [LocalSend Rust core](https://github.com/localsend/localsend) that also powers LocalSend itself.
- **Discovery.** UDP multicast on `224.0.0.167:53317`, and on the IPv6 group `ff12::fd3a:e420`.
  Omasend also probes the local IPv4 subnet, so a device that misses multicast still shows up.
- **Transfers.** HTTPS over TCP `53317`, with a certificate generated on first run and pinned by
  fingerprint. Omasend only talks to peers that announce HTTPS, and never falls back to plaintext.
- **Accepting.** Every incoming transfer waits for your confirmation. Omasend asks for no PIN, and
  cannot yet send to a device that requires one.

Keep both devices on the same network, and allow TCP and UDP port `53317` through the firewall.

## Install

Supports Linux x86_64 and ARM64, macOS Apple Silicon and Intel, and Windows x86_64.

**macOS / Linux**

```sh
curl -fsSL https://github.com/huacnlee/omasend/raw/refs/heads/main/install.sh | sh
```

**Windows PowerShell**

```powershell
irm https://github.com/huacnlee/omasend/raw/refs/heads/main/install.ps1 | iex
```

The installer selects the latest release for your device. For manual installation,
custom locations, or removal, see the [installation guide](docs/install.md).

## Send something

1. Open Omasend or [LocalSend](https://localsend.org) on your other device and connect both devices to the same network.
2. Paste content into Omasend, drag in files, or choose **Add files…**.
3. Select a nearby device, click **Send**, and accept the transfer on the receiving device.

Use `ctrl+v` to paste, `ctrl+o` to add files, and `enter` to send.
On macOS, `cmd+v` and `cmd+o` also work. Closing the macOS window keeps
Omasend available from the Dock; choose **Exit** to quit.

## Build and run

Built with [GPUI Kit](https://gpui-kit.com/),
[GPUI Omarchy](https://huacnlee.github.io/gpui-omarchy/), and the
[LocalSend Rust core](https://github.com/localsend/localsend).

With a recent stable Rust toolchain and your platform’s development libraries:

```sh
cargo run --release --locked
```

Linux builds require a C/C++ compiler, CMake, pkg-config, and development libraries
for OpenSSL, Fontconfig, FreeType, xkbcommon, X11/XCB, Wayland, Vulkan, and Zstandard.
Install `wl-clipboard` for clipboard support on Wayland.

## License

MIT licensed. An independent community project.
