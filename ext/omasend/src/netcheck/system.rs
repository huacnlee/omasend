//! Facts from the operating system: interfaces and non-privileged commands.
use super::redact;
use std::{net::Ipv4Addr, time::Duration};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_OUTPUT_LINES: usize = 60;

/// The result of one external command, already redacted and truncated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutput {
    pub label: String,
    pub text: String,
}

/// Runs a standard tool with a timeout and returns its combined output, or a
/// line saying why it could not run. Never prompts or elevates.
pub async fn run(program: &str, args: &[&str]) -> CommandOutput {
    let label = std::iter::once(program)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let text = if cfg!(any(target_os = "ios", target_os = "android")) {
        "(commands are unavailable on this platform)".to_owned()
    } else {
        run_inner(program, args).await
    };
    CommandOutput {
        label,
        text: truncate(&redact::text(&text)),
    }
}

async fn run_inner(program: &str, args: &[&str]) -> String {
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .env("LC_ALL", "C");
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: no console flashes from a GUI process.
        command.creation_flags(0x0800_0000);
    }
    match tokio::time::timeout(COMMAND_TIMEOUT, command.output()).await {
        Err(_) => format!("(timed out after {} s)", COMMAND_TIMEOUT.as_secs()),
        Ok(Err(error)) => format!("(could not run: {error})"),
        Ok(Ok(output)) => {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&stderr);
            }
            if !output.status.success() {
                match output.status.code() {
                    Some(code) => text.push_str(&format!("\n(exit code {code})")),
                    None => text.push_str("\n(terminated by a signal)"),
                }
            }
            text
        }
    }
}

fn truncate(text: &str) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    if lines.len() <= MAX_OUTPUT_LINES {
        return lines.join("\n");
    }
    let mut kept = lines[..MAX_OUTPUT_LINES].join("\n");
    kept.push_str(&format!(
        "\n… ({} more lines omitted)",
        lines.len() - MAX_OUTPUT_LINES
    ));
    kept
}

/// One IPv4 address of a non-loopback interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceV4 {
    pub name: String,
    pub address: Ipv4Addr,
    pub prefix: u32,
    pub up: bool,
    pub point_to_point: bool,
}

impl InterfaceV4 {
    pub fn is_virtual(&self) -> bool {
        self.point_to_point || is_virtual(&self.name, self.address)
    }

    /// Whether discovery traffic is expected on this interface: a private,
    /// up, physical-looking network.
    pub fn is_lan(&self) -> bool {
        self.up && !self.is_virtual() && self.address.is_private()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Interfaces {
    pub v4: Vec<InterfaceV4>,
    /// Names of interfaces with at least one IPv6 address.
    pub with_v6: Vec<String>,
}

pub fn interfaces() -> std::io::Result<Interfaces> {
    let mut result = Interfaces::default();
    for interface in if_addrs::get_if_addrs()? {
        if interface.is_loopback() {
            continue;
        }
        match &interface.addr {
            if_addrs::IfAddr::V4(address) => result.v4.push(InterfaceV4 {
                name: interface.name.clone(),
                address: address.ip,
                prefix: u32::from(address.netmask).leading_ones(),
                up: interface.is_oper_up(),
                point_to_point: interface.is_p2p,
            }),
            if_addrs::IfAddr::V6(_) => {
                if !result.with_v6.contains(&interface.name) {
                    result.with_v6.push(interface.name.clone());
                }
            }
        }
    }
    Ok(result)
}

/// Tunnels, bridges, container and VM adapters, and the address ranges proxy
/// TUN modes (198.18.0.0/15) and overlay VPNs (100.64.0.0/10) use.
pub fn is_virtual(name: &str, address: Ipv4Addr) -> bool {
    const PREFIXES: [&str; 17] = [
        "utun",
        "tun",
        "tap",
        "bridge",
        "vethernet",
        "docker",
        "veth",
        "vmnet",
        "vboxnet",
        "virbr",
        "wg",
        "zt",
        "tailscale",
        "llw",
        "awdl",
        "anpi",
        "br-",
    ];
    let lower = name.to_ascii_lowercase();
    let [a, b, ..] = address.octets();
    PREFIXES.iter().any(|prefix| lower.starts_with(prefix))
        || lower.contains("vpn")
        || lower.contains("virtual")
        || lower.contains("hyper-v")
        || (a == 198 && (b == 18 || b == 19))
        || (a == 100 && (64..128).contains(&b))
}

/// Whether a MAC address is locally administered, i.e. a private or
/// randomized address rather than the hardware one.
pub fn locally_administered(mac: &str) -> Option<bool> {
    let first = mac.split([':', '-']).next()?;
    let octet = u8::from_str_radix(first, 16).ok()?;
    (mac.split([':', '-']).count() == 6).then_some(octet & 0x02 != 0)
}

/// This device's MAC address of `interface`, where it can be read without
/// privileges. It is used only to derive [`locally_administered`].
pub async fn own_mac(interface: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = run("ifconfig", &[interface]).await;
        return output.text.lines().find_map(|line| {
            line.trim()
                .strip_prefix("ether ")
                .map(|mac| mac.trim().to_owned())
        });
    }
    #[cfg(target_os = "linux")]
    {
        return std::fs::read_to_string(format!("/sys/class/net/{interface}/address"))
            .ok()
            .map(|mac| mac.trim().to_owned());
    }
    #[allow(unreachable_code)]
    {
        let _ = interface;
        None
    }
}

pub fn os_name() -> &'static str {
    std::env::consts::OS
}

