#[cfg(windows)]
use std::os::windows::process::CommandExt;
use crate::anonymity_check::{self, AnonymityLevel};
use crate::geoip;
use crate::import_export;
use crate::kill_switch::KillSwitchState;
use crate::leak_test::{self, LeakTestResult};
use crate::notifications;
use crate::plugin_manager::PluginManager;
use crate::plugin_sdk::PluginInfo;
use crate::profiles::{self, Profile, SaveProfileRequest};
use crate::proxy_chain::ProxyChainConfig;
use crate::proxy_instance::{push_to_sink, ProxyInstanceInfo, ProxyStatusInfo};
use crate::proxy_lists::{self, ProxyListConfig};
use crate::proxy_manager::ProxyManager;
use crate::proxy_type::{Proxy, ProxyMode, ProxyProtocol};
use crate::scheduler::{self, SaveScheduleRequest, Schedule};
use crate::settings::{AppSettings, NotificationSettings};
use crate::speed_test::ProxyWithSpeed;
use crate::split_tunnel::{self, RoutingRule, SaveRoutingRuleRequest};
use crate::system_proxy;
use crate::tls_fingerprint;
use crate::{proxy_cache, sources, speed_test};
use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct ProxyManagerState(pub Mutex<ProxyManager>);
pub struct PluginManagerState(pub Mutex<PluginManager>);
pub struct SettingsState(pub Mutex<AppSettings>);
pub struct KillSwitchStateWrapper(pub KillSwitchState);

