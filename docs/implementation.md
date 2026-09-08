# OmaSend v0.1 implementation and acceptance

Spec: [RFC-OmaSend.md](../RFC-OmaSend.md). The source RFC ends before listing its development order; this document supplies the sequence without reducing its scope.

## Architecture

One Rust application owns a Tokio runtime, the official LocalSend discovery/server/client, and a GPUI Omarchy window. A thin adapter emits owned events; only the GPUI task updates views. App exit stops discovery, listeners and transfers. There is no daemon, CLI, database, cloud service or video playback stack.

LocalSend is pinned to upstream commit `6279d3e30d1d1290caee3b81549f8128a8b01d9f`. Enable `discovery`, which includes HTTP, TLS and multicast, without enabling WebRTC. Use the core's generated identity, mutual TLS and fingerprint-pinned clients. Never silently fall back to plaintext.

Use a single crate with `localsend/` for the adapter, `model/` for send items and transfer state, `clipboard/` for MIME selection and Wayland process access, and `views/` for GPUI composition. The optional desktop feature permits protocol tests without a display; it does not provide a CLI product.

## Ordered work

- [ ] Protocol adapter: bind HTTPS, discover automatically, accept/decline incoming requests, stream file/text transfers, expose progress/cancel/failure/completion, stop all tasks on exit. Integration tests must exercise real sockets, both directions, refusal, cancellation, fingerprint mismatch and port release.
- [ ] Unified inputs: file picker, drag/drop and clipboard produce the same SendItem pipeline. Expand folders into relative file names. Preserve original file URI paths. Own temporary image/video files until the last sender releases them. Test binary MIME priority, URI escaping, hostile URI rejection, text newlines, folder paths and temporary lifetime.
- [ ] Safe receives: resolve XDG Downloads; stage partial files privately; publish only verified files without overwriting, using numbered conflicts. Keep relative folder paths beneath Downloads. Test traversal, symlinks, simultaneous collisions, truncation and failed transfer cleanup.
- [ ] GPUI Omarchy window: nearby device selection, composer, thumbnails/enlarged image preview, video file card, receive decision, progress and in-memory history with Open/Show in Files. Use theme components and tokens; any reusable missing primitive belongs in the UI library.
- [ ] Keyboard and lifecycle: Ctrl+V, Ctrl+O, Escape, Enter, four arrows, Tab/Shift+Tab, focus restoration, visible selection, disappearing devices and responsive window sizes.
- [ ] Delivery: desktop entry and Linux build/runtime dependencies, build/test instructions, formatting/check/clippy/tests, real native-window exercise, Linux Wayland clipboard and cross-platform LocalSend interoperability matrix.

## Verification policy

Tests use literal payloads and real filesystem/socket boundaries. A saved file must match its offered bytes, a failed/cancelled upload must not be reported complete, and existing Downloads files must stay unchanged. Session completion waits for file results rather than assuming the core's SessionEnd implies successful saves.

Run `cargo test --no-default-features`, `cargo fmt --check`, `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings`. Native-window checks cover the actual keyboard path, minimum/default window sizes, light/dark themes, empty/error states, multiple files and long names. Linux and mobile-client checks remain explicitly unverified until exercised on those systems.

## Current verification evidence

- macOS: formatting, all-target compilation, Clippy with project warnings denied,
  and all 25 headless tests pass. The suite covers discovery scan bounds,
  language catalogs, MIME/process boundaries, state transitions, and real HTTPS
  transfer acceptance/refusal/cancellation/fingerprint/receive safety.
- Discovery publication is independent of slow announcement/probe work. Regression
  tests cover immediate new/renamed peers, bounded-channel recovery, expiry and
  deduplicated verification-failure diagnostics; TLS pinning remains enforced.
- Linux: final all-target desktop compilation and all 22 headless tests pass
  inside a Rust 1.95 Debian container. No real Wayland session has been tested.
- Native macOS screenshots confirm the English/Chinese interface, system machine
  name, titlebar emblem, and different frames of pixel discovery feedback.
  User-provided screenshot confirms sent and received transfers with a LocalSend
  desktop peer. iPhone interoperability remains unverified.
- GPUI keyboard regressions: `cargo test --features ui-tests --bin omasend`
  covers focused button Enter/Space activation, four-arrow device navigation,
  keyboard menu language selection, modal Tab/Shift+Tab containment, arrow
  isolation, and Escape restoration to the previous control. Nine UI tests now
  also cover virtual device-list scrolling, the chronological history sheet,
  and closing/reopening a retained session while input preparation finishes.
- Root POSIX installers pass eight mocked install/upgrade/rollback tests under
  sh and dash. Release workflow passes actionlint; macOS packaging includes
  Icon Composer assets and passes strict signature verification. Linux/Windows
  archive layout checks are not substitutes for builds on their CI runners.
- The bilingual website passes its production build and twelve browser tests,
  including installation tabs, first-viewport commands, theme and language
  controls, and reduced-motion behavior.
- gpui-omarchy 0.1.0 is pinned locally with its menu shortcut renderer using the
  existing keycap component and focus traversal respecting gpui-base modal traps.
- Remaining acceptance work: real Wayland clipboard/drag-drop/portal behavior,
  the complete keyboard/modal matrix, mobile peers, and PIN entry UX.
