use super::{
    Device, Events, OfferedFile, TransferEvent, Upload, emit,
    receive::{ReceiveFile, relative_path},
    send::SendTransfer,
};
use anyhow::{Context, Result, ensure};
use localsend::{
    crypto::cert::{SelfSignedCert, generate_self_signed},
    discovery::{
        self, DeviceChannel, DeviceIdentity, DiscoveredDevice, DiscoveryConfig, DiscoveryEvent,
        DiscoveryHandle, HttpChannel,
    },
    http::{
        dto_v2::RegisterDtoV2,
        server::{
            self, ServerConfigV2, ServerHandle, TlsConfig,
            common::save::FileUploadTarget,
            v2::{PrepareUploadDecisionV2, ServerEventV2, SessionEndReasonV2},
            web::WebConfig,
        },
        state::ClientInfo,
    },
    model::{
        discovery::{DeviceType, PROTOCOL_VERSION_V2, ProtocolType},
        transfer::FileDto,
    },
    multicast::{
        DEFAULT_MULTICAST_GROUP, DEFAULT_MULTICAST_GROUP_V6, DEFAULT_PORT, MulticastDevice,
    },
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    sync::{mpsc, oneshot},
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;

pub struct NodeConfig {
    pub alias: String,
    pub port: u16,
    pub downloads: PathBuf,
    pub discovery: bool,
}

impl NodeConfig {
    pub fn desktop() -> Result<Self> {
        Ok(Self {
            alias: machine_name(),
            port: DEFAULT_PORT,
            downloads: super::downloads_directory()?,
            discovery: true,
        })
    }
}

enum Command {
    Send {
        id: String,
        peer: Device,
        uploads: Vec<Upload>,
        pin: Option<String>,
    },
    Decide {
        id: String,
        accept: bool,
    },
    Cancel(String),
    Stop,
}

#[derive(Clone)]
pub struct Handle {
    commands: mpsc::UnboundedSender<Command>,
}

impl Handle {
    pub fn send(&self, peer: Device, uploads: Vec<Upload>, pin: Option<String>) -> Result<String> {
        ensure!(!uploads.is_empty(), "Add something to send first");
        let id = uuid::Uuid::new_v4().to_string();
        self.commands
            .send(Command::Send {
                id: id.clone(),
                peer,
                uploads,
                pin,
            })
            .map_err(|_| anyhow::anyhow!("Network service stopped"))?;
        Ok(id)
    }
    pub fn decide(&self, id: &str, accept: bool) -> Result<()> {
        self.commands
            .send(Command::Decide {
                id: id.into(),
                accept,
            })
            .map_err(|_| anyhow::anyhow!("Network service stopped"))
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        self.commands
            .send(Command::Cancel(id.into()))
            .map_err(|_| anyhow::anyhow!("Network service stopped"))
    }
}

pub struct Node {
    pub device: Device,
    pub handle: Handle,
    pub events: async_channel::Receiver<TransferEvent>,
    task: Option<JoinHandle<()>>,
}

impl Node {
    pub async fn start(config: NodeConfig) -> Result<Self> {
        tokio::fs::create_dir_all(&config.downloads)
            .await
            .context("Cannot create Downloads folder")?;
        let downloads = tokio::fs::canonicalize(&config.downloads).await?;
        let identity = Arc::new(tokio::task::spawn_blocking(generate_self_signed).await??);
        let (events, event_rx) = async_channel::unbounded();
        let (server_tx, server_rx) = mpsc::channel(128);
        let (stop_tx, stop_rx) = oneshot::channel();
        let info = ClientInfo {
            alias: config.alias.clone(),
            version: PROTOCOL_VERSION_V2.into(),
            device_model: Some("OmaSend".into()),
            device_type: Some(DeviceType::Desktop),
            token: identity.fingerprint.clone(),
        };
        let server = server::start_with_port(
            config.port,
            Some(TlsConfig {
                cert: identity.certificate_pem.clone(),
                private_key: identity.private_key_pem.clone(),
            }),
            info,
            None,
            Some(ServerConfigV2 {
                pin: None,
                verify_checksums: true,
                event_tx: server_tx,
            }),
            WebConfig::default(),
            stop_rx,
        )
        .await
        .context("Cannot listen for LocalSend transfers; another app may be using port 53317")?;
        let device = Device {
            fingerprint: identity.fingerprint.clone(),
            alias: config.alias,
            model: "OmaSend".into(),
            host: "127.0.0.1".into(),
            port: server.port(),
        };
        let register = RegisterDtoV2 {
            alias: device.alias.clone(),
            version: PROTOCOL_VERSION_V2.into(),
            device_model: Some(device.model.clone()),
            device_type: Some(DeviceType::Desktop),
            fingerprint: device.fingerprint.clone(),
            port: device.port,
            protocol: ProtocolType::Https,
            download: false,
        };
        let (commands, command_rx) = mpsc::unbounded_channel();
        let handle = Handle { commands };
        let task = tokio::spawn(async move {
            Actor {
                identity,
                register,
                server,
                downloads,
                events,
                incoming: None,
                outgoing: HashMap::new(),
            }
            .run(command_rx, server_rx, stop_tx, config.discovery)
            .await;
        });
        Ok(Self {
            device,
            handle,
            events: event_rx,
            task: Some(task),
        })
    }

    pub async fn shutdown(mut self) {
        let _ = self.handle.commands.send(Command::Stop);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.handle.commands.send(Command::Stop);
    }
}

struct Incoming {
    id: String,
    peer: Device,
    files: HashMap<String, FileDto>,
    decision: Option<oneshot::Sender<PrepareUploadDecisionV2>>,
    cancel: CancellationToken,
    progress: HashMap<String, u64>,
    paths: Vec<PathBuf>,
    saved: usize,
    last_activity: Instant,
}

impl Incoming {
    fn total(&self) -> u64 {
        self.files.values().map(|file| file.size).sum()
    }
}

struct Outgoing {
    cancel: CancellationToken,
    host: String,
    remote: Arc<Mutex<Option<String>>>,
}

enum Finished {
    Send { id: String, result: Result<()> },
    Receive { id: String, result: Result<PathBuf> },
}

struct Actor {
    identity: Arc<SelfSignedCert>,
    register: RegisterDtoV2,
    server: ServerHandle,
    downloads: PathBuf,
    events: Events,
    incoming: Option<Incoming>,
    outgoing: HashMap<String, Outgoing>,
}

impl Actor {
    async fn run(
        mut self,
        mut commands: mpsc::UnboundedReceiver<Command>,
        mut server_events: mpsc::Receiver<ServerEventV2>,
        stop_server: oneshot::Sender<()>,
        discover: bool,
    ) {
        let mut tasks = JoinSet::new();
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
        let (discovery, stop_discovery, discovery_task) = if discover {
            let (stop_tx, stop_rx) = oneshot::channel();
            let (discovery_tx, discovery_rx) = mpsc::channel(256);
            let handle = Arc::new(
                discovery::start(
                    DiscoveryConfig {
                        group: DEFAULT_MULTICAST_GROUP,
                        group_v6: Some(DEFAULT_MULTICAST_GROUP_V6),
                        port: DEFAULT_PORT,
                        interface_filter: Default::default(),
                        device: MulticastDevice {
                            alias: self.register.alias.clone(),
                            version: self.register.version.clone(),
                            device_model: self.register.device_model.clone(),
                            device_type: self.register.device_type.clone(),
                            fingerprint: self.register.fingerprint.clone(),
                            port: self.register.port,
                            protocol: ProtocolType::Https,
                            download: false,
                        },
                        identity: DeviceIdentity {
                            cert_pem: self.identity.certificate_pem.clone(),
                            private_key_pem: self.identity.private_key_pem.clone(),
                        },
                        timeout: Duration::from_secs(2),
                        event_tx: Some(discovery_tx),
                    },
                    stop_rx,
                )
                .await,
            );
            if let Some(error) = handle.multicast_error() {
                emit(
                    &self.events,
                    TransferEvent::NetworkError(format!(
                        "Multicast unavailable; trying local network discovery: {error}"
                    )),
                );
            }
            let task = tokio::spawn(publish_while_discovering(
                handle.clone(),
                self.events.clone(),
                discovery_rx,
                discover_devices(handle.clone(), self.events.clone()),
            ));
            (Some(handle), Some(stop_tx), Some(task))
        } else {
            (None, None, None)
        };
        let mut timer = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                command = commands.recv() => match command {
                    Some(Command::Stop) | None => break,
                    Some(Command::Send { id, peer, uploads, pin }) => {
                        let total = uploads.iter().try_fold(0u64, |sum, upload| sum.checked_add(upload.offer.size));
                        let Some(total) = total else { emit(&self.events, TransferEvent::Failed { id, error: "Transfer is too large".into() }); continue };
                        let cancel = CancellationToken::new();
                        let remote = Arc::new(Mutex::new(None));
                        self.outgoing.insert(id.clone(), Outgoing { cancel: cancel.clone(), host: peer.host.clone(), remote: remote.clone() });
                        emit(&self.events, TransferEvent::Started { id: id.clone(), peer: peer.alias.clone(), sending: true, files: uploads.iter().map(|upload| upload.offer.clone()).collect(), total });
                        let send = SendTransfer { id: id.clone(), peer, identity: self.identity.clone(), info: self.register.clone(), uploads, pin, cancel, remote_session: remote, events: self.events.clone() };
                        tasks.spawn(async move { Finished::Send { id, result: send.run().await } });
                    }
                    Some(Command::Decide { id, accept }) => self.decide(&id, accept),
                    Some(Command::Cancel(id)) => self.cancel(&id).await,
                },
                event = server_events.recv() => {
                    let Some(event) = event else { break };
                    self.server_event(event, discovery.as_deref(), &mut tasks, &progress_tx).await;
                }
                Some((id, file, bytes)) = progress_rx.recv() => {
                    if let Some(incoming) = self.incoming.as_mut().filter(|incoming| incoming.id == id) {
                        incoming.last_activity = Instant::now();
                        incoming.progress.insert(file, bytes);
                        emit(&self.events, TransferEvent::Progress { id, transferred: incoming.progress.values().sum(), total: incoming.total() });
                    }
                }
                Some(result) = tasks.join_next(), if !tasks.is_empty() => match result {
                    Ok(result) => self.finished(result).await,
                    Err(error) => emit(&self.events, TransferEvent::NetworkError(format!("Transfer task stopped: {error}"))),
                },
                _ = timer.tick() => {
                    if let Some(incoming) = &self.incoming {
                        let timeout = if incoming.decision.is_some() { 300 } else { 90 };
                        if incoming.last_activity.elapsed() > Duration::from_secs(timeout) {
                            let id = incoming.id.clone();
                            self.cancel(&id).await;
                        }
                    }
                }
            }
        }
        if let Some(incoming) = &self.incoming {
            incoming.cancel.cancel();
        }
        for outgoing in self.outgoing.values() {
            outgoing.cancel.cancel();
        }
        if let Some(stop) = stop_discovery {
            let _ = stop.send(());
        }
        if let Some(task) = discovery_task {
            task.abort();
            let _ = task.await;
        }
        if let Some(discovery) = discovery {
            discovery.wait_stopped().await;
        }
        let _ = stop_server.send(());
        self.server.wait_stopped().await;
        // Cancelling stream consumers drops their receivers and temporary files.
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }

    fn decide(&mut self, id: &str, accept: bool) {
        let Some(incoming) = self.incoming.as_mut().filter(|incoming| incoming.id == id) else {
            return;
        };
        let Some(decision) = incoming.decision.take() else {
            return;
        };
        if accept {
            let total = incoming.total();
            let files = incoming.files.values().map(offer).collect();
            if decision
                .send(PrepareUploadDecisionV2::Accept(
                    incoming.files.keys().cloned().collect(),
                ))
                .is_ok()
            {
                incoming.last_activity = Instant::now();
                emit(
                    &self.events,
                    TransferEvent::Started {
                        id: id.into(),
                        peer: incoming.peer.alias.clone(),
                        sending: false,
                        files,
                        total,
                    },
                );
                return;
            }
        } else {
            let _ = decision.send(PrepareUploadDecisionV2::Decline);
        }
        self.incoming = None;
        emit(&self.events, TransferEvent::Cancelled { id: id.into() });
    }

    async fn cancel(&mut self, id: &str) {
        if let Some(outgoing) = self.outgoing.get(id) {
            outgoing.cancel.cancel();
        }
        if self
            .incoming
            .as_ref()
            .is_some_and(|incoming| incoming.id == id)
        {
            let incoming = self.incoming.take().unwrap();
            incoming.cancel.cancel();
            if let Some(decision) = incoming.decision {
                let _ = decision.send(PrepareUploadDecisionV2::Decline);
            }
            self.server.cancel_v2_session(id).await;
            emit(&self.events, TransferEvent::Cancelled { id: id.into() });
        }
    }

    async fn server_event(
        &mut self,
        event: ServerEventV2,
        discovery: Option<&DiscoveryHandle>,
        tasks: &mut JoinSet<Finished>,
        progress_tx: &mpsc::UnboundedSender<(String, String, u64)>,
    ) {
        match event {
            ServerEventV2::Register { ip, info } => {
                tracing::debug!(%ip, alias = %info.alias, protocol = ?info.protocol, "Registered peer");
                if info.protocol != ProtocolType::Https {
                    return;
                }
                if let Some(discovery) = discovery {
                    discovery
                        .add_device(DiscoveredDevice {
                            alias: info.alias,
                            version: info.version,
                            device_model: info.device_model,
                            device_type: info.device_type,
                            fingerprint: info.fingerprint,
                            channel: DeviceChannel::Http(HttpChannel {
                                host: ip.to_string(),
                                port: info.port,
                                protocol: info.protocol,
                            }),
                            download: info.download,
                        })
                        .await;
                }
            }
            ServerEventV2::PrepareUpload {
                session_id,
                ip,
                info,
                cert_fingerprint,
                files,
                decision_tx,
            } => {
                tracing::debug!(id = %session_id, "Received transfer request");
                if self.incoming.is_some() {
                    let _ = decision_tx.send(PrepareUploadDecisionV2::Decline);
                    return;
                }
                let valid = validate_offer(&files).and_then(|_| {
                    ensure!(
                        cert_fingerprint.is_some(),
                        "An encrypted connection is required"
                    );
                    Ok(())
                });
                if let Err(error) = valid {
                    let _ = decision_tx.send(PrepareUploadDecisionV2::Decline);
                    emit(
                        &self.events,
                        TransferEvent::Failed {
                            id: session_id,
                            error: error.to_string(),
                        },
                    );
                    return;
                }
                let peer = Device {
                    fingerprint: cert_fingerprint.unwrap(),
                    alias: info.alias,
                    model: info.device_model.unwrap_or_default(),
                    host: ip.to_string(),
                    port: info.port,
                };
                let mut offered: Vec<_> = files.values().map(offer).collect();
                offered.sort_by(|a, b| a.name.cmp(&b.name));
                emit(
                    &self.events,
                    TransferEvent::IncomingRequest {
                        id: session_id.clone(),
                        peer: peer.clone(),
                        files: offered,
                    },
                );
                self.incoming = Some(Incoming {
                    id: session_id,
                    peer,
                    files,
                    decision: Some(decision_tx),
                    cancel: CancellationToken::new(),
                    progress: HashMap::new(),
                    paths: Vec::new(),
                    saved: 0,
                    last_activity: Instant::now(),
                });
            }
            ServerEventV2::FileUpload {
                session_id,
                file_id,
                file,
                target_tx,
            } => {
                let Some(incoming) = self
                    .incoming
                    .as_mut()
                    .filter(|incoming| incoming.id == session_id && incoming.decision.is_none())
                else {
                    return;
                };
                if !incoming.files.contains_key(&file_id) {
                    return;
                }
                incoming.last_activity = Instant::now();
                let (binary_tx, binary_rx) = mpsc::channel(16);
                let (result_tx, result_rx) = oneshot::channel();
                if target_tx
                    .send(FileUploadTarget::Stream {
                        binary_tx,
                        result_rx,
                    })
                    .is_err()
                {
                    return;
                }
                let receive = ReceiveFile {
                    downloads: self.downloads.clone(),
                    session: session_id.clone(),
                    file,
                    cancel: incoming.cancel.clone(),
                    events: self.events.clone(),
                    progress_tx: progress_tx.clone(),
                };
                tasks.spawn(async move {
                    let result = receive.save(binary_rx).await;
                    let _ = result_tx.send(
                        result
                            .as_ref()
                            .map(|_| ())
                            .map_err(|error| format!("{error:#}")),
                    );
                    Finished::Receive {
                        id: session_id,
                        result,
                    }
                });
            }
            ServerEventV2::SessionEnd {
                session_id,
                reason: SessionEndReasonV2::Cancelled,
            }
            | ServerEventV2::PrepareUploadAborted { session_id } => self.cancel(&session_id).await,
            ServerEventV2::SessionEnd { .. } => {} // File outcomes, not this event, determine success.
            ServerEventV2::CancelReceived { ip, session_id } => {
                for outgoing in self.outgoing.values() {
                    if outgoing.host == ip.to_string()
                        && outgoing.remote.lock().unwrap().as_deref() == Some(&session_id)
                    {
                        outgoing.cancel.cancel();
                    }
                }
            }
            ServerEventV2::ListenerFailed { error } => emit(
                &self.events,
                TransferEvent::NetworkError(format!(
                    "Receiving stopped: {error}. Reopen OmaSend to reconnect."
                )),
            ),
        }
    }

    async fn finished(&mut self, finished: Finished) {
        match finished {
            Finished::Send { id, result } => {
                let cancelled = self
                    .outgoing
                    .remove(&id)
                    .is_some_and(|outgoing| outgoing.cancel.is_cancelled());
                emit(
                    &self.events,
                    if cancelled {
                        TransferEvent::Cancelled { id }
                    } else {
                        match result {
                            Ok(()) => TransferEvent::Completed {
                                id,
                                paths: Vec::new(),
                            },
                            Err(error) => TransferEvent::Failed {
                                id,
                                error: format!("{error:#}"),
                            },
                        }
                    },
                );
            }
            Finished::Receive { id, result } => {
                let Some(incoming) = self.incoming.as_mut().filter(|incoming| incoming.id == id)
                else {
                    return;
                };
                match result {
                    Ok(path) => {
                        incoming.paths.push(path);
                        incoming.saved += 1;
                        if incoming.saved == incoming.files.len() {
                            let incoming = self.incoming.take().unwrap();
                            emit(
                                &self.events,
                                TransferEvent::Completed {
                                    id,
                                    paths: incoming.paths,
                                },
                            );
                        }
                    }
                    Err(error) => {
                        incoming.cancel.cancel();
                        self.incoming = None;
                        self.server.cancel_v2_session(&id).await;
                        emit(
                            &self.events,
                            TransferEvent::Failed {
                                id,
                                error: format!("{error:#}"),
                            },
                        );
                    }
                }
            }
        }
    }
}