fn map_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn validate_tor_binary_path(path: &str) -> Result<(), String> {
    let path_trim = path.trim();
    if path_trim.is_empty() {
        return Err("Tor binary path is empty".to_string());
    }
    let has_sep = path_trim.contains(std::path::MAIN_SEPARATOR) || path_trim.contains('/');
    if !has_sep {
        return Ok(());
    }
    let p = std::path::Path::new(path_trim);
    if !p.exists() {
        return Err(format!("Tor binary path does not exist: {}", path_trim));
    }
    if !p.is_file() {
        return Err(format!("Tor binary path is not a file: {}", path_trim));
    }
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !name.eq_ignore_ascii_case("tor") && !name.eq_ignore_ascii_case("tor.exe") {
        return Err(format!(
            "Tor binary must be named 'tor' or 'tor.exe', got: {}",
            name
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_instances(
    manager: State<'_, ProxyManagerState>,
) -> Result<Vec<ProxyInstanceInfo>, String> {
    let mgr = manager.0.lock().await;
    Ok(mgr.get_all())
}

#[tauri::command]
pub async fn get_instance(
    manager: State<'_, ProxyManagerState>,
    id: String,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mgr = manager.0.lock().await;
    mgr.get_instance(uuid)
        .ok_or_else(|| format!("Instance {} not found", id))
}

#[tauri::command]
pub async fn create_instance(
    manager: State<'_, ProxyManagerState>,
    name: String,
    bind_addr: String,
    port: u16,
    mode: ProxyMode,
    local_protocol: Option<ProxyProtocol>,
    auth_username: Option<String>,
    auth_password: Option<String>,
    auto_rotate: Option<bool>,
    proxy_list: Option<String>,
    auto_rotate_minutes: Option<u64>,
    auto_start_on_boot: Option<bool>,
    proxy_chain: Option<ProxyChainConfig>,
) -> Result<ProxyInstanceInfo, String> {
    let mut mgr = manager.0.lock().await;
    let info = mgr
        .create_instance(
            name,
            bind_addr,
            port,
            mode,
            local_protocol.unwrap_or(ProxyProtocol::Socks5),
            auth_username,
            auth_password,
            auto_rotate.unwrap_or(false),
            proxy_list.unwrap_or_else(|| "default".to_string()),
            auto_rotate_minutes,
            auto_start_on_boot.unwrap_or(false),
            proxy_chain,
        )
        .map_err(map_err)?;
    mgr.save_instances().await;
    Ok(info)
}

/// Start a proxy instance.
///
/// The heavy lifting (proxy discovery for Auto mode) runs **without** holding
/// the ProxyManager mutex, so other commands (e.g. `get_instances`) are not
/// blocked during the potentially long network operation.
#[tauri::command]
pub async fn start_instance(
    app: tauri::AppHandle,
    manager: State<'_, ProxyManagerState>,
    settings: State<'_, SettingsState>,
    kill_switch: State<'_, KillSwitchStateWrapper>,
    id: String,
    upstream_host: Option<String>,
    upstream_port: Option<u16>,
    upstream_protocol: Option<String>,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;

    let manual_upstream = match (upstream_host, upstream_port) {
        (Some(host), Some(port)) => {
            let protocol = upstream_protocol
                .as_deref()
                .unwrap_or("socks5")
                .to_lowercase();
            let proto = match protocol.as_str() {
                "http" | "https" => crate::proxy_type::ProxyProtocol::Http,
                "socks4" => crate::proxy_type::ProxyProtocol::Socks4,
                _ => crate::proxy_type::ProxyProtocol::Socks5,
            };
            Some(Proxy::new(host, port, proto))
        }
        _ => None,
    };

    let (mode, concurrency, log_sink, local_protocol, discovery_token, _stats, proxy_list, _auto_rotate_minutes, bind_addr, port) = {
        let mut mgr = manager.0.lock().await;
        mgr.mark_starting(uuid).map_err(map_err)?
        
    };

    if mode == ProxyMode::Tor {
        let tor_config = {
            let s = settings.0.lock().await;
            s.tor_config.clone()
        };

        let tor_path = tor_config
            .binary_path
            .clone()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "tor".to_string());

        if let Err(e) = validate_tor_binary_path(&tor_path) {
            let mut mgr = manager.0.lock().await;
            mgr.mark_error(uuid, e.clone());
            return Err(e);
        }

        push_to_sink(&log_sink, format!("Tor binary: {}", tor_path));
        push_to_sink(&log_sink, format!("Launching Tor on {}:{}…", bind_addr, port));

        if let Ok(true) = crate::port_kill::port_is_in_use(port) {
            let msg = format!(
                "Port {} is in use. Please stop the application using this port or choose another port.",
                port
            );
            let mut mgr = manager.0.lock().await;
            mgr.mark_error(uuid, msg.clone());
            return Err(msg);
        }

        let data_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("relay")
            .join("tor-data")
            .join(uuid.to_string());

        push_to_sink(&log_sink, format!("Tor data dir: {}", data_dir.display()));

        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            let msg = format!("Failed to create Tor data dir: {}", e);
            let mut mgr = manager.0.lock().await;
            mgr.mark_error(uuid, msg.clone());
            return Err(msg);
        }

        let torrc_content = match crate::settings::generate_torrc(
            &tor_config,
            &bind_addr,
            port,
            &data_dir,
        ) {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("Invalid custom torrc: {}", e);
                let mut mgr = manager.0.lock().await;
                mgr.mark_error(uuid, msg.clone());
                return Err(msg);
            }
        };
        let torrc_path = data_dir.join("torrc");
        if let Err(e) = std::fs::write(&torrc_path, &torrc_content) {
            let msg = format!("Failed to write torrc: {}", e);
            let mut mgr = manager.0.lock().await;
            mgr.mark_error(uuid, msg.clone());
            return Err(msg);
        }
        push_to_sink(&log_sink, format!("Using torrc: {}", torrc_path.display()));

        const MAX_TOR_ATTEMPTS: u32 = 3;
        let mut last_error = String::new();

        for attempt in 1..=MAX_TOR_ATTEMPTS {
            if attempt > 1 {
                push_to_sink(&log_sink, format!("Tor: retrying (attempt {}/{})", attempt, MAX_TOR_ATTEMPTS));
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }

            let mut cmd = std::process::Command::new(&tor_path);
            cmd.arg("-f")
                .arg(torrc_path.to_string_lossy().to_string())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000);
            }
            let child_result = cmd.spawn();

            match child_result {
                Ok(mut child) => {
                    let pid = child.id();
                    push_to_sink(&log_sink, format!("Tor process spawned (PID {})", pid));

                    let child_stdout = child.stdout.take();
                    let child_stderr = child.stderr.take();

                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

                    match child.try_wait() {
                        Ok(Some(exit_status)) => {
                            let mut err_output = String::new();
                            if let Some(mut stderr) = child_stderr {
                                use std::io::Read;
                                let _ = stderr.read_to_string(&mut err_output);
                            }
                            if err_output.is_empty() {
                                if let Some(mut stdout) = child_stdout {
                                    use std::io::Read;
                                    let _ = stdout.read_to_string(&mut err_output);
                                }
                            }
                            last_error = format!(
                                "Tor exited immediately (code: {}). Output:\n{}",
                                exit_status,
                                if err_output.is_empty() { "(no output)".to_string() } else { err_output.chars().take(1000).collect() }
                            );
                            push_to_sink(&log_sink, last_error.clone());
                        }
                        Ok(None) => {
                            push_to_sink(&log_sink, "Tor is running".to_string());

                            if let Some(stdout) = child_stdout {
                                let sink = log_sink.clone();
                                tokio::spawn(async move {
                                    use tokio::io::{AsyncBufReadExt, BufReader};
                                    let reader = BufReader::new(tokio::process::ChildStdout::from_std(stdout).unwrap());
                                    let mut lines = reader.lines();
                                    while let Ok(Some(line)) = lines.next_line().await {
                                        push_to_sink(&sink, format!("[tor] {}", line));
                                    }
                                });
                            }
                            if let Some(stderr) = child_stderr {
                                let sink = log_sink.clone();
                                tokio::spawn(async move {
                                    use tokio::io::{AsyncBufReadExt, BufReader};
                                    let reader = BufReader::new(tokio::process::ChildStderr::from_std(stderr).unwrap());
                                    let mut lines = reader.lines();
                                    while let Ok(Some(line)) = lines.next_line().await {
                                        push_to_sink(&sink, format!("[tor] {}", line));
                                    }
                                });
                            }

                            let mut mgr = manager.0.lock().await;
                            let info = mgr.finish_start_tor(uuid, child).map_err(map_err)?;
                            let notif_settings = settings.0.lock().await.notifications.clone();
                            notify(&app, &notif_settings, "tor", "Tor started", &format!("Tor proxy '{}' is running on port {}", info.name, info.port));
                            return Ok(info);
                        }
                        Err(e) => {
                            last_error = format!("Warning: could not check Tor status: {}", e);
                            push_to_sink(&log_sink, last_error.clone());
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("Failed to start Tor ({}): {}", tor_path, e);
                    push_to_sink(&log_sink, last_error.clone());
                }
            }
        }

        let msg = format!("Tor failed after {} attempts: {}", MAX_TOR_ATTEMPTS, last_error);
        push_to_sink(&log_sink, msg.clone());
        let mut mgr = manager.0.lock().await;
        mgr.mark_error(uuid, msg.clone());
        return Err(msg);
    }

    let auto_protocol = local_protocol.clone();

    let custom_list = if proxy_list != "default" {
        proxy_lists::find_by_id(&proxy_list).await
    } else {
        None
    };

    let upstream_result: Result<(Proxy, u64), String> = match mode {
        ProxyMode::Manual => manual_upstream
            .ok_or_else(|| "Manual mode requires an upstream proxy".to_string())
            .map(|p| (p, 0u64)),
        ProxyMode::Auto => {
            tokio::select! {
                _ = discovery_token.cancelled() => {
                    Err("Discovery cancelled".to_string())
                }
                result = ProxyManager::auto_discover_upstream(concurrency, log_sink, auto_protocol, custom_list) => {
                    result.map(|pws| (pws.proxy, pws.latency.as_millis() as u64)).map_err(map_err)
                }
            }
        }
        ProxyMode::Tor => unreachable!(),
    };

    let mut mgr = manager.0.lock().await;
    match upstream_result {
        Ok((proxy, latency_ms)) => {
            let info = mgr.finish_start(uuid, proxy.clone(), latency_ms).map_err(map_err)?;

            if kill_switch.0.is_active() {
                tracing::info!("Proxy started — deactivating kill-switch");
                let _ = kill_switch.0.deactivate();
            }

            let notif_settings = settings.0.lock().await.notifications.clone();
            notify(
                &app,
                &notif_settings,
                "proxy_start",
                "Proxy started",
                &format!("'{}' is running on {}:{}", info.name, info.bind_addr, info.port),
            );

            let anon_arc = mgr.get_anonymity_arc(uuid);
            if let Some(arc) = anon_arc {
                let proxy_clone = proxy.clone();
                tokio::spawn(async move {
                    if let Some(level) = anonymity_check::check_anonymity_safe(&proxy_clone).await
                    {
                        *arc.write() = Some(level);
                        tracing::info!("Anonymity check for {}: {:?}", proxy_clone, arc.read());
                    }
                });
            }

            Ok(info)
        }
        Err(e) => {
            mgr.mark_error(uuid, e.clone());

            if kill_switch.0.is_enabled() {
                let has_running = mgr.get_all().iter().any(|i| i.status == ProxyStatusInfo::Running);
                if !has_running {
                    tracing::info!("Proxy failed and no running proxies — auto-activating kill-switch");
                    let _ = kill_switch.0.activate();
                }
            }

            let notif_settings = settings.0.lock().await.notifications.clone();
            notify(
                &app,
                &notif_settings,
                "proxy_error",
                "Proxy error",
                &format!("Failed to start proxy: {}", e),
            );

            Err(e)
        }
    }
}

#[tauri::command]
pub async fn stop_instance(
    app: tauri::AppHandle,
    manager: State<'_, ProxyManagerState>,
    settings: State<'_, SettingsState>,
    kill_switch: State<'_, KillSwitchStateWrapper>,
    id: String,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mut mgr = manager.0.lock().await;
    let info = mgr.stop_instance(uuid).await.map_err(map_err)?;

    if kill_switch.0.is_enabled() {
        let has_running = mgr.get_all().iter().any(|i| i.status == ProxyStatusInfo::Running);
        if !has_running {
            tracing::info!("No running proxies — auto-activating kill-switch");
            let _ = kill_switch.0.activate();
        }
    }

    let notif_settings = settings.0.lock().await.notifications.clone();
    notify(
        &app,
        &notif_settings,
        "proxy_stop",
        "Proxy stopped",
        &format!("'{}' has been stopped", info.name),
    );

    Ok(info)
}

#[tauri::command]
pub async fn delete_instance(
    manager: State<'_, ProxyManagerState>,
    id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mut mgr = manager.0.lock().await;
    mgr.delete_instance(uuid).await.map_err(map_err)?;
    mgr.save_instances().await;
    Ok(())
}

#[tauri::command]
pub async fn rename_instance(
    manager: State<'_, ProxyManagerState>,
    id: String,
    name: String,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mut mgr = manager.0.lock().await;
    let info = mgr.rename_instance(uuid, name).map_err(map_err)?;
    mgr.save_instances().await;
    Ok(info)
}

#[tauri::command]
pub async fn get_instance_logs(
    manager: State<'_, ProxyManagerState>,
    id: String,
) -> Result<Vec<String>, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mgr = manager.0.lock().await;
    mgr.get_instance_logs(uuid).map_err(map_err)
}

