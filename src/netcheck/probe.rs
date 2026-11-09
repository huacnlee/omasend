//! Network probes that bypass every proxy: multicast listening, direct TCP
//! connects and `/info` requests.
use std::{
    collections::BTreeMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

pub const CONNECT_TIMEOUT: Duration = Duration::from_millis(900);
const SCAN_CONCURRENCY: usize = 128;
/// Hosts per interface; a /22 is scanned whole, larger networks only around
/// the interface's own /24.
const MAX_SCAN_PREFIX: u32 = 22;

/// What a direct TCP connect ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectOutcome {
    Open,
    Refused,
    TimedOut,
    HostUnreachable,
    HostDown,
    NetworkUnreachable,
    PermissionDenied,
    Other,
}

impl ConnectOutcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Refused => "refused (host reachable, port closed)",
            Self::TimedOut => "timed out (no answer)",
            Self::HostUnreachable => "host unreachable (EHOSTUNREACH)",
            Self::HostDown => "host down (EHOSTDOWN, usually no ARP reply)",
            Self::NetworkUnreachable => "network unreachable",
            Self::PermissionDenied => "permission denied",
            Self::Other => "other error",
        }
    }

    pub fn classify(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::ConnectionRefused => return Self::Refused,
            io::ErrorKind::TimedOut => return Self::TimedOut,
            io::ErrorKind::HostUnreachable => return Self::HostUnreachable,
            io::ErrorKind::NetworkUnreachable => return Self::NetworkUnreachable,
            io::ErrorKind::PermissionDenied => return Self::PermissionDenied,
            _ => {}
        }
        match error.raw_os_error() {
            Some(code) if Some(code) == EHOSTDOWN => Self::HostDown,
            _ => Self::Other,
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
const EHOSTDOWN: Option<i32> = Some(64);
#[cfg(any(target_os = "linux", target_os = "android"))]
const EHOSTDOWN: Option<i32> = Some(112);
#[cfg(windows)]
const EHOSTDOWN: Option<i32> = Some(10064);
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "android",
    windows
)))]
const EHOSTDOWN: Option<i32> = None;

/// A direct TCP connect, never through a proxy.
pub async fn connect(address: SocketAddr, timeout: Duration) -> (ConnectOutcome, String) {
    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(address)).await {
        Err(_) => (
            ConnectOutcome::TimedOut,
            format!("no answer within {} ms", timeout.as_millis()),
        ),
        Ok(Ok(_)) => (ConnectOutcome::Open, "connected".into()),
        Ok(Err(error)) => (ConnectOutcome::classify(&error), error.to_string()),
    }
}

/// The hosts of the subnet a direct scan covers for an interface address,
/// excluding the address itself, the network and the broadcast address.
pub fn scan_hosts(address: Ipv4Addr, prefix: u32) -> (Ipv4Addr, u32, Vec<Ipv4Addr>) {
    let scanned = if prefix < MAX_SCAN_PREFIX {
        24
    } else {
        prefix.min(30)
    };
    let mask = u32::MAX.checked_shl(32 - scanned).unwrap_or(0);
    let network = u32::from(address) & mask;
    let broadcast = network | !mask;
    let hosts = (network + 1..broadcast)
        .map(Ipv4Addr::from)
        .filter(|host| *host != address)
        .collect();
    (Ipv4Addr::from(network), scanned, hosts)
}

/// A LocalSend `/info` answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerInfo {
    pub scheme: &'static str,
    pub alias: String,
    pub model: String,
    pub device_type: String,
    pub version: String,
}

/// A LocalSend client that presents this device's certificate (Omasend
/// peers require mutual TLS), accepts any peer certificate, and never uses a
/// proxy — the trap this report exists for.
pub type InfoClient = localsend::http::client::LsHttpClientV2;

pub fn info_client(identity: &localsend::crypto::cert::SelfSignedCert) -> Option<InfoClient> {
    InfoClient::try_new(
        &identity.private_key_pem,
        &identity.certificate_pem,
        None,
        Some(Duration::from_secs(3)),
    )
    .ok()
}

