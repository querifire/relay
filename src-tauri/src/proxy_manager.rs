use crate::local_proxy;
use crate::proxy_cache;
use crate::proxy_chain::ProxyChainConfig;
use crate::proxy_instance::{
    push_to_sink, LogSink, ProxyInstance, ProxyInstanceInfo, ProxyStats, ProxyStatus, SavedInstance,
};
use crate::proxy_lists::{self, ProxyListConfig};
use crate::proxy_type::{Proxy, ProxyMode, ProxyProtocol};
use crate::sources;
use crate::speed_test::{self, ProxyWithSpeed};
use anyhow::{anyhow, Result};
use futures::stream::{self, StreamExt};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Manages multiple proxy instances, each with its own local SOCKS5 server.
pub struct ProxyManager {
    instances: HashMap<Uuid, ProxyInstance>,
    pub default_concurrency: usize,
}

/// Maximum number of proxy instances allowed.
const MAX_INSTANCES: usize = 100;

/// Normalize bind address: default to 127.0.0.1 for empty. Reject 0.0.0.0 (open proxy risk).
fn normalize_bind_addr(addr: String) -> Result<String, String> {
    let s = addr.trim();
    if s.is_empty() {
        return Ok("127.0.0.1".to_string());
    }
    if s == "0.0.0.0" || s.eq_ignore_ascii_case("::") {
        return Err(
            "Binding to 0.0.0.0 or :: makes the proxy accessible to the entire network. Use 127.0.0.1 for local-only access.".into()
        );
    }
    Ok(s.to_string())
}

/// Path to the persisted instances file.
fn instances_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join("relay").join("instances.json")
}

impl ProxyManager {
    pub fn new(default_concurrency: usize) -> Self {
        Self {
            instances: HashMap::new(),
            default_concurrency,
        }
    }