#[tauri::command]
pub async fn toggle_auto_start_on_boot(
    manager: State<'_, ProxyManagerState>,
    id: String,
    enabled: bool,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mut mgr = manager.0.lock().await;
    let info = mgr.toggle_auto_start_on_boot(uuid, enabled).map_err(map_err)?;
    mgr.save_instances().await;
    Ok(info)
}

/// Fetch and test available proxies (standalone, not tied to an instance).
#[tauri::command]
pub async fn fetch_proxies(
    settings: State<'_, SettingsState>,
    protocol: Option<ProxyProtocol>,
) -> Result<Vec<ProxyWithSpeed>, String> {
    let concurrency = {
        let s = settings.0.lock().await;
        s.concurrency
    };

    let proto = protocol.unwrap_or(ProxyProtocol::Socks5);

    tracing::info!("[*] Загрузка и тестирование {:?} прокси…", proto);

    let cached = proxy_cache::load_cache().await.unwrap_or_default();
    let cached_filtered: Vec<_> = cached.into_iter().filter(|p| p.protocol == proto).collect();

    let mut tested = if !cached_filtered.is_empty() {
        speed_test::test_proxies_parallel(cached_filtered.clone(), concurrency).await
    } else {
        Vec::new()
    };

    let new_proxies = sources::fetch_proxies(proto.clone(), None)
        .await
        .map_err(map_err)?;

    let cached_set: std::collections::HashSet<_> = cached_filtered.into_iter().collect();
    let new_unique: Vec<_> = new_proxies
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter(|p| !cached_set.contains(p))
        .collect();

    if !new_unique.is_empty() {
        let new_tested = speed_test::test_proxies_parallel(new_unique, concurrency).await;
        tested.extend(new_tested);
    }

    tested.sort_by_key(|p| p.latency);

    let working: Vec<_> = tested.iter().map(|p| p.proxy.clone()).collect();
    let _ = proxy_cache::save_cache(&working).await;

    Ok(tested)
}