fn offer(file: &FileDto) -> OfferedFile {
    OfferedFile {
        name: file.file_name.clone(),
        size: file.size,
        mime: file.file_type.clone(),
    }
}

fn validate_offer(files: &HashMap<String, FileDto>) -> Result<()> {
    ensure!(
        !files.is_empty() && files.len() <= 10_000,
        "Offer must contain between 1 and 10,000 files"
    );
    let mut total = 0u64;
    for (id, file) in files {
        ensure!(*id == file.id, "File ID does not match its metadata");
        relative_path(&file.file_name)?;
        total = total
            .checked_add(file.size)
            .context("Transfer is too large")?;
    }
    Ok(())
}

async fn publish_while_discovering(
    discovery: Arc<DiscoveryHandle>,
    events: Events,
    mut updates: mpsc::Receiver<DiscoveryEvent>,
    work: impl std::future::Future<Output = ()>,
) {
    // Announcements and HTTP probes may take seconds. Poll them alongside
    // confirmations instead of making the GUI wait for a whole network pass.
    tokio::pin!(work);
    let mut refresh = tokio::time::interval(Duration::from_secs(1));
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut visible = HashMap::new();
    let mut failures: HashMap<String, Instant> = HashMap::new();
    let mut updates_open = true;
    loop {
        tokio::select! {
            _ = &mut work => break,
            update = updates.recv(), if updates_open => {
                match update {
                    Some(DiscoveryEvent::Discovered { .. } | DiscoveryEvent::Updated { .. }) => {}
                    Some(DiscoveryEvent::ProbeFailed { host, alias, error }) => {
                        let lower = error.to_ascii_lowercase();
                        if !lower.contains("certificate") && !lower.contains("fingerprint") {
                            // Background peers routinely disappear or time out.
                            // The discovery core logs these; they are not UI errors.
                            continue;
                        }
                        let now = Instant::now();
                        failures.retain(|_, last| now.duration_since(*last) < Duration::from_secs(60));
                        if !failures.contains_key(&host) {
                            failures.insert(host.clone(), now);
                            let message = "Nearby device identity could not be verified";
                            emit(&events, TransferEvent::NetworkError(format!(
                                "{message}: {alias} ({host})"
                            )));
                        }
                    }
                    Some(DiscoveryEvent::MulticastFailed) => {
                        emit(&events, TransferEvent::NetworkError(
                            "Multicast unavailable; trying local network discovery: multicast sockets stopped".into()
                        ));
                    }
                    None => updates_open = false,
                }
            }
            _ = refresh.tick() => {}
        }
        // Events are bounded upstream and may be dropped in a burst. Read the
        // authoritative store on both events and a timer; the timer also owns
        // expiry when a slow network operation never produces a confirmation.
        publish_discovery_snapshot(&discovery, &events, &mut visible, SystemTime::now());
    }
}

