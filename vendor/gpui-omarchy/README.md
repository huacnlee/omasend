# gpui-omarchy

**gpui-omarchy is a [gpui-kit](https://gpui-kit.com/) component library designed specifically for the Omarchy system.** It follows Omarchy’s UI and UX conventions, helping developers build desktop applications that feel at home in Omarchy.

<img width="1920" height="1056" alt="image" src="https://github.com/user-attachments/assets/76be18ae-274d-493f-b4dc-967433121de1" />

The library brings Omarchy’s system themes, restrained visual style and keyboard-first interactions to reusable GPUI components. Built on **[gpui-base](https://gpui-kit.com/base/)**, it combines Omarchy presentation with shared focus, input, accessibility and composition primitives.

It also serves as a practical proving ground for **[gpui-base’s customization capabilities](https://gpui-kit.com/base/)**: building a complete Omarchy design system tests how freely applications can define their own visual identity and interaction patterns on top of base primitives. The project demonstrates that a shared behavioral foundation can support a fully custom design language, while exposing gaps to improve in gpui-base.

The project is under development, with 46 component previews in one gallery. Form, navigation, overlay and data components are still being expanded; the framework is not yet complete.

## Running the gallery

Install Rust and the platform-specific build dependencies required by GPUI. The gallery uses `.SystemUIFont`, so no additional fonts are needed. Icons use embedded SVGs from `gpui-kit-assets`, with inherited colors resolved during rendering; the application does not need to replace its AssetSource.

```sh
cargo run --example gallery
```

The single `gallery` example groups components under Actions, Forms, Navigation, Overlays and Display. Choose a component in the sidebar to inspect and interact with it. Use Up/Down, j/k or Home/End to navigate the sidebar, and Tab to enter the controls. The top Menu reloads the system theme, previews dark or light colors, and exits the gallery.

## Themes

`gpui_omarchy::init(cx)` initializes base and first reads:

```text
$HOME/.local/state/omarchy/current/theme/colors.toml
$HOME/.local/state/omarchy/current/theme.name
```

If the current directory does not exist, the loader supports the legacy `$HOME/.config/omarchy/current` location. If the current directory exists but its theme is invalid, it falls back to Tokyo Night without loading a stale legacy theme.

Both ANSI `color0..15` and semantic Omarchy color formats are supported. Missing files, insufficient permissions, invalid formats or missing required colors cause the entire palette to fall back to Tokyo Night. Non-Omarchy systems require no configuration.

Theme locations and the theme-name file follow the [Omarchy theme-set script](https://github.com/basecamp/omarchy/blob/master/bin/omarchy-theme-set). See the [ANSI theme](https://github.com/basecamp/omarchy/blob/master/themes/tokyo-night/colors.toml) and [semantic theme](https://github.com/basecamp/omarchy/blob/quattro/themes/tokyo-night/colors.toml) for color formats.

Native applications follow the system theme after `init`: a background check every second detects palette edits and theme symlink replacements, then updates all windows only when the theme changes. Applying an explicit theme with `Theme::apply(cx)` stops following; `Theme::follow_system(cx)` resumes it. `Theme::system_or_default()` remains a one-time snapshot. Browsers use the default palette without filesystem monitoring. `shell.toml` size and state overrides are not yet implemented. Flexoki Light is available for previewing light colors; the gallery follows the system theme by default.

## Usage

Add the library to your application:

```toml
[dependencies]
gpui-omarchy = "0.1.0"
```

Use `gpui_omarchy::gpui` for GPUI types and `gpui_omarchy::application()` for the desktop entry point. These are provided through GPUI Kit with its component facade disabled. See the [complete quick start](examples/hello.rs).

Call `gpui_omarchy::init(cx)` once at startup, then create components inside `Render`:

```rust,ignore
use gpui_omarchy::gpui::ParentElement;
use gpui_omarchy::{button, panel, ButtonVariant};

panel("Workspace", cx).child(
    button("save", "Save", ButtonVariant::Primary, cx)
        .on_click(cx.listener(|this, _, _, cx| {
            this.save();
            cx.notify();
        })),
)
```

Constructors return composable gpui-base elements with their native builder APIs. Button uses a thin `gpui_omarchy::Button` wrapper that retains base activation, focus, children and styling APIs while withholding hover, pressed and focus-visible styles when disabled. Calling `.disabled(true)` before or after styling has the same result. `gpui_omarchy::Link` likewise suppresses hover, pressed and focus-visible styling when disabled, while retaining base URL activation behavior. Applications own checkbox, switch, radio and toggle state: pass the current value on each render and update it in `on_change`. Avoid overriding these values after construction, which can leave visual indicators out of sync. Component IDs must be unique within the same parent.

Select and Combobox use an application-owned `Entity<ChoiceState>` and `ChoiceItem` options. Observe changes with `cx.observe`, then read the selected option's stable `value` through `state.selected()`. The constructors return customizable `gpui_base::Select` and `gpui_base::Combobox` elements. Combobox searches existing options in its popup; it does not create free-text values.

Wrap a window or form in `focus_scope("app")` to enable Tab and Shift+Tab traversal. Text fields and popups retain their own keyboard behavior.

`toggle_group(id, cx)` composes independent `toggle` children for multiple-choice filters. Add icons explicitly through child elements.

`button_group(...)` represents a single-choice setting; `tab_list(...)` switches between content pages. Both accept `ChoiceItem` options, the selected index and a callback, returning base RadioGroup and Tabs respectively. Each group has one Tab stop. Left/Right or h/l moves the cursor, and Enter or Space confirms. Use `tabs` and `tab` for custom compositions.

`with_tooltip(control, "Description")` preserves the control's type and adds a tooltip after 400 ms of hovering. `tooltip(text, cx)` returns a customizable tooltip surface. Icon buttons still need explicit accessible names.

`sheet(&focus, cx)` returns a base Sheet. Compose its right-side panel with `.surface(sheet_surface(cx).child(...))`. Focus a stable handle when opening and restore trigger focus in the close callback. Base handles focus trapping, Escape and backdrop dismissal.

`popover` returns a base Popover with square styling and keyboard isolation inside its content. Escape dismisses it and restores focus. `collapsible(open, cx)` returns a base Collapsible: regular children remain visible, while its content appears only when expanded.

`toast(id, cx)` returns a composable base Toast surface. Applications control its lifecycle, optionally using base ToastManager. The gallery displays notifications at the bottom right with Undo and Retry actions. Saved notifications expire after six seconds, with the timer paused during hover or focus; errors remain until handled manually.


`avatar(initials, cx)` returns a square base Avatar. Set its image slot with `.image(avatar_image(source))`. The application decides whether to fall back to initials when image loading fails.

`calendar(id, &state, cx)` uses base CalendarState, with month/year navigation, single-date or range selection and disabled-date matchers. `separator` and `vertical_separator` provide horizontal and vertical dividers.

`scrollbar(id, axis, &handle, cx)` returns a themed base Scrollbar. Share its handle with the scrollable content and render it last inside the same relative container. Vertical, horizontal and both-axis configurations are supported.

`virtual_list(view, id, sizes, render, cx)` returns a base VirtualList that builds only the visible range. Each height in `sizes` must match the corresponding rendered row. Use `.track_scroll(&handle)` to preserve position or navigate to an item. The gallery displays 1,000 activity records with varying heights.

`markdown(id, source, cx)` and `html(id, source, cx)` return base TextView elements with selectable text, links and themed document typography. State-backed TextViews can use `text_view_style(cx)`. Selection uses the application's root TextSelectionLayer.

`tree(&state, cx)` uses base TreeState. `resizable(id, axis, cx)` returns a base split container, with dragging and size constraints handled by base. `nav_stack(&state, cx)` preserves base push, pop and forward navigation state.

`color_picker(id, &state, window, cx)` uses base ColorPickerState with Hex input and HSLA sliders. Enter commits Hex input; Escape discards an uncommitted preview. Sliders apply immediately. Observe the state or subscribe to ColorPickerEvent to update application previews.

`date_picker(id, &state, cx)` uses DatePickerState. Its public calendar state supports ranges and disabled dates. Selecting a date closes the popup; Escape cancels and restores focus. `otp_input` uses base OtpState with an Omarchy wrapper for system clipboard paste and disabled interaction. Pasting replaces the code, filters non-digits, normalizes full-width digits, and truncates to the configured length.

`hover_card` provides a delayed supplementary preview; essential content should also be accessible on the main page. `dock_area` installs the Omarchy DockAreaRenderer. Its example demonstrates dragging, merging and splitting tabbed panels. Docking supports tabs and split layouts; floating layouts are not offered.

Input state uses `gpui_base::input::{InputState, TextareaState}`. See the [gallery implementation](examples/gallery/app.rs) for initialization examples.

## Validation

```sh
cargo fmt --check
cargo check --all-targets
cargo test --lib --test interaction --example gallery
```

Implementation boundaries and remaining work are recorded in the [design notes](docs/design.md).

If the gallery panics, it writes the error location and full backtrace to `gpui-omarchy-gallery-<PID>.panic.log` in the system temporary directory. The terminal also prints the report path. This diagnostic logging is limited to the example application.

## Website

The [website](website/README.md) presents GPUI Kit, gpui-base, and Omarchy Style with the actual Rust gallery compiled to WebAssembly and Cargo installation instructions. Built with Bun, Astro, Tailwind CSS, and Base UI.

Install the [WASM build tools](examples/gallery-wasm/README.md), then run:

```sh
bash examples/gallery-wasm/scripts/build.sh --release
cd website
bun install
bun run dev
```

## Publishing

The [Publish crate workflow](.github/workflows/publish.yml) publishes to crates.io when a `v*` tag is pushed. It checks that the tag exactly matches `v` followed by the version in `Cargo.toml`, runs formatting and tests, and verifies the package with `cargo publish --dry-run` before uploading.

Set the repository Actions secret `CARGO_REGISTRY_TOKEN` to a crates.io API token with permission to publish `gpui-omarchy`. The token is only passed to the final publish step.

To release:

1. Update `version` in `Cargo.toml` and regenerate `Cargo.lock` with `cargo check`.
2. Commit and push the release changes, including the workflow.
3. Create and push the matching tag, for example:

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

Use a new version for each release; crates.io does not allow replacing an existing version. A tag/version mismatch or failed check stops publication.

## License

[MIT](LICENSE) © 2026 Jason Lee (huacnlee).