#[tauri::command]
pub async fn check_proxies_live(
    app: tauri::AppHandle,
    settings: State<'_, SettingsState>,
    protocol: Option<ProxyProtocol>,
) -> Result<Vec<ProxyWithSpeed>, String> {
    use futures::stream::{self, StreamExt};
    use serde::Serialize;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tauri::Emitter;

    #[derive(Clone, Serialize)]
    struct CheckerProgress {
        tested: usize,
        working: usize,
        failed: usize,
        total: usize,
        phase: String,
    }

    #[derive(Clone, Serialize)]
    struct CheckerLog {
        message: String,
        level: String,
    }

    let emit_log = |msg: String, level: &str| {
        let _ = app.emit(
            "checker-log",
            CheckerLog { message: msg, level: level.to_string() },
        );
    };

    let concurrency = {
        let s = settings.0.lock().await;
        s.concurrency
    };

    let proto = protocol.unwrap_or(ProxyProtocol::Socks5);

    emit_log(format!("Loading {:?} proxies from cache...", proto), "info");

    let cached = proxy_cache::load_cache().await.unwrap_or_default();
    let cached_filtered: Vec<_> = cached.into_iter().filter(|p| p.protocol == proto).collect();
    let cached_count = cached_filtered.len();

    emit_log(format!("Found {} cached {:?} proxies", cached_count, proto), "info");

    // Phase 1: test cached proxies
    let tested_total = Arc::new(AtomicUsize::new(0));
    let working_total = Arc::new(AtomicUsize::new(0));
    let failed_total = Arc::new(AtomicUsize::new(0));

    let mut all_working: Vec<ProxyWithSpeed> = Vec::new();

    if !cached_filtered.is_empty() {
        emit_log(format!("Testing {} cached proxies ({} threads)...", cached_count, concurrency), "info");

        let total_to_test = cached_count;
        let tc = tested_total.clone();
        let wc = working_total.clone();
        let fc = failed_total.clone();
        let app2 = app.clone();

        let progress_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let t = tc.load(Ordering::Relaxed);
                let w = wc.load(Ordering::Relaxed);
                let f = fc.load(Ordering::Relaxed);
                let _ = app2.emit("checker-progress", CheckerProgress {
                    tested: t,
                    working: w,
                    failed: f,
                    total: total_to_test,
                    phase: "cache".to_string(),
                });
            }
        });

        let tc2 = tested_total.clone();
        let wc2 = working_total.clone();
        let fc2 = failed_total.clone();

        let results: Vec<Option<ProxyWithSpeed>> = stream::iter(cached_filtered.clone())
            .map(move |proxy| {
                let tc = tc2.clone();
                let wc = wc2.clone();
                let fc = fc2.clone();
                async move {
                    let latency = speed_test::test_proxy(&proxy).await;
                    tc.fetch_add(1, Ordering::Relaxed);
                    if let Some(lat) = latency {
                        wc.fetch_add(1, Ordering::Relaxed);
                        Some(ProxyWithSpeed { proxy, latency: lat })
                    } else {
                        fc.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        progress_handle.abort();

        let cached_working: Vec<_> = results.into_iter().flatten().collect();
        let cw = cached_working.len();
        let cf = cached_count - cw;
        emit_log(format!("Cache check done: {} working, {} failed", cw, cf), "success");
        all_working.extend(cached_working);
    }

    // Phase 2: fetch from sources
    emit_log(format!("Fetching new {:?} proxies from sources...", proto), "info");

    let new_proxies = sources::fetch_proxies(proto.clone(), None)
        .await
        .map_err(map_err)?;

    let cached_set: std::collections::HashSet<_> = cached_filtered.into_iter().collect();
    let new_unique: Vec<_> = new_proxies
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter(|p| !cached_set.contains(p))
        .collect();

    let new_count = new_unique.len();
    emit_log(format!("Found {} new unique proxies from sources", new_count), "info");

    if !new_unique.is_empty() {
        emit_log(format!("Testing {} new proxies...", new_count), "info");

        let prev_working = working_total.load(Ordering::Relaxed);
        let prev_failed = failed_total.load(Ordering::Relaxed);
        let tc_new = Arc::new(AtomicUsize::new(0));
        let wc_new = Arc::new(AtomicUsize::new(0));
        let fc_new = Arc::new(AtomicUsize::new(0));

        let tc3 = tc_new.clone();
        let wc3 = wc_new.clone();
        let fc3 = fc_new.clone();
        let app3 = app.clone();

        let progress_handle2 = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let t = tc3.load(Ordering::Relaxed);
                let w = wc3.load(Ordering::Relaxed);
                let f = fc3.load(Ordering::Relaxed);
                let _ = app3.emit("checker-progress", CheckerProgress {
                    tested: cached_count + t,
                    working: prev_working + w,
                    failed: prev_failed + f,
                    total: cached_count + new_count,
                    phase: "sources".to_string(),
                });
            }
        });

        let tc4 = tc_new.clone();
        let wc4 = wc_new.clone();
        let fc4 = fc_new.clone();

        let new_results: Vec<Option<ProxyWithSpeed>> = stream::iter(new_unique)
            .map(move |proxy| {
                let tc = tc4.clone();
                let wc = wc4.clone();
                let fc = fc4.clone();
                async move {
                    let latency = speed_test::test_proxy(&proxy).await;
                    tc.fetch_add(1, Ordering::Relaxed);
                    if let Some(lat) = latency {
                        wc.fetch_add(1, Ordering::Relaxed);
                        Some(ProxyWithSpeed { proxy, latency: lat })
                    } else {
                        fc.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        progress_handle2.abort();

        let new_working: Vec<_> = new_results.into_iter().flatten().collect();
        emit_log(format!("Sources check done: {} working, {} failed", new_working.len(), new_count - new_working.len()), "success");
        all_working.extend(new_working);
    }

    all_working.sort_by_key(|p| p.latency);

    let working_proxies: Vec<_> = all_working.iter().map(|p| p.proxy.clone()).collect();
    let _ = proxy_cache::save_cache(&working_proxies).await;

    emit_log(format!("Total working: {} proxies", all_working.len()), "success");
    if let Some(fastest) = all_working.first() {
        emit_log(
            format!("Fastest: {}:{} ({}ms)", fastest.proxy.host, fastest.proxy.port, fastest.latency.as_millis()),
            "success",
        );
    }

    let grand_total = cached_count + new_count;
    let _ = app.emit("checker-progress", CheckerProgress {
        tested: grand_total,
        working: all_working.len(),
        failed: grand_total.saturating_sub(all_working.len()),
        total: grand_total,
        phase: "done".to_string(),
    });

    Ok(all_working)
}

#[tauri::command]
pub async fn get_proxy_cache_stats() -> Result<proxy_cache::ProxyCacheStats, String> {
    Ok(proxy_cache::load_cache_stats().await)
}

#[tauri::command]
pub async fn get_proxy_lists() -> Result<Vec<ProxyListConfig>, String> {
    Ok(proxy_lists::load_all().await)
}

#[tauri::command]
pub async fn save_proxy_list(config: ProxyListConfig) -> Result<Vec<ProxyListConfig>, String> {
    let mut lists = proxy_lists::load_all().await;
    if let Some(existing) = lists.iter_mut().find(|l| l.id == config.id) {
        *existing = config;
    } else {
        lists.push(config);
    }
    proxy_lists::save_all(&lists).await.map_err(map_err)?;
    Ok(lists)
}

#[tauri::command]
pub async fn delete_proxy_list(id: String) -> Result<Vec<ProxyListConfig>, String> {
    let mut lists = proxy_lists::load_all().await;
    lists.retain(|l| l.id != id);
    proxy_lists::save_all(&lists).await.map_err(map_err)?;
    Ok(lists)
}

#[tauri::command]
pub async fn update_instance_proxy_list(
    manager: State<'_, ProxyManagerState>,
    id: String,
    proxy_list: String,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mut mgr = manager.0.lock().await;
    let info = mgr.update_proxy_list(uuid, proxy_list).map_err(map_err)?;
    mgr.save_instances().await;
    Ok(info)
}

#[tauri::command]
pub async fn refresh_proxy_list(
    settings: State<'_, SettingsState>,
    id: String,
) -> Result<proxy_cache::ProxyCacheStats, String> {
    let concurrency = {
        let s = settings.0.lock().await;
        s.concurrency
    };

    let config = proxy_lists::find_by_id(&id)
        .await
        .ok_or_else(|| format!("Proxy list '{}' not found", id))?;

    let proxies = proxy_lists::fetch_from_config(&config, ProxyProtocol::Socks5, None).await;

    if proxies.is_empty() {
        return Err("No proxies found in this list".to_string());
    }

    let tested = speed_test::test_proxies_parallel(proxies.clone(), concurrency).await;

    let working: Vec<_> = tested.iter().map(|p| p.proxy.clone()).collect();
    let mut merged = proxy_cache::load_cache().await.unwrap_or_default();
    let new_hosts: std::collections::HashSet<_> = working.iter().map(|p| (&p.host, p.port)).collect();
    merged.retain(|p| !new_hosts.contains(&(&p.host, p.port)));
    merged.extend(working);
    proxy_cache::save_cache(&merged).await.map_err(map_err)?;

    Ok(proxy_cache::load_cache_stats().await)
}

#[tauri::command]
pub async fn toggle_auto_rotate(
    manager: State<'_, ProxyManagerState>,
    id: String,
    enabled: bool,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mut mgr = manager.0.lock().await;
    let info = mgr.toggle_auto_rotate(uuid, enabled).map_err(map_err)?;
    mgr.save_instances().await;
    Ok(info)
}

const AUTO_ROTATE_MINUTES_MIN: u64 = 1;
const AUTO_ROTATE_MINUTES_MAX: u64 = 1440;

#[tauri::command]
pub async fn update_auto_rotate_minutes(
    manager: State<'_, ProxyManagerState>,
    id: String,
    minutes: u64,
) -> Result<ProxyInstanceInfo, String> {
    let minutes = minutes.clamp(AUTO_ROTATE_MINUTES_MIN, AUTO_ROTATE_MINUTES_MAX);
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mut mgr = manager.0.lock().await;
    let info = mgr.update_auto_rotate_minutes(uuid, minutes).map_err(map_err)?;
    mgr.save_instances().await;
    Ok(info)
}

/// Test the current upstream proxy connection and return latency in ms.
#[tauri::command]
pub async fn test_connection(
    manager: State<'_, ProxyManagerState>,
    id: String,
) -> Result<u64, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;

    let proxy = {
        let mgr = manager.0.lock().await;
        let info = mgr
            .get_instance(uuid)
            .ok_or_else(|| format!("Instance {} not found", id))?;
        info.upstream
            .ok_or_else(|| "No upstream proxy configured".to_string())?
    };

    let latency = speed_test::test_proxy(&proxy)
        .await
        .ok_or_else(|| "Connection test failed".to_string())?;

    Ok(latency.as_millis() as u64)
}

#[tauri::command]
pub async fn change_ip(
    manager: State<'_, ProxyManagerState>,
    id: String,
) -> Result<ProxyInstanceInfo, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;

    // Phase 1: get context (short lock)
    let (upstream_arc, current_proxy, protocol, log_sink, upstream_latency_arc, concurrency) = {
        let mgr = manager.0.lock().await;
        let (arc, current, proto, sink, lat_arc) = mgr.get_change_ip_context(uuid).map_err(map_err)?;
        (arc, current, proto, sink, lat_arc, mgr.default_concurrency)
    };

    // Phase 2: load and test cached proxies (no lock held)
    let cached = proxy_cache::load_cache().await.unwrap_or_default();
    let filtered: Vec<_> = cached
        .into_iter()
        .filter(|p| p.protocol == protocol)
        .collect();

    if filtered.is_empty() {
        return Err("No cached proxies available".to_string());
    }

    push_to_sink(
        &log_sink,
        format!("Change IP: testing {} cached proxies…", filtered.len()),
    );

    let tested = speed_test::test_proxies_parallel(filtered, concurrency).await;

    let mut candidates: Vec<_> = tested
        .into_iter()
        .filter(|p| {
            current_proxy
                .as_ref()
                .map_or(true, |c| c.host != p.proxy.host || c.port != p.proxy.port)
        })
        .collect();

    if candidates.is_empty() {
        return Err("No alternative working proxies available".to_string());
    }

    candidates.sort_by_key(|p| p.latency);
    let fastest = candidates.into_iter().next().unwrap();

    push_to_sink(
        &log_sink,
        format!(
            "Change IP: switched to {}://{}:{} ({}ms)",
            fastest.proxy.protocol,
            fastest.proxy.host,
            fastest.proxy.port,
            fastest.latency.as_millis()
        ),
    );

    use std::sync::atomic::Ordering;
    upstream_latency_arc.store(fastest.latency.as_millis() as u64, Ordering::Relaxed);

    // Phase 3: update upstream via shared Arc (no manager lock needed)
    *upstream_arc.write() = Some(fastest.proxy);

    let mgr = manager.0.lock().await;
    mgr.get_instance(uuid)
        .ok_or_else(|| "Instance not found".to_string())
}

#[tauri::command]
pub async fn get_settings(
    settings: State<'_, SettingsState>,
) -> Result<AppSettings, String> {
    let s = settings.0.lock().await;
    Ok(s.clone())
}

#[tauri::command]
pub async fn update_settings(
    settings: State<'_, SettingsState>,
    kill_switch: State<'_, KillSwitchStateWrapper>,
    new_settings: AppSettings,
) -> Result<(), String> {
    kill_switch.0.set_enabled(new_settings.kill_switch.enabled);

    if !new_settings.kill_switch.enabled && kill_switch.0.is_active() {
        let _ = kill_switch.0.deactivate();
    }

    let mut s = settings.0.lock().await;
    let mut validated = new_settings;
    validated.concurrency = validated.concurrency.clamp(1, 1000);
    *s = validated;
    s.save().await.map_err(map_err)
}

#[tauri::command]
pub async fn check_proxy_anonymity(
    manager: State<'_, ProxyManagerState>,
    id: String,
) -> Result<Option<AnonymityLevel>, String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;

    let (proxy, anon_arc) = {
        let mgr = manager.0.lock().await;
        let info = mgr
            .get_instance(uuid)
            .ok_or_else(|| format!("Instance {} not found", id))?;
        let proxy = info
            .upstream
            .ok_or_else(|| "No upstream proxy configured".to_string())?;
        let arc = mgr
            .get_anonymity_arc(uuid)
            .ok_or_else(|| "Instance not found".to_string())?;
        (proxy, arc)
    };

    let level = anonymity_check::check_anonymity_safe(&proxy).await;
    *anon_arc.write() = level;

    Ok(level)
}