fn publish_discovery_snapshot(
    discovery: &DiscoveryHandle,
    events: &Events,
    visible: &mut HashMap<String, Device>,
    now: SystemTime,
) {
    let mut next = HashMap::new();
    for state in discovery.devices() {
        let Some(last) = state.logs.last() else {
            continue;
        };
        if now.duration_since(last.timestamp).unwrap_or_default() > Duration::from_secs(15) {
            continue;
        }
        let Some(channel) = state.device.http() else {
            continue;
        };
        if channel.protocol != ProtocolType::Https {
            continue;
        }
        let device = Device {
            fingerprint: state.device.fingerprint.clone(),
            alias: state.device.alias.clone(),
            model: state.device.device_model.clone().unwrap_or_default(),
            host: channel.host.clone(),
            port: channel.port,
        };
        if visible.get(&device.fingerprint).is_none_or(|known| {
            known.alias != device.alias
                || known.model != device.model
                || known.host != device.host
                || known.port != device.port
        }) {
            emit(events, TransferEvent::DeviceFound(device.clone()));
            tracing::debug!(alias = %device.alias, host = %device.host, "Publishing nearby peer");
        }
        next.insert(device.fingerprint.clone(), device);
    }
    for id in visible.keys().filter(|id| !next.contains_key(*id)) {
        emit(events, TransferEvent::DeviceLost(id.clone()));
    }
    *visible = next;
}

