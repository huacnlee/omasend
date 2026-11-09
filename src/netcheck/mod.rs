//! Network diagnostics: runs the connection checks against the running node
//! and produces a [`Report`] meant to be pasted into an AI assistant.
//!
//! The report states facts, never a root cause. Every probe bypasses proxies,
//! because proxied reachability is the most misleading fact on a LAN.
pub mod probe;
pub mod redact;
pub mod report;
pub mod system;

pub use report::{Block, Report, Section, Status};

use crate::localsend::NodeProbe;
use localsend::{
    model::discovery::PROTOCOL_VERSION_V2,
    multicast::{DEFAULT_MULTICAST_GROUP, DEFAULT_PORT},
};
use probe::{ConnectOutcome, Heard, ScanResult};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, SystemTime},
};
use system::{CommandOutput, InterfaceV4, Interfaces};

const PROXY_NOTE: &str = "A proxy, TUN or VPN is present. Reachability through a proxy (e.g. curl or a browser) does not imply direct LAN reachability: proxy rules may tunnel LAN addresses to a remote node. All probes in this report bypass proxies. LocalSend multicast (TTL 1) and transfers never use a proxy.";

/// The checks in the order they run; each names its progress text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Basics,
    Interfaces,
    Routing,
    Proxy,
    Network,
    Peer,
    System,
    Logs,
}

impl Step {
    pub fn key(self) -> &'static str {
        match self {
            Self::Basics => "diagnostics.step.basics",
            Self::Interfaces => "diagnostics.step.interfaces",
            Self::Routing => "diagnostics.step.routing",
            Self::Proxy => "diagnostics.step.proxy",
            Self::Network => "diagnostics.step.network",
            Self::Peer => "diagnostics.step.peer",
            Self::System => "diagnostics.step.system",
            Self::Logs => "diagnostics.step.logs",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    /// A device the user wants tested specifically.
    pub peer: Option<SocketAddrV4>,
    /// "desktop", "terminal" or "mobile".
    pub frontend: &'static str,
    /// How long to listen for multicast announcements.
    pub window: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            peer: None,
            frontend: "desktop",
            window: Duration::from_secs(8),
        }
    }
}

/// Parses the optional peer the user typed: an IPv4 address, optionally
/// with a port; LocalSend's default port otherwise.
pub fn parse_peer(text: &str) -> Result<Option<SocketAddrV4>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<Ipv4Addr>()
        .map(|ip| SocketAddrV4::new(ip, DEFAULT_PORT))
        .or_else(|_| text.parse::<SocketAddrV4>())
        .map(Some)
        .map_err(|_| format!("Not an IPv4 address: {text}"))
}

/// Runs every check. `node` is the running node, or why it is unavailable.
pub async fn run(
    node: Result<NodeProbe, String>,
    options: Options,
    progress: impl Fn(Step),
) -> Report {
    let started = SystemTime::now();
    let mut sections = Vec::new();

    progress(Step::Basics);
    sections.push(basics(&node, &options).await);

    progress(Step::Interfaces);
    let interfaces = system::interfaces();
    sections.push(interfaces_section(&interfaces).await);
    let interfaces = interfaces.unwrap_or_default();

    progress(Step::Routing);
    sections.push(routing_section().await);

    progress(Step::Proxy);
    sections.push(proxy_section(&interfaces).await);

    progress(Step::Network);
    // Peers listen on the protocol's port, whatever port this node got.
    let port = DEFAULT_PORT;
    let client = node
        .as_ref()
        .ok()
        .and_then(|node| probe::info_client(&node.identity));
    let lan: Vec<InterfaceV4> = interfaces
        .v4
        .iter()
        .filter(|i| i.is_lan())
        .cloned()
        .collect();
    let listen_on: Vec<Ipv4Addr> = interfaces
        .v4
        .iter()
        .filter(|i| i.up)
        .map(|i| i.address)
        .collect();
    let own_fingerprint = node
        .as_ref()
        .map(|node| node.device.fingerprint.clone())
        .unwrap_or_default();
    let announce = async {
        if let Ok(NodeProbe {
            discovery: Some(discovery),
            ..
        }) = &node
        {
            discovery.announce().await;
        }
    };
    let listening = probe::listen_multicast(
        DEFAULT_MULTICAST_GROUP,
        DEFAULT_PORT,
        &listen_on,
        &own_fingerprint,
        options.window,
        announce,
    );
    let scanning = async {
        let mut results = Vec::new();
        for interface in &lan {
            results.push((
                interface.clone(),
                probe::scan(interface.address, interface.prefix, port, client.as_ref()).await,
            ));
        }
        results
    };
    let (heard, scans) = tokio::join!(listening, scanning);
    sections.insert(1, service_section(&node, &heard, &interfaces).await);
    sections.push(multicast_section(&node, &heard, &interfaces, started));
    sections.push(scan_section(&lan, &scans));

    progress(Step::Peer);
    sections.push(peer_section(options.peer, client.as_ref()).await);

    progress(Step::System);
    sections.push(system_section(&scans).await);

    progress(Step::Logs);
    sections.push(logs_section());

    Report {
        generated: chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S %:z")
            .to_string(),
        window_secs: options.window.as_secs(),
        sections,
    }
}