#[tauri::command]
pub async fn check_ip_leak(
    manager: State<'_, ProxyManagerState>,
    id: Option<String>,
) -> Result<leak_test::IpLeakResult, String> {
    let proxy = if let Some(id) = id {
        let uuid = Uuid::parse_str(&id).map_err(map_err)?;
        let mgr = manager.0.lock().await;
        let info = mgr
            .get_instance(uuid)
            .ok_or_else(|| format!("Instance {} not found", id))?;
        info.upstream
    } else {
        None
    };

    Ok(leak_test::check_ip_leak(proxy.as_ref()).await)
}

#[tauri::command]
pub async fn check_dns_leak() -> Result<leak_test::DnsLeakResult, String> {
    Ok(leak_test::check_dns_leak().await)
}

#[tauri::command]
pub async fn run_full_leak_test(
    manager: State<'_, ProxyManagerState>,
    id: Option<String>,
) -> Result<LeakTestResult, String> {
    let proxy = if let Some(id) = id {
        let uuid = Uuid::parse_str(&id).map_err(map_err)?;
        let mgr = manager.0.lock().await;
        let info = mgr
            .get_instance(uuid)
            .ok_or_else(|| format!("Instance {} not found", id))?;
        info.upstream
    } else {
        None
    };

    Ok(leak_test::run_full_leak_test(proxy.as_ref()).await)
}