async fn discover_devices(discovery: Arc<DiscoveryHandle>, events: Events) {
    // Keep fallback scans separate from publishing the live device list. A
    // slow/unreachable subnet must not delay incoming peers or expiry updates.
    // JoinSet aborts outstanding probes when this discovery task is stopped.
    let mut scans = JoinSet::new();
    let mut next_scan = Instant::now();
    loop {
        while scans.try_join_next().is_some() {}
        if Instant::now() >= next_scan && scans.is_empty() {
            next_scan = Instant::now() + Duration::from_secs(60);
            match if_addrs::get_if_addrs() {
                Ok(interfaces) => {
                    let mut targets = std::collections::BTreeSet::new();
                    for interface in interfaces {
                        if interface.is_loopback() || interface.is_p2p {
                            continue;
                        }
                        if let if_addrs::IfAddr::V4(address) = interface.addr {
                            targets.extend(scan_targets(address.ip, address.netmask));
                        }
                    }
                    let scanner = discovery.clone();
                    scans.spawn(async move {
                        for target in targets {
                            tracing::debug!(%target, "Scanning local subnet for LocalSend");
                            if let Err(error) = scanner
                                .scan_subnet(target, DEFAULT_PORT, ProtocolType::Https)
                                .await
                            {
                                tracing::debug!(%error, "LocalSend subnet scan failed");
                            }
                        }
                    });
                }
                Err(error) => tracing::debug!(%error, "Cannot enumerate discovery interfaces"),
            }
        }
        emit(&events, TransferEvent::DiscoveryActive(true));
        tracing::debug!("Announcing this device");
        discovery.announce().await;
        let known = discovery
            .devices()
            .iter()
            .filter_map(|state| state.device.http().cloned())
            .filter(|channel| channel.protocol == ProtocolType::Https)
            .collect();
        tracing::debug!("Probing known peers");
        let _ = discovery.discover_known_http_channels(known).await;
        emit(&events, TransferEvent::DiscoveryActive(!scans.is_empty()));
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

// The official scanner probes /24s. Cover small home LANs through /22,
// including both halves of /23 Wi-Fi networks. On larger corporate ranges,
// bound fallback traffic to the interface's own /24; multicast remains active.
fn scan_targets(ip: std::net::Ipv4Addr, mask: std::net::Ipv4Addr) -> Vec<std::net::Ipv4Addr> {
    if !ip.is_private() {
        return Vec::new();
    }
    let mask = u32::from(mask);
    let prefix = mask.leading_ones();
    if prefix > 24 || mask != u32::MAX.checked_shl(32 - prefix).unwrap_or(0) {
        return Vec::new();
    }
    let address = u32::from(ip);
    let host = address & !mask;
    if host == 0 || host == !mask {
        return Vec::new();
    }
    let effective_mask = if prefix < 22 { 0xffffff00 } else { mask };
    let base = address & effective_mask;
    let count = ((u32::MAX ^ effective_mask) + 1) / 256;
    (0..count)
        .map(|index| {
            let subnet = base + index * 256;
            if address & 0xffffff00 == subnet {
                ip
            } else {
                std::net::Ipv4Addr::from(subnet)
            }
        })
        .collect()
}

fn machine_name() -> String {
    // macOS exposes the user-facing name separately from its DNS hostname.
    #[cfg(target_os = "macos")]
    if let Ok(output) = std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        && output.status.success()
        && let Ok(name) = String::from_utf8(output.stdout)
        && !name.trim().is_empty()
    {
        return name.trim().to_owned();
    }
    let hostname = gethostname::gethostname()
        .to_string_lossy()
        .trim()
        .to_owned();
    if hostname.is_empty() {
        "OmaSend".into()
    } else {
        hostname
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    use std::net::Ipv4Addr;

    async fn offline_discovery() -> (
        Arc<DiscoveryHandle>,
        mpsc::Receiver<DiscoveryEvent>,
        mpsc::Sender<DiscoveryEvent>,
    ) {
        let (updates, receiver) = mpsc::channel(4);
        let (_stop, stop_rx) = oneshot::channel();
        let discovery = discovery::start(
            DiscoveryConfig {
                group: DEFAULT_MULTICAST_GROUP,
                group_v6: None,
                port: 0,
                // An empty whitelist excludes every interface: no socket can
                // bind or announce, and tests never probe the user's LAN.
                interface_filter: localsend::util::interface::InterfaceFilter {
                    whitelist: Some(Vec::new()),
                    blacklist: None,
                },
                device: MulticastDevice {
                    alias: "test receiver".into(),
                    version: PROTOCOL_VERSION_V2.into(),
                    device_model: None,
                    device_type: Some(DeviceType::Desktop),
                    fingerprint: "self".into(),
                    port: 0,
                    protocol: ProtocolType::Https,
                    download: false,
                },
                identity: DeviceIdentity {
                    cert_pem: String::new(),
                    private_key_pem: String::new(),
                },
                timeout: Duration::from_secs(2),
                event_tx: Some(updates.clone()),
            },
            stop_rx,
        )
        .await;
        assert!(discovery.multicast_error().is_some());
        (Arc::new(discovery), receiver, updates)
    }

    fn confirmed_peer(alias: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            alias: alias.into(),
            version: PROTOCOL_VERSION_V2.into(),
            device_model: Some("Test desktop".into()),
            device_type: Some(DeviceType::Desktop),
            fingerprint: "confirmed-peer".into(),
            channel: DeviceChannel::Http(HttpChannel {
                host: "127.0.0.1".into(),
                port: 45678,
                protocol: ProtocolType::Https,
            }),
            download: false,
        }
    }

    #[tokio::test]
    async fn confirmations_reach_ui_while_announcement_or_probes_are_pending() {
        let (discovery, updates, _) = offline_discovery().await;
        let (events, receiver) = async_channel::unbounded();
        let task = tokio::spawn(publish_while_discovering(
            discovery.clone(),
            events,
            updates,
            std::future::pending(),
        ));
        // Model a confirmed incoming register during an unfinished announce
        // burst or unreachable-peer probe, using the official store + events.
        discovery.add_device(confirmed_peer("New computer")).await;
        let result = tokio::time::timeout(Duration::from_millis(250), receiver.recv()).await;
        let event = result
            .expect("confirmed peer must publish before discovery network work finishes")
            .unwrap();
        assert!(matches!(event, TransferEvent::DeviceFound(peer) if peer.alias == "New computer"));
        discovery
            .add_device(confirmed_peer("Renamed computer"))
            .await;
        let event = tokio::time::timeout(Duration::from_millis(250), receiver.recv())
            .await
            .expect("updated alias must publish without waiting for the next polling tick")
            .unwrap();
        task.abort();
        assert!(
            matches!(event, TransferEvent::DeviceFound(peer) if peer.alias == "Renamed computer")
        );
    }

    #[tokio::test]
    async fn snapshot_recovers_dropped_events_and_expires_visible_devices() {
        let (discovery, _updates, _) = offline_discovery().await;
        // The upstream event channel holds four messages. The official store
        // must still publish every confirmed peer after notifications overflow.
        for index in 0..8 {
            let mut peer = confirmed_peer("Computer");
            peer.fingerprint = format!("peer-{index}");
            discovery.add_device(peer).await;
        }
        let (events, receiver) = async_channel::unbounded();
        let mut visible = HashMap::new();
        publish_discovery_snapshot(&discovery, &events, &mut visible, SystemTime::now());
        assert_eq!(visible.len(), 8);
        for _ in 0..8 {
            assert!(matches!(
                receiver.try_recv().unwrap(),
                TransferEvent::DeviceFound(_)
            ));
        }
        publish_discovery_snapshot(&discovery, &events, &mut visible, SystemTime::now());
        assert!(
            receiver.try_recv().is_err(),
            "unchanged peers must not retrigger GUI work"
        );
        publish_discovery_snapshot(
            &discovery,
            &events,
            &mut visible,
            SystemTime::now() + Duration::from_secs(16),
        );
        assert!(visible.is_empty());
        for _ in 0..8 {
            assert!(matches!(
                receiver.try_recv().unwrap(),
                TransferEvent::DeviceLost(_)
            ));
        }
    }

    #[tokio::test]
    async fn discovery_failures_are_visible_and_deduplicated_per_host() {
        let (discovery, updates, sender) = offline_discovery().await;
        let (events, receiver) = async_channel::unbounded();
        let task = tokio::spawn(publish_while_discovering(
            discovery,
            events,
            updates,
            std::future::pending(),
        ));
        let failure = DiscoveryEvent::ProbeFailed {
            host: "127.0.0.1".into(),
            alias: "Office desktop".into(),
            error: "server certificate fingerprint mismatch".into(),
        };
        sender.send(failure.clone()).await.unwrap();
        sender.send(failure).await.unwrap();
        sender
            .send(DiscoveryEvent::ProbeFailed {
                host: "127.0.0.2".into(),
                alias: "Other desktop".into(),
                error: "connection refused".into(),
            })
            .await
            .unwrap();
        sender
            .send(DiscoveryEvent::ProbeFailed {
                host: "127.0.0.2".into(),
                alias: "Other desktop".into(),
                error: "server certificate fingerprint mismatch".into(),
            })
            .await
            .unwrap();
        for expected in [
            "Nearby device identity could not be verified: Office desktop (127.0.0.1)",
            "Nearby device identity could not be verified: Other desktop (127.0.0.2)",
        ] {
            let event = tokio::time::timeout(Duration::from_millis(250), receiver.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(event, TransferEvent::NetworkError(message) if message == expected));
        }
        task.abort();
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn wifi_slash_23_covers_both_halves_and_large_ranges_are_bounded() {
        assert_eq!(
            scan_targets(
                Ipv4Addr::new(10, 20, 94, 239),
                Ipv4Addr::new(255, 255, 254, 0)
            ),
            vec![Ipv4Addr::new(10, 20, 94, 239), Ipv4Addr::new(10, 20, 95, 0)]
        );
        assert_eq!(
            scan_targets(
                Ipv4Addr::new(10, 20, 94, 239),
                Ipv4Addr::new(255, 255, 0, 0)
            ),
            vec![Ipv4Addr::new(10, 20, 94, 239)]
        );
    }
    #[test]
    fn excludes_public_addresses_vpn_benchmark_ranges_and_small_subnets() {
        for address in [
            Ipv4Addr::new(8, 8, 8, 8),
            Ipv4Addr::new(198, 18, 0, 1),
            Ipv4Addr::LOCALHOST,
        ] {
            assert!(scan_targets(address, Ipv4Addr::new(255, 255, 255, 0)).is_empty());
        }
        assert!(
            scan_targets(
                Ipv4Addr::new(10, 0, 0, 2),
                Ipv4Addr::new(255, 255, 255, 252)
            )
            .is_empty()
        );
    }
}