    pub async fn load_instances(&mut self) {
        let path = instances_path();
        if !path.exists() {
            return;
        }
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => {
                if let Ok(saved) = serde_json::from_str::<Vec<SavedInstance>>(&content) {
                    for s in saved {
                        let inst = ProxyInstance::from_saved(s);
                        self.instances.insert(inst.id, inst);
                    }
                    tracing::info!(
                        "[OK] Загружено {} сохранённых инстансов",
                        self.instances.len()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("[!] Не удалось загрузить инстансы: {}", e);
            }
        }
    }

    pub async fn save_instances(&self) {
        let path = instances_path();
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let saved: Vec<SavedInstance> = self.instances.values().map(|i| i.to_saved()).collect();
        match serde_json::to_string_pretty(&saved) {
            Ok(json) => {
                if let Err(e) = crate::atomic_write::atomic_write_async(&path, &json).await {
                    tracing::warn!("[!] Не удалось сохранить инстансы: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("[!] Ошибка сериализации инстансов: {}", e);
            }
        }
    }

    pub fn create_instance(
        &mut self,
        name: String,
        bind_addr: String,
        port: u16,
        mode: ProxyMode,
        local_protocol: ProxyProtocol,
        auth_username: Option<String>,
        auth_password: Option<String>,
        auto_rotate: bool,
        proxy_list: String,
        auto_rotate_minutes: Option<u64>,
        auto_start_on_boot: bool,
        proxy_chain: Option<ProxyChainConfig>,
    ) -> Result<ProxyInstanceInfo> {
        let bind_addr = normalize_bind_addr(bind_addr)
            .map_err(|e| anyhow!("{}", e))?;

        if self.instances.len() >= MAX_INSTANCES {
            return Err(anyhow!(
                "Maximum number of instances ({}) reached",
                MAX_INSTANCES
            ));
        }
        for inst in self.instances.values() {
            if inst.port == port && inst.bind_addr == bind_addr {
                return Err(anyhow!(
                    "Port {} on {} is already used by '{}'",
                    port,
                    bind_addr,
                    inst.name
                ));
            }
        }
        if name.trim().is_empty() {
            return Err(anyhow!("Instance name cannot be empty"));
        }
        if port == 0 {
            return Err(anyhow!("Port must be between 1 and 65535"));
        }
        // SOCKS4 does not support authentication; allowing auth would create an open proxy.
        if local_protocol == ProxyProtocol::Socks4 {
            let has_auth = auth_username.as_ref().map_or(false, |u| !u.is_empty())
                || auth_password.as_ref().map_or(false, |p| !p.is_empty());
            if has_auth {
                return Err(anyhow!(
                    "SOCKS4 does not support authentication. Use SOCKS5 or HTTP for local proxy with password, or disable authentication."
                ));
            }
        }

        let auto_rotate_minutes = auto_rotate_minutes.map(|m| m.clamp(1, 1440));

        let instance = ProxyInstance::new(
            name,
            bind_addr,
            port,
            mode,
            local_protocol,
            auth_username,
            auth_password,
            auto_rotate,
            proxy_list,
            auto_rotate_minutes,
            auto_start_on_boot,
            proxy_chain,
        );
        let info = instance.to_info();
        self.instances.insert(instance.id, instance);
        Ok(info)
    }

    /// Get the anonymity level Arc for an instance (for updating from background tasks).
    pub fn get_anonymity_arc(
        &self,
        id: Uuid,
    ) -> Option<Arc<parking_lot::RwLock<Option<crate::anonymity_check::AnonymityLevel>>>> {
        self.instances.get(&id).map(|i| i.anonymity_level.clone())
    }

    /// Phase 1: validate & set status to Starting.
    pub fn mark_starting(
        &mut self,
        id: Uuid,
    ) -> Result<(ProxyMode, usize, LogSink, ProxyProtocol, CancellationToken, Arc<ProxyStats>, String, Option<u64>, String, u16)>
    {
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;

        match &instance.status {
            ProxyStatus::Running => return Err(anyhow!("Instance is already running")),
            ProxyStatus::Starting => return Err(anyhow!("Instance is already starting")),
            _ => {}
        }

        let discovery_token = CancellationToken::new();
        instance.discovery_token = Some(discovery_token.clone());

        instance.stats.reset();
        instance.upstream_latency_ms.store(0, Ordering::Relaxed);

        instance.status = ProxyStatus::Starting;
        instance.push_log("Starting…".into());

        Ok((
            instance.mode.clone(),
            self.default_concurrency,
            instance.logs.clone(),
            instance.local_protocol.clone(),
            discovery_token,
            instance.stats.clone(),
            instance.proxy_list.clone(),
            instance.auto_rotate_minutes,
            instance.bind_addr.clone(),
            instance.port,
        ))
    }

    /// Phase 2: store upstream, spawn the local server, mark Running.
    pub fn finish_start(
        &mut self,
        id: Uuid,
        upstream_proxy: Proxy,
        latency_ms: u64,
    ) -> Result<ProxyInstanceInfo> {
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance was deleted while starting"))?;

        if !matches!(instance.status, ProxyStatus::Starting) {
            return Err(anyhow!("Instance is no longer in Starting state"));
        }

        instance.discovery_token = None;
        instance.upstream_latency_ms.store(latency_ms, Ordering::Relaxed);

        *instance.upstream.write() = Some(upstream_proxy.clone());

        let token = CancellationToken::new();
        instance.cancel_token = Some(token.clone());

        let listen_addr = format!("{}:{}", instance.bind_addr, instance.port);
        let upstream_arc: Arc<RwLock<Proxy>> = Arc::new(RwLock::new(upstream_proxy.clone()));
        let log_sink = instance.logs.clone();
        let stats = instance.stats.clone();

        instance.push_log(format!(
            "Upstream: {}://{}:{}",
            upstream_proxy.protocol, upstream_proxy.host, upstream_proxy.port
        ));
        instance.push_log(format!(
            "Local protocol: {}",
            instance.local_protocol
        ));
        instance.push_log(format!("Listening on {}", listen_addr));

        let auth = match (&instance.auth_username, &instance.auth_password) {
            (Some(u), Some(p)) if !u.is_empty() => Some((u.clone(), p.clone())),
            _ => None,
        };

        let chain = instance
            .proxy_chain
            .as_ref()
            .filter(|c| c.enabled && !c.proxies.is_empty())
            .map(|c| Arc::new(c.proxies.clone()));

        if chain.is_some() {
            instance.push_log(format!(
                "Proxy chain enabled ({} intermediate hops)",
                chain.as_ref().unwrap().len()
            ));
        }

        let handle = local_proxy::start_local_server(
            instance.local_protocol.clone(),
            listen_addr,
            upstream_arc.clone(),
            token,
            log_sink.clone(),
            auth,
            stats,
            chain,
        );
        instance.handle = Some(handle);
        instance.status = ProxyStatus::Running;

        if instance.auto_rotate && instance.mode == ProxyMode::Auto {
            let rotation_cancel = CancellationToken::new();
            instance.rotation_token = Some(rotation_cancel.clone());

            let upstream_shared = instance.upstream.clone();
            let upstream_latency = instance.upstream_latency_ms.clone();
            let rotation_sink = log_sink;
            let concurrency = self.default_concurrency;
            let protocol = instance.local_protocol.clone();
            let plist = instance.proxy_list.clone();
            let rotate_mins = instance.auto_rotate_minutes;

            tokio::spawn(Self::rotation_loop(
                upstream_shared,
                upstream_latency,
                rotation_cancel,
                rotation_sink,
                concurrency,
                protocol,
                plist,
                rotate_mins,
            ));

            let interval_label = rotate_mins.unwrap_or(5);
            instance.push_log(format!("Auto-rotation enabled (every {} min)", interval_label));
        }

        Ok(instance.to_info())
    }

    /// Phase 2 (Tor mode): store the Tor child process handle, mark Running.
    pub fn finish_start_tor(
        &mut self,
        id: Uuid,
        tor_child: std::process::Child,
    ) -> Result<ProxyInstanceInfo> {
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance was deleted while starting"))?;

        if !matches!(instance.status, ProxyStatus::Starting) {
            return Err(anyhow!("Instance is no longer in Starting state"));
        }

        instance.discovery_token = None;

        let token = CancellationToken::new();
        instance.cancel_token = Some(token.clone());

        let pid = tor_child.id();
        instance.push_log(format!("Tor process started (PID {})", pid));

        let child = Arc::new(std::sync::Mutex::new(Some(tor_child)));
        let child_for_cancel = child.clone();

        let cancel_token = token.clone();
        tokio::spawn(async move {
            cancel_token.cancelled().await;
            if let Ok(mut guard) = child_for_cancel.lock() {
                if let Some(mut c) = guard.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            }
        });

        let tor_proxy = Proxy::new(instance.bind_addr.clone(), instance.port, ProxyProtocol::Socks5);
        *instance.upstream.write() = Some(tor_proxy.clone());

        let latency_arc = instance.upstream_latency_ms.clone();
        let ping_cancel = token.clone();
        
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            loop {
                if let Some(latency) = speed_test::test_proxy(&tor_proxy).await {
                    latency_arc.store(latency.as_millis() as u64, Ordering::Relaxed);
                } else {
                    latency_arc.store(0, Ordering::Relaxed);
                }
                tokio::select! {
                    _ = ping_cancel.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(15)) => {}
                }
            }
        });

        instance.status = ProxyStatus::Running;
        Ok(instance.to_info())
    }

