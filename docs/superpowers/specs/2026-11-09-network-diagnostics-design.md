# Network diagnostics report — design

Date: 2026-11-09 · Status: draft for approval

## Intent

When Omasend cannot see, or be seen by, other devices, users have no way to tell
why. On 2026-11-09 a real case took an hour of manual work: the app, firewall and
Local Network permission were all fine, yet a Windows peer on the same /24 never
appeared. ARP to the peer never resolved; `curl` *seemed* to reach it only
because a ClashMac rule (`IP-CIDR 10.20.94.0/23 → remote node`) tunnelled the
request; and the Mac used a Private Wi-Fi Address on a WPA2-Enterprise network.

Omasend should run those checks itself and produce **one Markdown report the
user copies and pastes into an AI assistant** (ChatGPT, Claude, …). The app
**does not diagnose**: it collects accurate, complete, clearly labelled facts.
The AI does the reasoning and walks the user through the fix.

## Goals

- One "Network diagnostics" surface, reachable from the existing menu.
- Running it takes roughly 10–20 seconds, shows progress, and never blocks the
  UI or the running transfer service. It keeps running if the surface is
  closed (cancelling was left out of the first version).
- The report starts with a short instruction for the AI in the app's current
  language, followed by facts. Copy puts the whole report on the clipboard.
- The report covers the ten checkpoints below, on every platform, with each
  check either producing data or stating explicitly that it is unavailable on
  this platform / failed and why. Missing data is never silently omitted.
- Facts that commonly mislead are annotated for the AI in the report itself,
  e.g. "proxy/TUN present: HTTP reachability through a proxy does not imply
  direct LAN reachability; all probes in this report bypass proxies".

## Non-goals

- Automatic fixes, or the app stating a root cause.
- Calling an AI from inside the app, or uploading the report anywhere.
- Changing discovery behaviour.
- Persistent history of reports.

## Anti-goals (failure even if it "works")

- Leaking secrets: proxy credentials (`http://user:pass@…`), full device
  fingerprints, Wi-Fi SSID/BSSID, MAC addresses of other devices.
- Probes that go through a proxy or VPN tunnel and so report false
  reachability (the exact trap from the real case).
- Prompting for admin/root, triggering OS permission dialogs other than ones
  Omasend already needs, or installing anything.
- Hanging: every external command and network probe has a timeout.
- Disturbing normal operation: the diagnostics must not stop/restart the
  server, change discovery settings, or flood the network beyond one scan.

## Checkpoints (approved by the user)

1. **Basics** — Omasend version, OS + version, architecture, UI kind
   (desktop/TUI/mobile), LocalSend protocol version, port, HTTP/HTTPS mode,
   device alias, device type, fingerprint (first 8 chars only).
2. **Service status** — whether the TCP listener and UDP multicast sockets on
   the port are bound (per interface, from the running node), and any startup
   or multicast error the node recorded (e.g. "Multicast unavailable").
3. **Interfaces** — for each non-loopback interface: name, IPv4 address with
   real prefix length, IPv6 presence, up/running, a "likely virtual" flag
   (utun/tun/tap/bridge/vEthernet/docker/veth/vmnet/wg/zt/tailscale, plus
   198.18.0.0/15 and 100.64.0.0/10 addresses), and whether **our own** MAC is
   locally administered (private/randomized). Our own MAC itself is not shown.
4. **Routing** — default route(s) with interface (all of them; the real case had
   a second scoped default on `bridge100`), and which interface the OS would use
   for 224.0.0.167 and for each scanned subnet.
5. **Proxy / VPN** — proxy environment variables (credentials redacted), system
   proxy settings where readable, TUN interfaces detected in (3), active VPN
   services where readable. Annotated for the AI as in Goals.
6. **Multicast self-test** — send announcements on every bound interface and
   listen for the test window. Report: whether our own loopback copy came back
   (proves the socket works locally), and every *other* device heard: source
   IP, receiving interface, alias, device model, protocol, and whether it came
   via multicast or via HTTP register.
7. **Direct scan** — TCP/HTTPS probe of port 53317 across each physical
   interface's subnet, **bypassing all proxies**. Report the interface's real
   prefix, the range actually scanned, and an explicit note when they differ
   (e.g. "/23 network, current discovery scans only /24"). List responding
   hosts with alias from `/api/localsend/v2/info`.