pub async fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        return run("sw_vers", &["-productVersion"]).await.text;
    }
    #[cfg(target_os = "linux")]
    {
        return std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|text| {
                text.lines()
                    .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                    .map(|name| name.trim_matches('"').to_owned())
            })
            .unwrap_or_else(|| "unknown".into());
    }
    #[cfg(windows)]
    {
        return run("cmd", &["/C", "ver"]).await.text.trim().to_owned();
    }
    #[allow(unreachable_code)]
    "unknown".into()
}

/// Default routes and the route the OS picks for the multicast group.
pub async fn routing() -> Vec<CommandOutput> {
    let group = "224.0.0.167";
    if cfg!(target_os = "macos") {
        vec![
            default_routes(run("netstat", &["-rn", "-f", "inet"]).await),
            run("route", &["-n", "get", group]).await,
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            run("ip", &["-4", "route", "show", "default"]).await,
            run("ip", &["-4", "route", "get", group]).await,
        ]
    } else if cfg!(windows) {
        vec![run("route", &["print", "-4", "0.0.0.0"]).await]
    } else {
        Vec::new()
    }
}

/// Keeps the header and `default` lines of a BSD routing table.
fn default_routes(mut output: CommandOutput) -> CommandOutput {
    output.text = output
        .text
        .lines()
        .filter(|line| {
            line.starts_with("default") || line.starts_with("Destination") || line.starts_with('(')
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.label.push_str("  (default routes only)");
    output
}

pub async fn route_to(address: Ipv4Addr) -> Option<CommandOutput> {
    let ip = address.to_string();
    if cfg!(target_os = "macos") {
        Some(run("route", &["-n", "get", &ip]).await)
    } else if cfg!(target_os = "linux") {
        Some(run("ip", &["-4", "route", "get", &ip]).await)
    } else if cfg!(windows) {
        Some(
            run(
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Find-NetRoute -RemoteIPAddress {ip} | Select-Object -First 2 InterfaceAlias,IPAddress,NextHop | Format-List"
                    ),
                ],
            )
            .await,
        )
    } else {
        None
    }
}

pub async fn neighbor(address: Ipv4Addr) -> Option<CommandOutput> {
    let ip = address.to_string();
    if cfg!(target_os = "macos") {
        Some(run("arp", &["-n", &ip]).await)
    } else if cfg!(target_os = "linux") {
        Some(run("ip", &["neigh", "show", &ip]).await)
    } else if cfg!(windows) {
        Some(run("arp", &["-a", &ip]).await)
    } else {
        None
    }
}

pub async fn ping(address: Ipv4Addr) -> Option<CommandOutput> {
    let ip = address.to_string();
    if cfg!(target_os = "macos") {
        Some(run("ping", &["-c", "2", "-t", "2", &ip]).await)
    } else if cfg!(target_os = "linux") {
        Some(run("ping", &["-c", "2", "-W", "1", &ip]).await)
    } else if cfg!(windows) {
        Some(run("ping", &["-n", "2", "-w", "1000", &ip]).await)
    } else {
        None
    }
}

/// Proxy environment variables, names upper- and lower-case, redacted.
pub fn proxy_environment() -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = std::env::vars()
        .filter(|(key, _)| {
            let key = key.to_ascii_lowercase();
            key.ends_with("_proxy") || key == "proxy"
        })
        .map(|(key, value)| (key, redact::url_credentials(&value)))
        .collect();
    found.sort();
    found
}

pub async fn proxy_and_vpn() -> Vec<CommandOutput> {
    if cfg!(target_os = "macos") {
        vec![
            run("scutil", &["--proxy"]).await,
            run("scutil", &["--nc", "list"]).await,
        ]
    } else if cfg!(target_os = "linux") {
        vec![run("gsettings", &["get", "org.gnome.system.proxy", "mode"]).await]
    } else if cfg!(windows) {
        vec![
            run("netsh", &["winhttp", "show", "proxy"]).await,
            run(
                "reg",
                &[
                    "query",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    "/v",
                    "Proxy*",
                ],
            )
            .await,
        ]
    } else {
        Vec::new()
    }
}