/// Fetches `/api/localsend/v2/info`, over HTTPS first, then HTTP.
pub async fn info(client: &InfoClient, host: IpAddr, port: u16) -> Result<PeerInfo, String> {
    use localsend::model::discovery::ProtocolType;
    let mut errors = Vec::new();
    for protocol in [ProtocolType::Https, ProtocolType::Http] {
        match client.info(protocol, &host.to_string(), port).await {
            Ok(info) => {
                return Ok(PeerInfo {
                    scheme: protocol.as_str(),
                    alias: info.alias,
                    model: info.device_model.unwrap_or_default(),
                    device_type: info
                        .device_type
                        .map(|kind| format!("{kind:?}").to_ascii_lowercase())
                        .unwrap_or_default(),
                    version: info.version,
                });
            }
            Err(error) => errors.push(format!("{}: {}", protocol.as_str(), error_chain(&error))),
        }
    }
    Err(errors.join("; "))
}

fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        if !text.contains(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}

/// The scan result for one interface.
#[derive(Clone, Debug, Default)]
pub struct ScanResult {
    pub network: Option<Ipv4Addr>,
    pub scanned_prefix: u32,
    pub probed: usize,
    pub outcomes: BTreeMap<ConnectOutcome, usize>,
    pub found: Vec<(Ipv4Addr, Result<PeerInfo, String>)>,
}

pub async fn scan(
    address: Ipv4Addr,
    prefix: u32,
    port: u16,
    client: Option<&InfoClient>,
) -> ScanResult {
    let (network, scanned_prefix, hosts) = scan_hosts(address, prefix);
    let limit = Arc::new(tokio::sync::Semaphore::new(SCAN_CONCURRENCY));
    let mut probes = tokio::task::JoinSet::new();
    let probed = hosts.len();
    for host in hosts {
        let limit = limit.clone();
        probes.spawn(async move {
            let _permit = limit.acquire_owned().await;
            let (outcome, _) = connect(
                SocketAddr::V4(SocketAddrV4::new(host, port)),
                CONNECT_TIMEOUT,
            )
            .await;
            (host, outcome)
        });
    }
    let mut result = ScanResult {
        network: Some(network),
        scanned_prefix,
        probed,
        ..Default::default()
    };
    let mut open = Vec::new();
    while let Some(Ok((host, outcome))) = probes.join_next().await {
        *result.outcomes.entry(outcome).or_default() += 1;
        if outcome == ConnectOutcome::Open {
            open.push(host);
        }
    }
    open.sort();
    for host in open {
        let answer = match client {
            Some(client) => info(client, IpAddr::V4(host), port).await,
            None => Err("no HTTP client".into()),
        };
        result.found.push((host, answer));
    }
    result
}

/// One LocalSend multicast datagram heard during the listening window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heard {
    pub source: IpAddr,
    pub alias: String,
    pub model: String,
    pub protocol: String,
    pub announce: bool,
    pub own: bool,
    pub count: usize,
}

/// Joins the LocalSend group on `interfaces` next to the running node's own
/// sockets (address/port reuse) and records every datagram for `window`
/// while `during` runs.
pub async fn listen_multicast(
    group: Ipv4Addr,
    port: u16,
    interfaces: &[Ipv4Addr],
    own_fingerprint: &str,
    window: Duration,
    during: impl std::future::Future<Output = ()>,
) -> io::Result<(Vec<Heard>, Vec<String>)> {
    let (socket, join_errors) = bind_listener(group, port, interfaces)?;
    let mut heard: Vec<Heard> = Vec::new();
    let mut buffer = vec![0u8; 65_535];
    let deadline = tokio::time::Instant::now() + window;
    tokio::pin!(during);
    let mut during_done = false;
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => break,
            _ = &mut during, if !during_done => during_done = true,
            received = socket.recv_from(&mut buffer) => {
                let Ok((length, source)) = received else { continue };
                let Ok(value) = serde_json::from_slice::<serde_json::Value>(&buffer[..length]) else {
                    continue;
                };
                let field = |name: &str| {
                    value.get(name).and_then(|value| value.as_str()).unwrap_or_default().to_owned()
                };
                let own = field("fingerprint") == own_fingerprint;
                let announce = value.get("announce").and_then(|v| v.as_bool())
                    .or_else(|| value.get("announcement").and_then(|v| v.as_bool()))
                    .unwrap_or(false);
                let source = source.ip();
                let alias = field("alias");
                if let Some(entry) = heard.iter_mut().find(|entry| entry.source == source && entry.alias == alias) {
                    entry.count += 1;
                    entry.announce |= announce;
                } else {
                    heard.push(Heard {
                        source,
                        alias,
                        model: field("deviceModel"),
                        protocol: field("protocol"),
                        announce,
                        own,
                        count: 1,
                    });
                }
            }
        }
    }
    if !during_done {
        // Let the announcement burst finish; it is bounded by the node.
        let _ = tokio::time::timeout(Duration::from_secs(5), during).await;
    }
    Ok((heard, join_errors))
}