8. **Specific device test (optional)** — the user may type a peer IP before
   running. For it: neighbor/ARP resolution state, ICMP ping where available
   without privileges, direct TCP connect result with precise error kind
   (refused / timed out / host unreachable / host down), direct `/info` fetch
   (HTTPS, then HTTP), and the route/interface the OS picks for it.
9. **System** — firewall state (macOS application firewall incl. block-all and
   stealth; Windows firewall per profile and whether an Omasend rule exists;
   Linux ufw/firewalld if present), Windows current network category
   (Public/Private), macOS Local Network permission *as inferred* (a LAN connect
   failing with EHOSTUNREACH while the route is on-link is reported as
   "possibly denied"; the OS offers no direct query).
10. **Recent logs** — discovery/server-related lines from the in-memory log
    buffer (`src/diagnostics.rs`), last ~100, same redaction rules.

## Report format

Markdown, in this order:

1. AI instruction paragraph, in the app's current language (locale key), e.g.
   "我在使用 Omasend（LocalSend 协议的文件传输应用）时遇到设备发现/连接问题。
   以下是应用自动收集的诊断信息，请分析可能的原因，并给出我可以逐步操作的解决办法。
   如需更多信息请告诉我该运行什么命令。" plus a one-line user-description
   placeholder: "我的问题：<请在这里描述>".
2. Summary table: one row per checkpoint, status `ok` / `warning` /
   `failed` / `unavailable`, one-line fact. Status is mechanical (a check ran
   and returned data that matches a documented rule, e.g. "no other device
   heard" = warning), not a diagnosis.
3. One section per checkpoint with the details, in English, using stable
   headings and `key: value` lines or small tables so AIs parse it reliably.
4. Footer: generated time (local, with UTC offset) and test window length.

## Placement and UI

- New `Surface::Diagnostics` next to `Logs` and `About` in
  `src/views/utilities.rs`, opened from the menu.
- Content: optional peer IP input, Run/Cancel button, progress text naming the
  current check, then a read-only multiline editor with the report (same
  pattern as the logs surface) and Copy / Run again buttons, copy notice reused
  from logs.
- Works in the desktop window, TUI and mobile, since surfaces are shared.
- All interface text in `locales/app.yml`, every existing locale.

## Architecture constraints

- Collection lives in a UI-independent module (e.g. `src/diagnostics/` turning
  the current `diagnostics.rs` into a module directory, or a new
  `src/localsend/diagnostics.rs`) so it is testable headless with
  `cargo test --no-default-features`.
- Output is a typed `Report` value; Markdown rendering is a separate pure
  function. Redaction happens while building the `Report`, not at render time.
- The probe runs against the **running** node (its discovery handle, bound
  sockets, recorded errors) and must not start a second server on the port.
- Platform data that needs OS tools (firewall, routes, neighbor table, system
  proxy, network category) is gathered by running standard non-privileged
  commands with a short timeout each (~2 s), behind `cfg(target_os)`. Mobile
  platforms report these as `unavailable` unless an in-process API exists.
- Direct probes use a client with proxies explicitly disabled.
- Existing NSH/project conventions apply (see the `nsh` and `nsh-design-guides`
  skills and `src/views/*` patterns).

## Implementation notes (2026-11-09)

- Code: `src/netcheck/` (collection, report, redaction, OS commands),
  `src/views/diagnostics.rs` (surface), `Handle::probe()` in the adapter.
- `/info` probes use `LsHttpClientV2` with the node's own certificate:
  Omasend servers require mutual TLS and reject certificate-less clients.
- Peers are probed on port 53317 (or `IP:port` typed by the user), never on
  this node's own port. Announcements from this machine's own addresses are
  listed as "other apps/instances on this machine", not as LAN devices.
- IPv6 multicast is not monitored by the listener.

## Left to the builder

- Exact module layout, type names, and how the self-test hooks into
  `DiscoveryHandle` (events vs. an extra listening socket).
- Which OS commands to use per platform, as long as they are non-privileged and
  time-limited.
- Concurrency and timeout values within the ~15 s budget.
- Visual layout details within the NSH design guides.

## Testing

- Unit: Markdown rendering snapshot; redaction (proxy credentials, fingerprint,
  peer MACs); virtual-interface classification; locally administered MAC
  detection; prefix vs scanned-range note; status mapping.
- Integration (real sockets, loopback): multicast self-test sees a second
  in-process node; direct scan finds a node on loopback; specific device test
  distinguishes refused vs timed-out.
- Platform command parsers tested against captured sample outputs.
- Manual: run on macOS with ClashMac TUN active and confirm the report alone
  would have led to the 2026-11-09 conclusion.