#[tauri::command]
pub async fn get_real_ip() -> Result<String, String> {
    leak_test::get_real_ip().await.map_err(map_err)
}

#[tauri::command]
pub async fn activate_kill_switch(
    kill_switch: State<'_, KillSwitchStateWrapper>,
    manager: State<'_, ProxyManagerState>,
) -> Result<(), String> {
    let (ports, hosts) = {
        let mgr = manager.0.lock().await;
        mgr.get_running_kill_switch_context()
    };

    let mut ips: std::collections::HashSet<String> = std::collections::HashSet::new();
    for host in hosts {
        let Ok(addrs) = tokio::net::lookup_host(format!("{}:0", host)).await else {
            tracing::warn!("Kill-switch: could not resolve host {}", host);
            continue;
        };
        for addr in addrs {
            ips.insert(addr.ip().to_string());
        }
    }
    let upstream_ips: Vec<String> = ips.into_iter().collect();

    kill_switch.0.set_allowed_ports(ports);
    kill_switch.0.set_allowed_upstream_ips(upstream_ips);
    kill_switch.0.activate().map_err(map_err)
}

#[tauri::command]
pub async fn deactivate_kill_switch(
    kill_switch: State<'_, KillSwitchStateWrapper>,
) -> Result<(), String> {
    kill_switch.0.deactivate().map_err(map_err)
}

#[tauri::command]
pub async fn get_kill_switch_status(
    kill_switch: State<'_, KillSwitchStateWrapper>,
) -> Result<crate::kill_switch::KillSwitchConfig, String> {
    Ok(kill_switch.0.get_config())
}

/// Returns recovery instruction if kill-switch was left active (e.g. after crash). UI can show this when activating.
#[tauri::command]
pub async fn get_kill_switch_recovery_instruction() -> String {
    crate::kill_switch::KILLSWITCH_RECOVERY_INSTRUCTION.to_string()
}