/// Firewall state and, on Windows, the network category of each connection.
pub async fn firewall() -> Vec<CommandOutput> {
    if cfg!(target_os = "macos") {
        let tool = "/usr/libexec/ApplicationFirewall/socketfilterfw";
        let mut outputs = vec![
            run(
                tool,
                &["--getglobalstate", "--getblockall", "--getstealthmode"],
            )
            .await,
        ];
        if let Ok(exe) = std::env::current_exe() {
            outputs.push(run(tool, &["--getappblocked", &exe.to_string_lossy()]).await);
        }
        outputs
    } else if cfg!(target_os = "linux") {
        vec![
            run("ufw", &["status"]).await,
            run("firewall-cmd", &["--state"]).await,
        ]
    } else if cfg!(windows) {
        vec![
            run("netsh", &["advfirewall", "show", "allprofiles", "state"]).await,
            run(
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    // Name is the network's (often SSID) name; leave it out.
                    "Get-NetConnectionProfile | Select-Object InterfaceAlias,NetworkCategory,IPv4Connectivity | Format-Table -AutoSize | Out-String -Width 200",
                ],
            )
            .await,
            run(
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    "Get-NetFirewallRule -DisplayName '*omasend*','*localsend*' -ErrorAction SilentlyContinue | Select-Object DisplayName,Enabled,Direction,Action,Profile | Format-Table -AutoSize | Out-String -Width 200",
                ],
            )
            .await,
        ]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_virtual_interfaces() {
        let private = Ipv4Addr::new(192, 168, 1, 20);
        assert!(is_virtual("utun5", Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_virtual("bridge100", private));
        assert!(is_virtual("vEthernet (WSL)", private));
        assert!(is_virtual("docker0", private));
        assert!(is_virtual("en7", Ipv4Addr::new(198, 18, 0, 1)));
        assert!(is_virtual("tailscale0", Ipv4Addr::new(100, 101, 1, 2)));
        assert!(!is_virtual("en0", Ipv4Addr::new(10, 20, 94, 240)));
        assert!(!is_virtual("wlan0", private));
        assert!(!is_virtual("Wi-Fi", private));
    }

    #[test]
    fn lan_needs_up_private_physical() {
        let lan = InterfaceV4 {
            name: "en0".into(),
            address: Ipv4Addr::new(10, 20, 94, 240),
            prefix: 23,
            up: true,
            point_to_point: false,
        };
        assert!(lan.is_lan());
        assert!(
            !InterfaceV4 {
                up: false,
                ..lan.clone()
            }
            .is_lan()
        );
        assert!(
            !InterfaceV4 {
                address: Ipv4Addr::new(8, 8, 8, 8),
                ..lan.clone()
            }
            .is_lan()
        );
        assert!(
            !InterfaceV4 {
                point_to_point: true,
                ..lan
            }
            .is_lan()
        );
    }

    #[test]
    fn detects_locally_administered_macs() {
        assert_eq!(locally_administered("66:da:28:9d:fa:49"), Some(true));
        assert_eq!(locally_administered("10:9f:41:d9:2e:60"), Some(false));
        assert_eq!(locally_administered("10-9F-41-D9-2E-60"), Some(false));
        assert_eq!(locally_administered("nonsense"), None);
    }

    #[test]
    fn keeps_only_default_routes() {
        let output = default_routes(CommandOutput {
            label: "netstat -rn".into(),
            text: "Routing tables\n\nInternet:\nDestination Gateway\ndefault 10.20.95.1 UGScg en0\n10.20.94/23 link#11 UCS en0\ndefault link#26 UCSIg bridge100 !".into(),
        });
        assert_eq!(
            output.text,
            "Destination Gateway\ndefault 10.20.95.1 UGScg en0\ndefault link#26 UCSIg bridge100 !"
        );
    }

    #[test]
    fn truncates_long_output() {
        let text = (0..100)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let kept = truncate(&text);
        assert!(kept.ends_with("… (40 more lines omitted)"));
        assert_eq!(kept.lines().count(), MAX_OUTPUT_LINES + 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reports_missing_commands_instead_of_failing() {
        let output = run("omasend-no-such-tool", &["--x"]).await;
        assert!(
            output.text.starts_with("(could not run:"),
            "{}",
            output.text
        );
        assert_eq!(output.label, "omasend-no-such-tool --x");
    }
}
