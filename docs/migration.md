# Omasend on Native Shell

The main `omasend` executable now mounts the Rust `App` component directly on
AppKit, GTK 4 or Win32. The root manifest, lockfile and application source have no
GPUI, QuickJS or JavaScript-runtime dependencies. `nsh.json` also declares the
Rust root for the NSH toolchain. The separate migration crate has been removed.
The backend constraint is explicit: AppKit on macOS, GTK on Linux, and Win32 on
Windows. The GPUI backend is prohibited. `scripts/check-native-runtime.py` checks
the resolved dependency graph in CI, including all application features.
Product parity and platform verification remain in progress.

```sh
cargo run --locked
cargo test --locked --features ui-tests --lib
cargo test --locked --no-default-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

macOS requires version 14 or later (the native AppKit display-link runtime).
The release deployment target, bundle minimum and archive checker all require
14.0. Linux requires GTK 4.14 or later, pkg-config, OpenSSL and XCB development files.
File dialogs use xdg-desktop-portal. Wayland clipboard input keeps the existing
MIME-aware `wl-paste` adapter. macOS reads advertised binary pasteboard formats
before the clipboard library's TIFF fallback, so PNG-only sources work and raw
image/video bytes are preserved.

## Implemented

- Discovery, stable device selection, files/folders, clipboard, messages, sending,
  acceptance/refusal, progress, cancellation and completion use the original
  LocalSend adapter and model. File preparation runs off the engine thread.
- Device, composer and transfer rows keep stable nodes during network updates.
  Nearby devices collapse to one responsive row while retaining the selected
  device; expansion uses a bounded scroll area and a 160ms native layout
  transition (immediate under reduced motion). A 12-device engine fixture checks
  expansion, selected-node retention and resizing from 840×630 to 640×520.
  Left/Right shortcuts cycle device selection outside text editing. The engine
  fixture dispatches the actual mounted keymap bindings, checks wraparound, and
  confirms that opening the application menu blocks device navigation. Selection
  and removal share the input/overlay guards. Native pointer/scroll verification
  of the populated list remains pending.
- English/Chinese catalogs drive labels, statuses and errors. Language selection
  persists under the application's configuration directory.
- The existing bounded transfer history is loaded and saved. Clearing history
  retains active work. Received files have individual Open/Show in Files actions;
  received text can be read and copied.
- Composer actions follow transfer state: active items show Cancel and the
  command layer rejects their removal. Ordinary files and video omit unavailable
  preview actions; images and text retain previews. The real HTTPS engine test
  checks these controls while the receiver is awaiting confirmation. Each
  composer card now includes its own transferred bytes, percentage and speed,
  calculated from its range in the sequential batch, with a 160ms native
  progress transition. Waiting requests omit the progress bar. An engine fixture
  verifies batch offsets, zero-byte files and failed-transfer action recovery. Native
  macOS video-card appearance was also checked at 640×520.
- Images have thumbnails and full previews; text has a read-only preview. Image
  previews retain temporary-file ownership until closed. Incoming requests use a
  native modal with sender, file names, sizes, destination and explicit choices.
- Button press, list changes, modal appearance, image loading and progress
  have short native transitions. Reduced motion selects immediate states.
  Discovery recurrence is only scheduled while discovery and motion are active.
  An engine fixture toggles reduced motion during a live burst, drains queued
  stroke callbacks, exercises button press/release, and verifies that no new
  nonzero-duration animation starts until motion is re-enabled.
- CI and release workflows build the root application. Linux runners and the Arch
  recipe use GTK 4 dependencies. README, install instructions and site credits
  describe the new runtime. Historical GPUI documents are labeled as such.

- Theme selection is environment-specific. Only Linux with `ID=omarchy` consumes
  Omarchy palette/font/spacing files and follows the shell; saved overrides are
  ignored there. Other systems use an independent neutral/green theme inspired
  by the sibling `disktree` project (`huacnlee/nsh-disktree`), with distinct light
  and dark palettes, panel surfaces and gently raised 4px-radius buttons. Stale
  Omarchy files do not affect this standalone style. Theme preferences still
  persist and System/Light/Dark remain available outside Omarchy.
- Omarchy semantic/ANSI palettes, font and spacing overrides, persisted appearance,
  explicit reload, light/dark controls, logs and About now use Rust components.
  Update checks run at startup and every six hours; installation/restart is
  available in the interface, including progress, retry and guarded restart. Real
  package replacement/relaunch still needs platform verification. Root-owned
  tasks survive closing the surface that started them.
- Visual parity is a migration requirement. The original `HEAD` view sources and
  `website/public/screenshot.png` are the references; the initial replacement
  form layout was rejected. The home window now restores the original header,
  Nearby section, growing Outbox, bottom controls and status bar. Completed
  history opens separately rather than occupying the home window. macOS keeps
  the original Menlo fallback and both Ctrl and Command file/paste shortcuts.

The custom TitleBar draws the existing logo, title and menu inside the client
area. NSH's native backends retain system window controls and handle dragging and
platform double-click behavior. Reactive left/right insets keep application
content clear of those controls, including fullscreen changes. Buttons explicitly
opt out of dragging. The Rust desktop API is contributed upstream in [NSH PR #92](https://github.com/longbridge/nsh/pull/92).
PR #92 has merged. Omasend depends on the upstream git `main` branch;
`Cargo.lock` records the resolved commit. It must not vendor or locally patch NSH.
No JavaScript runtime is enabled.

## Current verification

On macOS, root all-feature compilation, Clippy and formatting pass. The 45 library
and UI tests include a real NSH engine activating Send, transferring binary data
and text through HTTPS, and receiving a reply through the actual Accept control.
The test verifies bytes in both directions, language changes, text preview
opening/closing, stable draft/send nodes and received-file actions. It also disposes the real NSH root with an incoming
HTTPS request still awaiting confirmation, verifies the peer stops waiting,
confirms that no unaccepted file is written, and binds a new TCP listener to the
released port. This covers engine disposal with a pending request, not the full
macOS retained-window lifecycle or an in-progress streaming transfer.
A separate engine
test mounts the production window composition at 840×630 and 640×520 and checks
that the header, Nearby section, main Outbox, bottom actions and status bar keep
the original order and bounds, with no permanent message editor. It also checks
the hidden native title-bar command, draggable header/non-draggable menu props,
and reactive reservations for left-side, right-side and fullscreen controls.

The headless suite passes 25 library tests plus 6 input, 7 state, 8 HTTPS transfer
and 1 mocked Wayland test. The manually operated native-window fixture remains
ignored. The Arch recipe test and all 9 Unix installer tests pass.

The optimized ARM macOS executable was packaged with the production release
script, extracted and executed by `packaging/check-release.py`. The real Unix
installer then installed and replaced that bundle in an isolated temporary
folder (including spaces in its path), preserving a valid signature and version
output. The installed release executable also opened the native window and
produced a visually inspected screenshot. This is installer upgrade evidence,
not an exercise of the in-app updater. The release workflow now checks the
native dependency graph and each generated archive's checksum/resources/signature
(where applicable), then executes its packaged `--version` on native targets.
Intel macOS is cross-compiled and receives archive checks only. An intentionally
incorrect checksum was rejected locally. Linux/Windows packaged execution and
Intel macOS runtime verification remain pending.

Native macOS scripts exercised the root executable, PNG-only system clipboard
input, thumbnail decoding, opening the actual image preview and closing it with
Escape. The manual `scripts/check-macos-clipboard.swift` fixture now checks JPEG and
TIFF against the real pasteboard, compares imported temporary-file bytes with
the original payload, opens the native image preview, and verifies clipboard
restoration after both successful and intentionally failed runs. Run it after
`cargo build --locked` with `swift scripts/check-macos-clipboard.swift jpeg`
or `tiff` in a logged-in macOS session. An earlier ad hoc fixture exited before
restoring its clipboard snapshot; its restoration claim was not valid.
Earlier native checks
covered message entry at 840×630 and 640×520 in both appearances. These checks do
not prove keyboard/modal coverage or visual parity with the original application.
Native scripts verified the compact history bottom sheet at 840×630 and
640×520, including text expansion and Escape dismissal. Finished records omit
progress bars; Collapse/Copy precede the adaptive-height text reader so those
actions remain reachable in the smaller window. Clipboard copy itself still
needs an OS-level check.
Native scripts also exercised the restored title-bar menu, left-opening theme
and language submenus, Escape dismissal, and the 640×520 layout. Menu groups now
have scoped Up/Down navigation that skips disabled items, Enter/Space activation,
and Left to return from a submenu. The current row is highlighted and receives
an accessibility-focus command. The engine checks keyboard theme selection,
submenu return, automatic dismissal and unchanged device selection; native
macOS screenshots verify highlighted main/submenu rows and Left return. This
is not a complete screen-reader or Tab/focus-restoration verification. The file-picker
path and remaining non-PNG pasteboard formats (such as GIF/WebP) still need
native checks; JPEG and TIFF have passed. Real mouse dragging
is unverified: the NSH synthetic drag produces an AppKit warning and no window
movement. A real pointer test is still required. Do not count native drag
behavior as verified merely from the engine's WindowDrag property check.

CI includes native builds and engine tests on macOS, Windows and Linux. The
workflow configuration is not evidence that remote CI jobs have passed.

## Remaining migration

- Finish visual parity for populated device lists, composer cards, history, icons
  and preview surfaces. The application menu is now anchored under the TitleBar
  with language/theme submenus, shortcuts, separators and the original links. Verify populated
  states against the original; restored home-region geometry alone is not proof
  of complete parity. The existing emblem and theme integration are restored.
- Add native file drag/drop. The currently resolved NSH core and its AppKit,
  GTK and Win32 backends expose no file-drop event; this requires an upstream
  capability, not a vendored patch. Video cards now restore the original distinct play
  marker; text cards restore a bounded content excerpt, and all composer items
  regain individual bordered cards. Native macOS clipboard imports of multiline
  text and a generated MP4 were visually checked at 640×520. This verifies card
  presentation, not video playback (which was not part of the original card).
- Finish the complete keyboard, modal isolation and focus-restoration matrix;
  improve history navigation and verify file-opening and clipboard-copy actions
  through the native OS services.
- Verify packaged update installation and relaunch on each native platform.
  The engine fixture covers progress, verification failure, retry, task ownership
  after hiding its surface, and refusal to restart with an unfinished Outbox.
  There is a known macOS lifecycle gap: the resolved NSH AppKit delegate calls
  `NSApplication.terminate` when the window closes. AppKit termination exits the
  process instead of returning through `run_app`, so the restart code after
  `run_app` in `src/main.rs` cannot be relied on for macOS updates. Upstream
  needs a graceful shutdown/return path before this behavior is complete;
  the engine fixture does not exercise AppKit termination.
- Preserve retained sessions on macOS window close and verify orderly network
  shutdown on application exit.
- Verify Linux/Windows release packaging and installers against the new native
  executable, and run the Intel macOS package on an Intel host. ARM macOS
  packaging, isolated install/upgrade and native startup have passed locally.
- Exercise real Linux Wayland and Windows windows, file pickers, image/media
  clipboard formats, reduced motion and interoperability with other LocalSend
  clients.