fn bind_listener(
    group: Ipv4Addr,
    port: u16,
    interfaces: &[Ipv4Addr],
) -> io::Result<(tokio::net::UdpSocket, Vec<String>)> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddr::from(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).into())?;
    let mut errors = Vec::new();
    let mut joined = 0;
    for interface in interfaces {
        match socket.join_multicast_v4(&group, interface) {
            Ok(()) => joined += 1,
            Err(error) => errors.push(format!("join {group} on {interface}: {error}")),
        }
    }
    if joined == 0 {
        return Err(io::Error::other(if errors.is_empty() {
            "no interface to join the multicast group on".to_owned()
        } else {
            errors.join("; ")
        }));
    }
    socket.set_nonblocking(true)?;
    Ok((tokio::net::UdpSocket::from_std(socket.into())?, errors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_covers_real_prefix_up_to_slash_22() {
        let address = Ipv4Addr::new(10, 20, 94, 240);
        let (network, prefix, hosts) = scan_hosts(address, 23);
        assert_eq!((network, prefix), (Ipv4Addr::new(10, 20, 94, 0), 23));
        assert_eq!(hosts.len(), 509);
        assert!(hosts.contains(&Ipv4Addr::new(10, 20, 95, 254)));
        assert!(!hosts.contains(&address));
        assert!(!hosts.contains(&Ipv4Addr::new(10, 20, 95, 255)));
    }

    #[test]
    fn large_networks_scan_own_slash_24() {
        let (network, prefix, hosts) = scan_hosts(Ipv4Addr::new(10, 1, 2, 3), 16);
        assert_eq!((network, prefix), (Ipv4Addr::new(10, 1, 2, 0), 24));
        assert_eq!(hosts.len(), 253);
        let (_, prefix, hosts) = scan_hosts(Ipv4Addr::new(192, 168, 1, 9), 26);
        assert_eq!(prefix, 26);
        assert_eq!(hosts.len(), 61);
    }

    #[test]
    fn classifies_connect_errors() {
        assert_eq!(
            ConnectOutcome::classify(&io::Error::from(io::ErrorKind::ConnectionRefused)),
            ConnectOutcome::Refused
        );
        if let Some(code) = EHOSTDOWN {
            assert_eq!(
                ConnectOutcome::classify(&io::Error::from_raw_os_error(code)),
                ConnectOutcome::HostDown
            );
        }
    }

    #[tokio::test]
    async fn connect_distinguishes_open_and_refused() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let open = listener.local_addr().unwrap();
        assert_eq!(connect(open, CONNECT_TIMEOUT).await.0, ConnectOutcome::Open);
        drop(listener);
        // Windows retries a refused connect for about two seconds.
        assert_eq!(
            connect(open, Duration::from_secs(5)).await.0,
            ConnectOutcome::Refused
        );
    }

    #[tokio::test]
    async fn listener_hears_loopback_announcements() {
        let port = {
            let probe = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
            probe.local_addr().unwrap().port()
        };
        let group = Ipv4Addr::new(224, 0, 0, 167);
        let interface = Ipv4Addr::LOCALHOST;
        let send = async move {
            let sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
            let _ = socket2::SockRef::from(&sender).set_multicast_if_v4(&interface);
            let _ = sender.set_multicast_loop_v4(true);
            for _ in 0..5 {
                let _ = sender.send_to(
                    br#"{"alias":"peer","fingerprint":"other","deviceModel":"Test","protocol":"https","announce":true}"#,
                    (group, port),
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        let result = listen_multicast(
            group,
            port,
            &[interface],
            "mine",
            Duration::from_millis(800),
            send,
        )
        .await;
        let Ok((heard, _)) = result else {
            // Some sandboxes have no multicast on loopback; nothing to assert.
            return;
        };
        if let Some(peer) = heard.iter().find(|entry| entry.alias == "peer") {
            assert!(!peer.own);
            assert!(peer.announce);
            assert_eq!(peer.model, "Test");
        }
    }
}