    pub fn mark_error(&mut self, id: Uuid, msg: String) {
        if let Some(instance) = self.instances.get_mut(&id) {
            instance.push_log(format!("Error: {}", msg));
            instance.status = ProxyStatus::Error(msg);
        }
    }

    pub async fn stop_instance(&mut self, id: Uuid) -> Result<ProxyInstanceInfo> {
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;

        if let Some(discovery_token) = instance.discovery_token.take() {
            discovery_token.cancel();
        }

        if let Some(rotation_token) = instance.rotation_token.take() {
            rotation_token.cancel();
        }

        if let Some(token) = instance.cancel_token.take() {
            token.cancel();
        }
        if let Some(handle) = instance.handle.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
        }

        instance.status = ProxyStatus::Stopped;
        *instance.upstream.write() = None;
        instance.upstream_latency_ms.store(0, Ordering::Relaxed);
        instance.stats.reset();
        instance.push_log("Stopped".into());

        Ok(instance.to_info())
    }

    pub async fn delete_instance(&mut self, id: Uuid) -> Result<()> {
        if let Some(inst) = self.instances.get(&id) {
            if matches!(inst.status, ProxyStatus::Running) {
                self.stop_instance(id).await?;
            }
        }
        self.instances
            .remove(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;
        Ok(())
    }

    pub fn rename_instance(&mut self, id: Uuid, name: String) -> Result<ProxyInstanceInfo> {
        if name.trim().is_empty() {
            return Err(anyhow!("Name cannot be empty"));
        }
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;
        instance.name = name;
        Ok(instance.to_info())
    }

    pub fn get_all(&self) -> Vec<ProxyInstanceInfo> {
        let mut list: Vec<_> = self.instances.values().collect();
        list.sort_by_key(|i| i.created_at);
        list.into_iter().map(|i| i.to_info()).collect()
    }

    /// Returns (allowed_ports, upstream_hosts) for running instances for kill-switch.
    /// Upstream hosts are resolved to IPs by the caller; loopback is allowed separately.
    pub fn get_running_kill_switch_context(&self) -> (Vec<u16>, Vec<String>) {
        let mut ports = Vec::new();
        let mut hosts = HashSet::new();
        for inst in self.instances.values() {
            if !matches!(inst.status, ProxyStatus::Running) {
                continue;
            }
            ports.push(inst.port);
            if inst.mode == ProxyMode::Tor {
                hosts.insert(inst.bind_addr.clone());
            } else {
                if let Some(ref u) = *inst.upstream.read() {
                    hosts.insert(u.host.clone());
                }
                if let Some(ref chain) = inst.proxy_chain {
                    if chain.enabled {
                        for p in &chain.proxies {
                            hosts.insert(p.host.clone());
                        }
                    }
                }
            }
        }
        let host_list: Vec<String> = hosts.into_iter().collect();
        (ports, host_list)
    }

    pub fn get_instance(&self, id: Uuid) -> Option<ProxyInstanceInfo> {
        self.instances.get(&id).map(|i| i.to_info())
    }

    pub fn get_instance_logs(&self, id: Uuid) -> Result<Vec<String>> {
        let instance = self
            .instances
            .get(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;
        Ok(instance.get_logs())
    }

    /// Return all instances that have auto_start_on_boot = true.
    pub fn get_auto_start_ids(&self) -> Vec<Uuid> {
        self.instances
            .values()
            .filter(|i| i.auto_start_on_boot)
            .map(|i| i.id)
            .collect()
    }

    /// Get context needed for changing IP on a running instance.
    pub fn get_change_ip_context(
        &self,
        id: Uuid,
    ) -> Result<(Arc<RwLock<Option<Proxy>>>, Option<Proxy>, ProxyProtocol, LogSink, Arc<AtomicU64>)> {
        let instance = self
            .instances
            .get(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;

        if !matches!(instance.status, ProxyStatus::Running) {
            return Err(anyhow!("Instance is not running"));
        }

        Ok((
            instance.upstream.clone(),
            instance.upstream.read().clone(),
            instance.local_protocol.clone(),
            instance.logs.clone(),
            instance.upstream_latency_ms.clone(),
        ))
    }

    pub fn toggle_auto_start_on_boot(&mut self, id: Uuid, enabled: bool) -> Result<ProxyInstanceInfo> {
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;
        instance.auto_start_on_boot = enabled;
        Ok(instance.to_info())
    }

    /// Discover the fastest upstream proxy automatically.
    /// Returns the selected proxy WITH its measured latency.
    pub async fn auto_discover_upstream(
        concurrency: usize,
        log_sink: LogSink,
        protocol: ProxyProtocol,
        custom_list: Option<ProxyListConfig>,
    ) -> Result<ProxyWithSpeed> {
        let source_label = custom_list
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("built-in");
        push_to_sink(
            &log_sink,
            format!("Auto-discovery for {} proxies (source: {})…", protocol, source_label),
        );

        let all_cached = proxy_cache::load_cache().await.unwrap_or_default();
        let cached: Vec<_> = all_cached
            .into_iter()
            .filter(|p| p.protocol == protocol)
            .collect();

        let mut tested = if !cached.is_empty() {
            push_to_sink(&log_sink, format!("Checking {} cached {} proxies…", cached.len(), protocol));
            Self::test_with_progress(cached.clone(), concurrency, &log_sink).await
        } else {
            Vec::new()
        };

        push_to_sink(&log_sink, format!("Fetching {} proxy lists…", protocol));
        let new_proxies = if let Some(ref config) = custom_list {
            proxy_lists::fetch_from_config(config, protocol.clone(), Some(&log_sink)).await
        } else {
            sources::fetch_proxies(protocol.clone(), Some(&log_sink)).await?
        };

        if new_proxies.is_empty() && tested.is_empty() {
            return Err(anyhow!("No {} proxies available", protocol));
        }

        let cached_set: HashSet<_> = cached.into_iter().collect();
        let new_unique: Vec<_> = new_proxies
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|p| !cached_set.contains(p))
            .collect();

        if !new_unique.is_empty() {
            push_to_sink(
                &log_sink,
                format!("Testing {} new {} proxies…", new_unique.len(), protocol),
            );
            let new_tested =
                Self::test_with_progress(new_unique, concurrency, &log_sink).await;
            tested.extend(new_tested);
        }

        if tested.is_empty() {
            return Err(anyhow!("All {} proxies are unreachable", protocol));
        }

        let working: Vec<_> = tested.iter().map(|p| p.proxy.clone()).collect();
        push_to_sink(
            &log_sink,
            format!("Found {} working {} proxies", working.len(), protocol),
        );

        let mut merged_cache = proxy_cache::load_cache().await.unwrap_or_default();
        merged_cache.retain(|p| p.protocol != protocol);
        merged_cache.extend(working);
        if let Err(e) = proxy_cache::save_cache(&merged_cache).await {
            tracing::warn!("[!] Не удалось сохранить кэш: {}", e);
        }

        let fastest = speed_test::select_fastest(tested)
            .ok_or_else(|| anyhow!("Failed to select a {} proxy", protocol))?;

        push_to_sink(
            &log_sink,
            format!(
                "Selected: {}://{}:{} ({}ms)",
                fastest.proxy.protocol,
                fastest.proxy.host,
                fastest.proxy.port,
                fastest.latency.as_millis()
            ),
        );

        Ok(fastest)
    }

    /// Background task that periodically re-tests cached proxies and rotates
    /// the upstream proxy to a different fastest one (excluding the current).
    async fn rotation_loop(
        upstream_shared: Arc<RwLock<Option<Proxy>>>,
        upstream_latency_ms: Arc<AtomicU64>,
        cancel: CancellationToken,
        log_sink: LogSink,
        concurrency: usize,
        protocol: ProxyProtocol,
        proxy_list: String,
        auto_rotate_minutes: Option<u64>,
    ) {
        let minutes = auto_rotate_minutes.unwrap_or(5).clamp(1, 1440);
        let interval = Duration::from_secs(minutes * 60);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    push_to_sink(&log_sink, "Rotation task stopped".to_string());
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    push_to_sink(&log_sink, format!("Rotation: re-testing {} proxies…", protocol));

                    let cached = proxy_cache::load_cache().await.unwrap_or_default();
                    let filtered: Vec<_> = cached
                        .into_iter()
                        .filter(|p| p.protocol == protocol)
                        .collect();

                    if filtered.is_empty() {
                        push_to_sink(&log_sink, "Rotation: no cached proxies to test".to_string());
                        continue;
                    }

                    let tested = speed_test::test_proxies_parallel(filtered, concurrency).await;

                    if tested.is_empty() {
                        push_to_sink(&log_sink, "Rotation: all cached proxies unreachable, trying fresh…".to_string());

                        let fresh_result = if proxy_list != "default" {
                            match proxy_lists::find_by_id(&proxy_list).await {
                                Some(config) => Ok(proxy_lists::fetch_from_config(&config, protocol.clone(), Some(&log_sink)).await),
                                None => {
                                    push_to_sink(&log_sink, format!("Rotation: custom list '{}' not found, using built-in", proxy_list));
                                    sources::fetch_proxies(protocol.clone(), Some(&log_sink)).await
                                }
                            }
                        } else {
                            sources::fetch_proxies(protocol.clone(), Some(&log_sink)).await
                        };
                        match fresh_result {
                            Ok(fresh) if !fresh.is_empty() => {
                                let fresh_tested = speed_test::test_proxies_parallel(fresh, concurrency).await;
                                let current = upstream_shared.read().clone();
                                let candidates: Vec<_> = fresh_tested.iter()
                                    .filter(|p| current.as_ref().map_or(true, |c| c.host != p.proxy.host || c.port != p.proxy.port))
                                    .cloned()
                                    .collect();
                                let chosen = if candidates.is_empty() {
                                    speed_test::select_fastest(fresh_tested)
                                } else {
                                    speed_test::select_fastest(candidates)
                                };
                                if let Some(fastest) = chosen {
                                    push_to_sink(
                                        &log_sink,
                                        format!(
                                            "Rotation: switched to {}://{}:{} ({}ms)",
                                            fastest.proxy.protocol,
                                            fastest.proxy.host,
                                            fastest.proxy.port,
                                            fastest.latency.as_millis()
                                        ),
                                    );
                                    upstream_latency_ms.store(fastest.latency.as_millis() as u64, Ordering::Relaxed);
                                    *upstream_shared.write() = Some(fastest.proxy);
                                } else {
                                    push_to_sink(&log_sink, "Rotation: no working fresh proxies found".to_string());
                                }
                            }
                            _ => {
                                push_to_sink(&log_sink, "Rotation: failed to fetch fresh proxies".to_string());
                            }
                        }
                        continue;
                    }

                    let working: Vec<_> = tested.iter().map(|p| p.proxy.clone()).collect();
                    let mut merged = proxy_cache::load_cache().await.unwrap_or_default();
                    merged.retain(|p| p.protocol != protocol);
                    merged.extend(working);
                    let _ = proxy_cache::save_cache(&merged).await;

                    // Exclude current proxy from rotation candidates (avoid no-op switch).
                    let current = upstream_shared.read().clone();
                    let candidates: Vec<_> = tested.iter()
                        .filter(|p| current.as_ref().map_or(true, |c| c.host != p.proxy.host || c.port != p.proxy.port))
                        .cloned()
                        .collect();
                    let chosen = if candidates.is_empty() {
                        speed_test::select_fastest(tested)
                    } else {
                        speed_test::select_fastest(candidates)
                    };

                    if let Some(fastest) = chosen {
                        let new_proxy = fastest.proxy.clone();

                        let changed = current
                            .as_ref()
                            .map(|c| c.host != new_proxy.host || c.port != new_proxy.port)
                            .unwrap_or(true);

                        if changed {
                            push_to_sink(
                                &log_sink,
                                format!(
                                    "Rotation: switched to {}://{}:{} ({}ms)",
                                    new_proxy.protocol,
                                    new_proxy.host,
                                    new_proxy.port,
                                    fastest.latency.as_millis()
                                ),
                            );
                        } else {
                            push_to_sink(
                                &log_sink,
                                format!(
                                    "Rotation: keeping {}://{}:{} ({}ms) — no alternatives",
                                    new_proxy.protocol,
                                    new_proxy.host,
                                    new_proxy.port,
                                    fastest.latency.as_millis()
                                ),
                            );
                        }

                        upstream_latency_ms.store(fastest.latency.as_millis() as u64, Ordering::Relaxed);
                        *upstream_shared.write() = Some(new_proxy);
                    }
                }
            }
        }
    }

    pub fn update_proxy_list(&mut self, id: Uuid, proxy_list: String) -> Result<ProxyInstanceInfo> {
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;
        instance.proxy_list = proxy_list;
        Ok(instance.to_info())
    }

    pub fn toggle_auto_rotate(&mut self, id: Uuid, enabled: bool) -> Result<ProxyInstanceInfo> {
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;

        instance.auto_rotate = enabled;

        if !enabled {
            if let Some(token) = instance.rotation_token.take() {
                token.cancel();
            }
            instance.push_log("Auto-rotation disabled".into());
        } else if matches!(instance.status, ProxyStatus::Running) && instance.mode == ProxyMode::Auto
        {
            if instance.rotation_token.is_none() {
                let rotation_cancel = CancellationToken::new();
                instance.rotation_token = Some(rotation_cancel.clone());

                let upstream_shared = instance.upstream.clone();
                let upstream_latency = instance.upstream_latency_ms.clone();
                let rotation_sink = instance.logs.clone();
                let concurrency = self.default_concurrency;
                let protocol = instance.local_protocol.clone();
                let plist = instance.proxy_list.clone();
                let rotate_mins = instance.auto_rotate_minutes;

                tokio::spawn(Self::rotation_loop(
                    upstream_shared,
                    upstream_latency,
                    rotation_cancel,
                    rotation_sink,
                    concurrency,
                    protocol,
                    plist,
                    rotate_mins,
                ));

                let interval_label = rotate_mins.unwrap_or(5);
                instance.push_log(format!("Auto-rotation enabled (every {} min)", interval_label));
            }
        }

        Ok(instance.to_info())
    }

    /// Update the per-instance rotation interval.
    pub fn update_auto_rotate_minutes(&mut self, id: Uuid, minutes: u64) -> Result<ProxyInstanceInfo> {
        let minutes = minutes.clamp(1, 1440);
        let instance = self
            .instances
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Instance {} not found", id))?;

        instance.auto_rotate_minutes = Some(minutes);

        if instance.auto_rotate
            && matches!(instance.status, ProxyStatus::Running)
            && instance.mode == ProxyMode::Auto
        {
            if let Some(token) = instance.rotation_token.take() {
                token.cancel();
            }

            let rotation_cancel = CancellationToken::new();
            instance.rotation_token = Some(rotation_cancel.clone());

            let upstream_shared = instance.upstream.clone();
            let upstream_latency = instance.upstream_latency_ms.clone();
            let rotation_sink = instance.logs.clone();
            let concurrency = self.default_concurrency;
            let protocol = instance.local_protocol.clone();
            let plist = instance.proxy_list.clone();

            tokio::spawn(Self::rotation_loop(
                upstream_shared,
                upstream_latency,
                rotation_cancel,
                rotation_sink,
                concurrency,
                protocol,
                plist,
                Some(minutes),
            ));

            instance.push_log(format!("Rotation interval updated to {} min", minutes));
        }

        Ok(instance.to_info())
    }

    /// Run proxy tests with periodic progress updates written to `log_sink`.
    async fn test_with_progress(
        proxies: Vec<Proxy>,
        concurrency: usize,
        log_sink: &LogSink,
    ) -> Vec<speed_test::ProxyWithSpeed> {
        let total = proxies.len();
        let tested_count = Arc::new(AtomicUsize::new(0));
        let working_count = Arc::new(AtomicUsize::new(0));

        let tc = tested_count.clone();
        let wc = working_count.clone();
        let sink = log_sink.clone();

        let progress_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;
                let t = tc.load(Ordering::Relaxed);
                let w = wc.load(Ordering::Relaxed);
                push_to_sink(
                    &sink,
                    format!("  progress: {}/{} tested, {} working", t, total, w),
                );
            }
        });
        let _guard = AbortOnDrop(progress_handle);

        let tc2 = tested_count.clone();
        let wc2 = working_count.clone();

        let results: Vec<Option<speed_test::ProxyWithSpeed>> = stream::iter(proxies)
            .map(move |proxy| {
                let tc = tc2.clone();
                let wc = wc2.clone();
                async move {
                    let latency = speed_test::test_proxy(&proxy).await;
                    tc.fetch_add(1, Ordering::Relaxed);
                    if let Some(lat) = latency {
                        wc.fetch_add(1, Ordering::Relaxed);
                        Some(speed_test::ProxyWithSpeed {
                            proxy,
                            latency: lat,
                        })
                    } else {
                        None
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        let working: Vec<_> = results.into_iter().flatten().collect();
        push_to_sink(
            log_sink,
            format!("  done: {}/{} working", working.len(), total),
        );
        working
    }
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
