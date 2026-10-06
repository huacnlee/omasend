# GPUI Fast compatibility facades

GPUI Kit 0.7.1 pins `gpui-pre` and `gpui-pre-platform` 0.3.8. These two small
facades patch those packages and re-export the published `gpui-fast` and
`gpui-fast-platform` packages, pinned to exactly 0.1.0. Features are forwarded
to GPUI Fast so GPUI Kit, assets, and native windows use the same engine.

This follows the facade approach in
https://github.com/longbridge/gpui-fast/tree/main/compat, using crates.io releases.
The app aliases GPUI Kit as `gpui` because GPUI Fast's macros emit `::gpui` paths.

When updating GPUI Kit, align the facade versions with its GPUI snapshot and
recheck the feature forwarding against GPUI Fast. Shared `gpui-pre-*` utility
crates stay on 0.3.8, the version also used by GPUI Fast 0.1.0.