#[tauri::command]
pub async fn toggle_kill_switch_enabled(
    kill_switch: State<'_, KillSwitchStateWrapper>,
    enabled: bool,
) -> Result<(), String> {
    kill_switch.0.set_enabled(enabled);
    if !enabled && kill_switch.0.is_active() {
        kill_switch.0.deactivate().map_err(map_err)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_tls_fingerprint_hash(
    settings: State<'_, SettingsState>,
) -> Result<String, String> {
    let s = settings.0.lock().await;
    Ok(tls_fingerprint::compute_fingerprint_hash(&s.tls_fingerprint))
}

// ── Plugin Manager commands ───────────────────────────────────────────────────

#[tauri::command]
pub async fn get_plugins(
    plugin_mgr: State<'_, PluginManagerState>,
) -> Result<Vec<PluginInfo>, String> {
    let mgr = plugin_mgr.0.lock().await;
    Ok(mgr.get_plugins())
}

#[tauri::command]
pub async fn install_plugin(
    plugin_mgr: State<'_, PluginManagerState>,
    settings: State<'_, SettingsState>,
    id: String,
) -> Result<(), String> {
    {
        let mut mgr = plugin_mgr.0.lock().await;
        mgr.install_plugin(&id).map_err(map_err)?;
    }
    
    let mut s = settings.0.lock().await;
    *s = crate::settings::AppSettings::load().await;
    Ok(())
}

#[tauri::command]
pub async fn uninstall_plugin(
    plugin_mgr: State<'_, PluginManagerState>,
    settings: State<'_, SettingsState>,
    id: String,
) -> Result<(), String> {
    {
        let mut mgr = plugin_mgr.0.lock().await;
        mgr.uninstall_plugin(&id).map_err(map_err)?;
    }
    
    let mut s = settings.0.lock().await;
    *s = crate::settings::AppSettings::load().await;
    Ok(())
}

#[tauri::command]
pub async fn enable_plugin(
    plugin_mgr: State<'_, PluginManagerState>,
    id: String,
) -> Result<(), String> {
    let mut mgr = plugin_mgr.0.lock().await;
    mgr.enable_plugin(&id).map_err(map_err)
}

#[tauri::command]
pub async fn disable_plugin(
    plugin_mgr: State<'_, PluginManagerState>,
    id: String,
) -> Result<(), String> {
    let mut mgr = plugin_mgr.0.lock().await;
    mgr.disable_plugin(&id).map_err(map_err)
}

#[tauri::command]
pub async fn get_plugin_settings_schema(
    plugin_mgr: State<'_, PluginManagerState>,
    id: String,
) -> Result<Option<serde_json::Value>, String> {
    let mgr = plugin_mgr.0.lock().await;
    mgr.get_plugin_settings_schema(&id).map_err(map_err)
}

#[tauri::command]
pub async fn open_plugins_folder(
    plugin_mgr: State<'_, PluginManagerState>,
) -> Result<(), String> {
    let plugins_dir = {
        let mgr = plugin_mgr.0.lock().await;
        mgr.context().plugins_dir.clone()
    };
    std::fs::create_dir_all(&plugins_dir).map_err(map_err)?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(plugins_dir.as_os_str())
            .spawn()
            .map_err(map_err)?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(plugins_dir.as_os_str())
            .spawn()
            .map_err(map_err)?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(plugins_dir.as_os_str())
            .spawn()
            .map_err(map_err)?;
    }
    Ok(())
}

#[derive(Clone, Serialize)]
pub struct CountryInfoDto {
    pub country_code: String,
    pub country_name: Option<String>,
}

#[tauri::command]
pub async fn lookup_country(ip: String) -> Result<Option<CountryInfoDto>, String> {
    Ok(geoip::lookup_country(&ip).map(|c| CountryInfoDto {
        country_code: c.country_code,
        country_name: c.country_name,
    }))
}

#[tauri::command]
pub async fn lookup_host_country(host: String) -> Result<Option<CountryInfoDto>, String> {
    Ok(geoip::lookup_host_country(&host).await.map(|c| CountryInfoDto {
        country_code: c.country_code,
        country_name: c.country_name,
    }))
}

#[derive(Clone, Serialize)]
pub struct SystemProxyInfoDto {
    pub enabled: bool,
    pub server: Option<String>,
}

#[tauri::command]
pub async fn get_system_proxy_status() -> Result<SystemProxyInfoDto, String> {
    let status = system_proxy::get_system_proxy_status().map_err(map_err)?;
    Ok(SystemProxyInfoDto {
        enabled: status.enabled,
        server: status.server,
    })
}

#[tauri::command]
pub async fn set_as_system_proxy(
    manager: State<'_, ProxyManagerState>,
    id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mgr = manager.0.lock().await;
    let instance = mgr
        .get_instance(uuid)
        .ok_or_else(|| format!("Instance {} not found", id))?;
    system_proxy::set_system_proxy(&instance.bind_addr, instance.port).map_err(map_err)
}

#[tauri::command]
pub async fn unset_system_proxy() -> Result<(), String> {
    system_proxy::unset_system_proxy().map_err(map_err)
}

// ── Profile commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_profiles() -> Result<Vec<Profile>, String> {
    Ok(profiles::list_profiles().await)
}

#[tauri::command]
pub async fn save_profile(req: SaveProfileRequest) -> Result<Profile, String> {
    profiles::upsert_profile(req).await.map_err(map_err)
}

#[tauri::command]
pub async fn delete_profile(id: String) -> Result<(), String> {
    profiles::delete_profile(&id).await.map_err(map_err)
}

#[tauri::command]
pub async fn load_profile(
    settings: State<'_, SettingsState>,
    kill_switch: State<'_, KillSwitchStateWrapper>,
    id: String,
) -> Result<AppSettings, String> {
    let profile = profiles::get_profile(&id)
        .await
        .ok_or_else(|| format!("Profile {} not found", id))?;

    kill_switch.0.set_enabled(profile.settings.kill_switch.enabled);
    if !profile.settings.kill_switch.enabled && kill_switch.0.is_active() {
        let _ = kill_switch.0.deactivate();
    }

    let mut s = settings.0.lock().await;
    let mut validated = profile.settings.clone();
    validated.concurrency = validated.concurrency.clamp(1, 1000);
    *s = validated.clone();
    s.save().await.map_err(map_err)?;
    Ok(validated)
}

// ── Split Tunnel commands ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_split_tunnel_rules() -> Result<Vec<RoutingRule>, String> {
    Ok(split_tunnel::list_rules().await)
}

#[tauri::command]
pub async fn save_split_tunnel_rule(req: SaveRoutingRuleRequest) -> Result<RoutingRule, String> {
    split_tunnel::upsert_rule(req).await.map_err(map_err)
}

#[tauri::command]
pub async fn delete_split_tunnel_rule(id: String) -> Result<(), String> {
    split_tunnel::delete_rule(&id).await.map_err(map_err)
}

// ── Notification commands ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_notification_settings(
    settings: State<'_, SettingsState>,
) -> Result<NotificationSettings, String> {
    let s = settings.0.lock().await;
    Ok(s.notifications.clone())
}

#[tauri::command]
pub async fn update_notification_settings(
    settings: State<'_, SettingsState>,
    app: tauri::AppHandle,
    notif: NotificationSettings,
) -> Result<(), String> {
    let mut s = settings.0.lock().await;
    s.notifications = notif;
    s.save().await.map_err(map_err)?;
    let _ = app;
    Ok(())
}

// ── Scheduler commands ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_schedules() -> Result<Vec<Schedule>, String> {
    Ok(scheduler::list_schedules().await)
}

#[tauri::command]
pub async fn save_schedule(req: SaveScheduleRequest) -> Result<Schedule, String> {
    scheduler::upsert_schedule(req).await.map_err(map_err)
}

#[tauri::command]
pub async fn delete_schedule(id: String) -> Result<(), String> {
    scheduler::delete_schedule(&id).await.map_err(map_err)
}

// ── Import / Export commands ──────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ExportResult {
    pub path: String,
}