async fn basics(node: &Result<NodeProbe, String>, options: &Options) -> Section {
    let os = format!(
        "{} {}",
        system::os_name(),
        system::os_version().await.trim()
    );
    let mut fields = vec![
        ("Omasend version", env!("CARGO_PKG_VERSION").to_owned()),
        ("OS", os.clone()),
        ("Architecture", std::env::consts::ARCH.to_owned()),
        ("Frontend", options.frontend.to_owned()),
        ("LocalSend protocol", PROTOCOL_VERSION_V2.to_owned()),
    ];
    match node {
        Ok(node) => {
            fields.push(("Port", node.device.port.to_string()));
            fields.push(("Transport", node.protocol.as_str().to_owned()));
            fields.push(("Alias", node.device.alias.clone()));
            fields.push(("Device model", node.device.model.clone()));
            fields.push((
                "Fingerprint (first 8)",
                redact::short_fingerprint(&node.device.fingerprint),
            ));
        }
        Err(error) => fields.push(("Node", format!("not running: {error}"))),
    }
    Section::new(
        "Basics",
        Status::Ok,
        format!("Omasend {} on {os}", env!("CARGO_PKG_VERSION")),
    )
    .fields(fields)
}

async fn service_section(
    node: &Result<NodeProbe, String>,
    heard: &std::io::Result<(Vec<Heard>, Vec<String>)>,
    interfaces: &Interfaces,
) -> Section {
    let node = match node {
        Ok(node) => node,
        Err(error) => {
            return Section::new(
                "Service status",
                Status::Failed,
                format!("The LocalSend node is not running: {error}"),
            );
        }
    };
    let port = node.device.port;
    let (tcp, tcp_detail) = probe::connect(
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)),
        Duration::from_secs(2),
    )
    .await;
    let own_sources: Vec<String> = match heard {
        Ok((heard, _)) => heard
            .iter()
            .filter(|entry| entry.own)
            .map(|entry| {
                format!(
                    "{} ({})",
                    entry.source,
                    interface_for(interfaces, entry.source).unwrap_or("unknown interface")
                )
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    let mut status = Status::Ok;
    let mut problems = Vec::new();
    if tcp != ConnectOutcome::Open {
        status = Status::Failed;
        problems.push(format!("TCP listener on port {port} not reachable locally"));
    }
    if node.discovery.is_none() {
        status = Status::Failed;
        problems.push("discovery is disabled".to_owned());
    }
    if let Some(error) = &node.multicast_error {
        status = Status::Failed;
        problems.push(format!("multicast unavailable: {error}"));
    }
    if own_sources.is_empty() && heard.is_ok() && node.multicast_error.is_none() {
        if status == Status::Ok {
            status = Status::Warning;
        }
        problems.push("no own announcement came back on any interface".to_owned());
    }
    let summary = if problems.is_empty() {
        format!(
            "Listening on TCP {port}; own announcements sent from {} interface address(es)",
            own_sources.len()
        )
    } else {
        problems.join("; ")
    };
    Section::new("Service status", status, summary)
        .fields(vec![
            (
                "TCP listener (127.0.0.1)",
                format!("{} — {tcp_detail}", tcp.label()),
            ),
            (
                "Discovery",
                if node.discovery.is_some() { "running" } else { "disabled" }.to_owned(),
            ),
            (
                "Multicast sockets",
                node.multicast_error
                    .clone()
                    .map(|error| format!("unavailable: {}", redact::text(&error)))
                    .unwrap_or_else(|| "bound".into()),
            ),
            (
                "Own announcements looped back from",
                if own_sources.is_empty() {
                    "none".into()
                } else {
                    own_sources.join(", ")
                },
            ),
        ])
        .note("Looped-back own announcements prove the node sends on those interfaces; they do not prove the packets left this machine or reached other devices.")
}

async fn interfaces_section(interfaces: &std::io::Result<Interfaces>) -> Section {
    let interfaces = match interfaces {
        Ok(interfaces) => interfaces,
        Err(error) => {
            return Section::new(
                "Network interfaces",
                Status::Failed,
                format!("Cannot list interfaces: {error}"),
            );
        }
    };
    let mut rows = Vec::new();
    for interface in &interfaces.v4 {
        let mac = if interface.is_lan() {
            match system::own_mac(&interface.name)
                .await
                .as_deref()
                .map(system::locally_administered)
            {
                Some(Some(true)) => "private/randomized (locally administered)",
                Some(Some(false)) => "hardware (globally unique)",
                _ => "unknown",
            }
        } else {
            "-"
        };
        rows.push(vec![
            interface.name.clone(),
            format!("{}/{}", interface.address, interface.prefix),
            if interface.up { "up" } else { "down" }.into(),
            if interface.is_virtual() {
                "likely virtual (VPN/TUN/VM/bridge)"
            } else {
                "physical"
            }
            .into(),
            if interface.is_lan() { "yes" } else { "no" }.into(),
            mac.into(),
            if interfaces.with_v6.contains(&interface.name) {
                "yes"
            } else {
                "no"
            }
            .into(),
        ]);
    }
    let lan = interfaces.v4.iter().filter(|i| i.is_lan()).count();
    let virtual_count = interfaces.v4.iter().filter(|i| i.is_virtual()).count();
    let status = if lan == 0 {
        Status::Warning
    } else {
        Status::Ok
    };
    Section::new(
        "Network interfaces",
        status,
        format!(
            "{lan} LAN interface(s), {virtual_count} likely virtual, {} IPv4 address(es) in total",
            interfaces.v4.len()
        ),
    )
    .block(Block::Table {
        headers: vec!["Interface", "IPv4/prefix", "State", "Kind", "Used for LAN checks", "Own MAC", "IPv6"],
        rows,
    })
    .note("Only this device's own MAC type is reported, not the address. A private (randomized) Wi-Fi address can be placed in a restricted role on enterprise (802.1X) networks.")
}

async fn routing_section() -> Section {
    let outputs = system::routing().await;
    if outputs.is_empty() {
        return Section::new(
            "Routing",
            Status::Unavailable,
            "Not available on this platform",
        );
    }
    let mut section = Section::new(
        "Routing",
        Status::Ok,
        "Default routes and the route chosen for the LocalSend multicast group 224.0.0.167",
    );
    for output in outputs {
        section = section.block(code(output));
    }
    section.note("More than one default route, or the multicast route on a virtual interface, can send discovery traffic to the wrong network.")
}

async fn proxy_section(interfaces: &Interfaces) -> Section {
    let environment = system::proxy_environment();
    let outputs = system::proxy_and_vpn().await;
    let tun: Vec<String> = interfaces
        .v4
        .iter()
        .filter(|interface| interface.is_virtual())
        .map(|interface| format!("{} ({})", interface.name, interface.address))
        .collect();
    let system_proxy = outputs.iter().any(|output| {
        let text = output.text.to_ascii_lowercase();
        text.contains("enable : 1")
            || text.contains("proxyenable    reg_dword    0x1")
            || text.contains("'manual'")
            || (output.label.contains("--nc") && text.contains("(connected)"))
    });
    // NO_PROXY alone configures nothing.
    let proxies = environment
        .iter()
        .filter(|(key, _)| !key.eq_ignore_ascii_case("no_proxy"))
        .count();
    let present = proxies > 0 || system_proxy || !tun.is_empty();
    let mut section = Section::new(
        "Proxy / VPN",
        if present { Status::Warning } else { Status::Ok },
        if present {
            format!(
                "Proxy or tunnel present: {} env var(s), system proxy {}, {} virtual interface(s)",
                proxies,
                if system_proxy { "on" } else { "off" },
                tun.len()
            )
        } else {
            "No proxy, TUN or VPN detected".into()
        },
    )
    .fields(vec![
        (
            "Proxy environment",
            if environment.is_empty() {
                "none".into()
            } else {
                environment
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ),
        (
            "Virtual interfaces",
            if tun.is_empty() {
                "none".into()
            } else {
                tun.join(", ")
            },
        ),
    ]);
    for output in outputs {
        section = section.block(code(output));
    }
    if present {
        section = section.note(PROXY_NOTE);
    }
    section
}

fn multicast_section(
    node: &Result<NodeProbe, String>,
    heard: &std::io::Result<(Vec<Heard>, Vec<String>)>,
    interfaces: &Interfaces,
    started: SystemTime,
) -> Section {
    let confirmed: Vec<Vec<String>> = match node {
        Ok(NodeProbe {
            discovery: Some(discovery),
            ..
        }) => discovery
            .devices()
            .into_iter()
            .filter_map(|state| {
                let last = state.logs.last()?;
                let host = state.device.http().map(|channel| {
                    format!(
                        "{}:{} ({})",
                        channel.host,
                        channel.port,
                        channel.protocol.as_str()
                    )
                });
                let age = SystemTime::now()
                    .duration_since(last.timestamp)
                    .unwrap_or_default()
                    .as_secs();
                Some(vec![
                    state.device.alias.clone(),
                    host.unwrap_or_else(|| "-".into()),
                    state.device.device_model.clone().unwrap_or_default(),
                    format!("{age} s ago"),
                    if last.timestamp >= started {
                        "yes"
                    } else {
                        "no"
                    }
                    .into(),
                ])
            })
            .collect(),
        _ => Vec::new(),
    };
    let (status, summary, mut blocks) = match heard {
        Ok((heard, join_errors)) => {
            // Another app or instance on this machine answers from our own
            // addresses; it says nothing about the LAN.
            let local = |entry: &&Heard| {
                interfaces
                    .v4
                    .iter()
                    .any(|interface| IpAddr::V4(interface.address) == entry.source)
            };
            let same_machine: Vec<String> = heard
                .iter()
                .filter(|entry| !entry.own)
                .filter(local)
                .map(|entry| format!("{} \"{}\" ({})", entry.source, entry.alias, entry.model))
                .collect();
            let others: Vec<&Heard> = heard
                .iter()
                .filter(|entry| !entry.own)
                .filter(|entry| !local(entry))
                .collect();
            let rows = others
                .iter()
                .map(|entry| {
                    vec![
                        entry.source.to_string(),
                        interface_for(interfaces, entry.source)
                            .unwrap_or("?")
                            .to_owned(),
                        entry.alias.clone(),
                        entry.model.clone(),
                        entry.protocol.clone(),
                        if entry.announce { "announce" } else { "reply" }.into(),
                        entry.count.to_string(),
                    ]
                })
                .collect();
            let mut blocks = vec![Block::Table {
                headers: vec![
                    "Source IP",
                    "Interface",
                    "Alias",
                    "Model",
                    "Protocol",
                    "Kind",
                    "Packets",
                ],
                rows,
            }];
            if !same_machine.is_empty() {
                blocks.push(Block::Fields(vec![(
                    "Other LocalSend apps/instances on this machine".into(),
                    same_machine.join(", "),
                )]));
            }
            if !join_errors.is_empty() {
                blocks.push(Block::Fields(vec![(
                    "Group join errors".into(),
                    join_errors.join("; "),
                )]));
            }
            if others.is_empty() {
                (
                    Status::Warning,
                    "No other device's multicast packet was heard".to_owned(),
                    blocks,
                )
            } else {
                (
                    Status::Ok,
                    format!("Heard {} other device(s) over multicast", others.len()),
                    blocks,
                )
            }
        }
        Err(error) => (
            Status::Failed,
            format!("Could not listen for multicast: {error}"),
            Vec::new(),
        ),
    };
    blocks.push(Block::Fields(vec![(
        "Devices known to the running node".into(),
        format!(
            "{} (table below; 'In window' = confirmed during this test)",
            confirmed.len()
        ),
    )]));
    blocks.push(Block::Table {
        headers: vec!["Alias", "Address", "Model", "Last seen", "In window"],
        rows: confirmed,
    });
    blocks.push(Block::Note("The node announced itself on every interface during the window. LocalSend peers usually answer an announcement by HTTP register instead of multicast, so a peer may appear only as known to the node. Hearing nothing from devices that see each other suggests this device is isolated from them (different subnet/VLAN, Wi-Fi client isolation, or multicast filtering).".into()));
    Section {
        title: "Multicast discovery",
        status,
        summary,
        blocks,
    }
}

fn scan_section(lan: &[InterfaceV4], scans: &[(InterfaceV4, ScanResult)]) -> Section {
    if lan.is_empty() {
        return Section::new(
            "Direct scan",
            Status::Warning,
            "No LAN interface to scan (no up, private, physical IPv4 interface)",
        );
    }
    let mut found_rows = Vec::new();
    let mut tally_rows = Vec::new();
    let mut fields = Vec::new();
    for (interface, result) in scans {
        let network = result
            .network
            .map(|network| format!("{network}/{}", result.scanned_prefix))
            .unwrap_or_default();
        let coverage = if result.scanned_prefix == interface.prefix {
            "whole subnet".to_owned()
        } else {
            format!(
                "only part of the /{} subnet (scan capped at /{})",
                interface.prefix, result.scanned_prefix
            )
        };
        fields.push((
            format!(
                "{} {}/{}",
                interface.name, interface.address, interface.prefix
            ),
            format!("scanned {network}, {} host(s), {coverage}", result.probed),
        ));
        let outcomes = result
            .outcomes
            .iter()
            .map(|(outcome, count)| format!("{}: {count}", outcome.label()))
            .collect::<Vec<_>>()
            .join(", ");
        tally_rows.push(vec![interface.name.clone(), outcomes]);
        for (host, answer) in &result.found {
            found_rows.push(match answer {
                Ok(info) => vec![
                    host.to_string(),
                    interface.name.clone(),
                    info.alias.clone(),
                    info.model.clone(),
                    info.device_type.clone(),
                    format!("{} v{}", info.scheme, info.version),
                ],
                Err(error) => vec![
                    host.to_string(),
                    interface.name.clone(),
                    "-".into(),
                    "-".into(),
                    "-".into(),
                    format!("port open, /info failed: {}", redact::text(error)),
                ],
            });
        }
    }
    let found = found_rows.len();
    Section {
        title: "Direct scan",
        status: if found == 0 { Status::Warning } else { Status::Ok },
        summary: format!("{found} host(s) answered on the LocalSend port directly (no proxy)"),
        blocks: vec![
            Block::Fields(fields),
            Block::Table {
                headers: vec!["IP", "Interface", "Alias", "Model", "Type", "Answer"],
                rows: found_rows,
            },
            Block::Table {
                headers: vec!["Interface", "Connect outcomes"],
                rows: tally_rows,
            },
            Block::Note("Many 'host down' or 'timed out' results are normal for unused addresses. 'refused' means the host answered, so the LAN path works. If no outcome is 'refused' or 'open' at all, direct LAN traffic from this app is likely blocked (OS permission, firewall or Wi-Fi isolation).".into()),
        ],
    }
}

async fn peer_section(peer: Option<SocketAddrV4>, client: Option<&probe::InfoClient>) -> Section {
    let Some(target) = peer else {
        return Section::new(
            "Specific device test",
            Status::Unavailable,
            "Not requested (no device IP entered)",
        );
    };
    let (peer, port) = (*target.ip(), target.port());
    let (tcp, tcp_detail) = probe::connect(SocketAddr::V4(target), Duration::from_secs(3)).await;
    let info = match client {
        Some(client) => probe::info(client, IpAddr::V4(peer), port).await,
        None => Err("no HTTP client".into()),
    };
    let (status, summary) = match (&info, tcp) {
        (Ok(info), _) => (
            Status::Ok,
            format!(
                "{target} answered directly as \"{}\" ({})",
                info.alias, info.model
            ),
        ),
        (Err(_), ConnectOutcome::Open) => (
            Status::Warning,
            format!("{peer}:{port} accepts TCP but /info failed"),
        ),
        (Err(_), outcome) => (
            Status::Failed,
            format!("{peer}:{port} not reachable directly: {}", outcome.label()),
        ),
    };
    let mut section = Section::new("Specific device test", status, summary).fields(vec![
        ("Target", format!("{peer}:{port}")),
        (
            "Direct TCP connect",
            format!("{} — {tcp_detail}", tcp.label()),
        ),
        (
            "Direct /info",
            match &info {
                Ok(info) => format!(
                    "{} — alias \"{}\", model {}, type {}, protocol v{}",
                    info.scheme, info.alias, info.model, info.device_type, info.version
                ),
                Err(error) => redact::text(error),
            },
        ),
    ]);
    for output in [
        system::route_to(peer).await,
        system::neighbor(peer).await,
        system::ping(peer).await,
    ]
    .into_iter()
    .flatten()
    {
        section = section.block(code(output));
    }
    section.note("An '(incomplete)' or missing neighbor (ARP) entry means the peer never answered at layer 2, so no app on this device can reach it directly, whatever a proxy shows.")
}

async fn system_section(scans: &[(InterfaceV4, ScanResult)]) -> Section {
    let outputs = system::firewall().await;
    let mut totals: BTreeMap<ConnectOutcome, usize> = BTreeMap::new();
    for (_, result) in scans {
        for (outcome, count) in &result.outcomes {
            *totals.entry(*outcome).or_default() += count;
        }
    }
    let reached = totals.get(&ConnectOutcome::Refused).copied().unwrap_or(0)
        + totals.get(&ConnectOutcome::Open).copied().unwrap_or(0);
    let blocked = totals
        .get(&ConnectOutcome::HostUnreachable)
        .copied()
        .unwrap_or(0)
        + totals
            .get(&ConnectOutcome::PermissionDenied)
            .copied()
            .unwrap_or(0);
    let mut fields = Vec::new();
    let mut status = Status::Ok;
    if cfg!(any(target_os = "macos", target_os = "ios")) {
        let inference = if scans.is_empty() {
            "unknown (no LAN scan ran)".to_owned()
        } else if reached == 0 && blocked > 0 {
            status = Status::Warning;
            format!(
                "possibly denied: no LAN host answered and {blocked} connect(s) failed as unreachable/denied"
            )
        } else if reached > 0 {
            format!("granted: {reached} LAN host(s) answered direct connects")
        } else {
            "unknown: no LAN host answered, but failures do not look like a permission denial"
                .into()
        };
        fields.push((
            "Local Network permission (inferred, cannot be queried)",
            inference,
        ));
    }
    let firewall_on = outputs.iter().any(|output| {
        let text = output.text.to_ascii_lowercase();
        text.contains("firewall is enabled")
            || text.contains("block all state set to enabled")
            || text.contains("status: active")
    });
    let public_network = outputs
        .iter()
        .any(|output| output.text.lines().any(|line| line.contains(" Public ")));
    if firewall_on || public_network {
        status = Status::Warning;
    }
    let summary = match (firewall_on, public_network) {
        (true, true) => "Firewall enabled and a network is in the Public category",
        (true, false) => "Firewall enabled",
        (false, true) => "A network is in the Public category",
        (false, false) => "No firewall restriction detected",
    };
    let mut section = Section::new("System", status, summary).fields(fields);
    if outputs.is_empty() {
        section = section.note("Firewall state cannot be read on this platform.");
    }
    for output in outputs {
        section = section.block(code(output));
    }
    section
}

fn logs_section() -> Section {
    let logs = crate::diagnostics::snapshot();
    let lines: Vec<&str> = logs.lines().collect();
    let kept = &lines[lines.len().saturating_sub(100)..];
    Section::new(
        "Recent logs",
        Status::Ok,
        format!("Last {} log line(s) of this session", kept.len()),
    )
    .block(Block::Code {
        label: "log".into(),
        text: redact::text(&kept.join("\n")),
    })
}

fn code(output: CommandOutput) -> Block {
    Block::Code {
        label: output.label,
        text: output.text,
    }
}

/// The interface whose subnet contains `address`.
fn interface_for(interfaces: &Interfaces, address: IpAddr) -> Option<&str> {
    let IpAddr::V4(address) = address else {
        return None;
    };
    interfaces
        .v4
        .iter()
        .find(|interface| interface.address == address)
        .or_else(|| {
            interfaces.v4.iter().find(|interface| {
                let mask = u32::MAX.checked_shl(32 - interface.prefix).unwrap_or(0);
                u32::from(interface.address) & mask == u32::from(address) & mask
            })
        })
        .map(|interface| interface.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_optional_peer() {
        assert_eq!(parse_peer("  "), Ok(None));
        assert_eq!(
            parse_peer(" 10.20.94.213 "),
            Ok(Some(SocketAddrV4::new(
                Ipv4Addr::new(10, 20, 94, 213),
                DEFAULT_PORT
            )))
        );
        assert_eq!(
            parse_peer("10.20.94.213:53318"),
            Ok(Some(SocketAddrV4::new(
                Ipv4Addr::new(10, 20, 94, 213),
                53318
            )))
        );
        assert!(parse_peer("10.20.94").is_err());
        assert!(parse_peer("10.20.94.213:x").is_err());
    }

    #[test]
    fn maps_sources_to_interfaces() {
        let interfaces = Interfaces {
            v4: vec![
                InterfaceV4 {
                    name: "en0".into(),
                    address: Ipv4Addr::new(10, 20, 94, 240),
                    prefix: 23,
                    up: true,
                    point_to_point: false,
                },
                InterfaceV4 {
                    name: "utun5".into(),
                    address: Ipv4Addr::new(198, 18, 0, 1),
                    prefix: 30,
                    up: true,
                    point_to_point: true,
                },
            ],
            with_v6: vec![],
        };
        let at = |ip: [u8; 4]| interface_for(&interfaces, IpAddr::from(ip));
        assert_eq!(at([10, 20, 95, 7]), Some("en0"));
        assert_eq!(at([198, 18, 0, 1]), Some("utun5"));
        assert_eq!(at([192, 168, 1, 1]), None);
    }

    #[test]
    fn scan_section_explains_partial_coverage() {
        let interface = InterfaceV4 {
            name: "en0".into(),
            address: Ipv4Addr::new(10, 1, 2, 3),
            prefix: 16,
            up: true,
            point_to_point: false,
        };
        let result = ScanResult {
            network: Some(Ipv4Addr::new(10, 1, 2, 0)),
            scanned_prefix: 24,
            probed: 253,
            outcomes: BTreeMap::from([(ConnectOutcome::TimedOut, 253)]),
            found: vec![],
        };
        let section = scan_section(
            std::slice::from_ref(&interface),
            &[(interface.clone(), result)],
        );
        assert_eq!(section.status, Status::Warning);
        let markdown = Report {
            generated: String::new(),
            window_secs: 0,
            sections: vec![section],
        }
        .to_markdown(crate::i18n::Language::En);
        assert!(markdown.contains("only part of the /16 subnet (scan capped at /24)"));
        assert!(markdown.contains("timed out (no answer): 253"));
    }

    /// The whole run against a real node on loopback-only settings: every
    /// section is present and the node's own TCP listener is found.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn full_run_produces_every_section() {
        let downloads =
            std::env::temp_dir().join(format!("omasend-netcheck-{}", std::process::id()));
        let node = crate::localsend::Node::start(crate::localsend::NodeConfig {
            alias: "Diagnostics Test".into(),
            port: 0,
            downloads: downloads.clone(),
            discovery: false,
        })
        .await
        .unwrap();
        let port = node.device.port;
        let probe = node.handle.probe().await.map_err(|error| error.to_string());
        let report = run(
            probe,
            Options {
                peer: Some(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)),
                frontend: "test",
                window: Duration::from_millis(300),
            },
            |_| {},
        )
        .await;
        node.shutdown().await;
        let _ = std::fs::remove_dir_all(downloads);
        let titles: Vec<&str> = report
            .sections
            .iter()
            .map(|section| section.title)
            .collect();
        assert_eq!(
            titles,
            [
                "Basics",
                "Service status",
                "Network interfaces",
                "Routing",
                "Proxy / VPN",
                "Multicast discovery",
                "Direct scan",
                "Specific device test",
                "System",
                "Recent logs"
            ]
        );
        let markdown = report.to_markdown(crate::i18n::Language::En);
        assert!(markdown.contains("Frontend: test"));
        assert!(markdown.contains("- TCP listener (127.0.0.1): open"));
        assert!(markdown.contains("Discovery: disabled"));
        // The peer test hits the node itself on loopback.
        assert!(
            markdown.contains("answered directly as \"Diagnostics Test\""),
            "{markdown}"
        );
    }
}