#[tauri::command]
pub async fn export_config(
    manager: State<'_, ProxyManagerState>,
    settings: State<'_, SettingsState>,
) -> Result<ExportResult, String> {
    let current_settings = settings.0.lock().await.clone();
    let instances = {
        let mgr = manager.0.lock().await;
        mgr.get_all_saved()
    };
    let profiles = profiles::list_profiles().await;
    let split_tunnel_rules = split_tunnel::list_rules().await;
    let schedules = scheduler::list_schedules().await;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let bundle = import_export::ExportBundle {
        version: 1,
        exported_at: now,
        settings: current_settings,
        instances,
        profiles,
        split_tunnel_rules,
        schedules,
    };

    let default_path = import_export::default_export_path();
    let path_str = default_path.to_string_lossy().to_string();
    import_export::save_bundle(&path_str, &bundle)
        .await
        .map_err(map_err)?;

    Ok(ExportResult { path: path_str })
}

/// Result returned by [`import_config`].  The import always succeeds if no
/// parse/IO error occurs, but may carry non-fatal warnings that the frontend
/// should surface to the user.
#[derive(Clone, Serialize)]
pub struct ImportResult {
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn import_config(
    settings: State<'_, SettingsState>,
    kill_switch: State<'_, KillSwitchStateWrapper>,
    manager: State<'_, ProxyManagerState>,
    json: String,
) -> Result<ImportResult, String> {
    let bundle = serde_json::from_str::<import_export::ExportBundle>(&json).map_err(map_err)?;
    let mut warnings: Vec<String> = Vec::new();

    if bundle.settings.kill_switch.enabled {
        let no_running_proxies = {
            let mgr = manager.0.lock().await;
            !mgr.get_all().iter().any(|i| i.status == ProxyStatusInfo::Running)
        };
        if no_running_proxies {
            warnings.push(
                "This configuration enables the kill-switch. \
                 If no proxy is running when the kill-switch activates, \
                 ALL internet traffic will be blocked until a proxy is started \
                 or the kill-switch is manually deactivated."
                    .to_string(),
            );
        }
    }

    {
        let mut s = settings.0.lock().await;
        let mut validated = bundle.settings;
        validated.concurrency = validated.concurrency.clamp(1, 1000);

        if let Some(ref path_str) = validated.tor_config.binary_path.clone() {
            if !path_str.is_empty() {
                let relay_data_dir = dirs::config_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("relay");
                let candidate = std::path::Path::new(path_str);
                let allowed = candidate
                    .canonicalize()
                    .ok()
                    .map(|canon| canon.starts_with(&relay_data_dir))
                    .unwrap_or(false);
                if !allowed {
                    warnings.push(format!(
                        "The imported configuration contained a custom Tor binary path ('{}') \
                         that is outside the Relay data directory. \
                         The path has been cleared for security. \
                         Use the built-in Tor Downloader plugin to install Tor.",
                        path_str
                    ));
                    validated.tor_config.binary_path = None;
                }
            }
        }

        kill_switch.0.set_enabled(validated.kill_switch.enabled);
        if !validated.kill_switch.enabled && kill_switch.0.is_active() {
            let _ = kill_switch.0.deactivate();
        }
        *s = validated;
        s.save().await.map_err(map_err)?;
    }

    if !bundle.profiles.is_empty() {
        profiles::save_profiles(&bundle.profiles)
            .await
            .map_err(map_err)?;
    }

    if !bundle.split_tunnel_rules.is_empty() {
        split_tunnel::save_rules(&bundle.split_tunnel_rules)
            .await
            .map_err(map_err)?;
    }

    if !bundle.schedules.is_empty() {
        scheduler::save_all(&bundle.schedules)
            .await
            .map_err(map_err)?;
    }

    if !bundle.instances.is_empty() {
        warnings.push(format!(
            "The export contained {} proxy instance(s) which were not imported. \
             Proxy instances must be recreated manually after import.",
            bundle.instances.len()
        ));
    }

    Ok(ImportResult { warnings })
}

#[tauri::command]
pub async fn would_stop_trigger_kill_switch(
    manager: State<'_, ProxyManagerState>,
    kill_switch: State<'_, KillSwitchStateWrapper>,
    id: String,
) -> Result<bool, String> {
    if !kill_switch.0.is_enabled() {
        return Ok(false);
    }
    let uuid = Uuid::parse_str(&id).map_err(map_err)?;
    let mgr = manager.0.lock().await;
    let uuid_str = uuid.to_string();
    let would_have_running = mgr
        .get_all()
        .iter()
        .any(|i| i.status == ProxyStatusInfo::Running && i.id != uuid_str);
    Ok(!would_have_running)
}

#[derive(Clone, Serialize)]
pub struct ProxyBandwidthDto {
    pub id: String,
    pub name: String,
    pub total_bytes: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
}

#[derive(Clone, Serialize)]
pub struct BandwidthStatsDto {
    pub total_bytes: u64,
    pub total_requests: u64,
    pub per_proxy: Vec<ProxyBandwidthDto>,
}

#[tauri::command]
pub async fn get_bandwidth_stats(
    manager: State<'_, ProxyManagerState>,
) -> Result<BandwidthStatsDto, String> {
    let instances = {
        let mgr = manager.0.lock().await;
        mgr.get_all()
    };

    let mut total_bytes: u64 = 0;
    let mut total_requests: u64 = 0;
    let mut per_proxy = Vec::new();

    for inst in &instances {
        let s = &inst.stats;
        total_bytes += s.total_bytes;
        total_requests += s.total_requests;
        per_proxy.push(ProxyBandwidthDto {
            id: inst.id.clone(),
            name: inst.name.clone(),
            total_bytes: s.total_bytes,
            total_requests: s.total_requests,
            successful_requests: s.successful_requests,
            avg_latency_ms: s.avg_latency_ms,
            success_rate: s.success_rate,
        });
    }

    // Sort by bytes descending
    per_proxy.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));

    Ok(BandwidthStatsDto {
        total_bytes,
        total_requests,
        per_proxy,
    })
}

// ── Notification send helpers (called from other modules) ─────────────────────

pub fn notify(
    app: &tauri::AppHandle,
    settings: &NotificationSettings,
    key: &str,
    title: &str,
    body: &str,
) {
    notifications::send(app, settings, key, title, body);
}
