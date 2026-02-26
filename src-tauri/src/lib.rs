mod anonymity_check;
mod atomic_write;
mod builtin_plugins;
mod commands;
mod cred_encrypt;
mod dns_resolver;
mod geoip;
mod import_export;
mod kill_switch;
mod leak_test;
mod local_proxy;
mod notifications;
mod plugin_manager;
mod plugin_sdk;
mod port_kill;
mod profiles;
mod proxy_cache;
mod proxy_chain;
mod proxy_instance;
mod proxy_lists;
mod proxy_manager;
mod proxy_type;
mod scheduler;
mod settings;
mod sources;
mod speed_test;
mod split_tunnel;
mod system_proxy;
mod tls_fingerprint;
mod upstream;

use commands::{KillSwitchStateWrapper, PluginManagerState, ProxyManagerState, SettingsState};
use plugin_manager::PluginManager;
use proxy_manager::ProxyManager;
use settings::AppSettings;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

async fn refresh_tray_menu(app: &tauri::AppHandle) {
    use crate::proxy_instance::ProxyStatusInfo;
    use tauri::menu::{MenuBuilder, MenuItem};
    use tauri::Manager;

    let instances = {
        let state: tauri::State<'_, ProxyManagerState> = app.state();
        let mgr = state.0.lock().await;
        mgr.get_all()
    };

    let running: Vec<_> = instances
        .iter()
        .filter(|i| matches!(i.status, ProxyStatusInfo::Running | ProxyStatusInfo::Starting))
        .collect();

    let tooltip = if running.is_empty() {
        "Relay — No active proxies".to_string()
    } else {
        let list: Vec<String> = running
            .iter()
            .map(|i| format!("{}:{}", i.bind_addr, i.port))
            .collect();
        format!("Relay — {} active: {}", running.len(), list.join(", "))
    };

    let Some(tray) = app.tray_by_id("relay-tray") else {
        return;
    };

    let _ = tray.set_tooltip(Some(&tooltip));

    let Ok(show_item) = MenuItem::with_id(app, "show", "Open Relay", true, None::<&str>) else {
        return;
    };
    let Ok(sep) = tauri::menu::PredefinedMenuItem::separator(app) else {
        return;
    };
    let Ok(quit_item) = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>) else {
        return;
    };

    let mut builder = MenuBuilder::new(app).item(&show_item);

    if !running.is_empty() {
        if let Ok(inst_sep) = tauri::menu::PredefinedMenuItem::separator(app) {
            builder = builder.item(&inst_sep);
        }
        for inst in &running {
            let label = format!(
                "● {} — {}:{} ({}ms)",
                inst.name,
                inst.bind_addr,
                inst.port,
                inst.upstream_latency_ms
            );
            if let Ok(item) = MenuItem::with_id(
                app,
                &format!("proxy-{}", inst.id),
                &label,
                false,
                None::<&str>,
            ) {
                builder = builder.item(&item);
            }
        }
    }

    let Ok(menu) = builder.item(&sep).item(&quit_item).build() else {
        return;
    };

    let _ = tray.set_menu(Some(menu));
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_target(false)
        .with_level(false)
        .without_time()
        .init();

    let settings = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build temp runtime")
        .block_on(AppSettings::load());

    let concurrency = settings.concurrency;

    let manager = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build temp runtime")
        .block_on(async {
            let mut mgr = ProxyManager::new(concurrency);
            mgr.load_instances().await;
            mgr
        });

    let plugin_manager = PluginManager::new().unwrap_or_else(|e| {
        tracing::warn!("[plugin] Failed to initialize plugin manager: {}", e);
        PluginManager::new_empty()
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .manage(ProxyManagerState(Mutex::new(manager)))
        .manage(PluginManagerState(Mutex::new(plugin_manager)))
        .manage(SettingsState(Mutex::new(settings)))
        .manage(KillSwitchStateWrapper(kill_switch::KillSwitchState::new()))
        .setup(|app| {
            use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
            use tauri::menu::{MenuBuilder, MenuItem};
            use tauri::Manager;

            // Clean up orphaned kill-switch firewall rules from previous runs (recovery).
            kill_switch::cleanup_orphaned_rules();

            let show_item = MenuItem::with_id(app, "show", "Open Relay", true, None::<&str>)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = MenuBuilder::new(app)
                .items(&[&show_item, &separator, &quit_item])
                .build()?;

            let _tray = TrayIconBuilder::with_id("relay-tray")
                .icon(tauri::include_image!("icons/32x32.png"))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Relay Proxy Manager")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            {
                let launched_hidden = std::env::args().any(|a| a == "--hidden");
                let settings_state: tauri::State<'_, SettingsState> = app.state();
                let start_hidden = {
                    let s = settings_state.0.blocking_lock();
                    s.start_hidden
                };

                if launched_hidden && start_hidden {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                        tracing::info!("Started in hidden mode (tray only)");
                    }
                }
            }

            let app_handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app_handle.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                });
            }

            let app_for_tray = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    refresh_tray_menu(&app_for_tray).await;
                }
            });

            {
                let app_for_sched = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        let due = match scheduler::take_due_schedules().await {
                            Ok(d) => d,
                            Err(e) => {
                                tracing::warn!("[scheduler] {}", e);
                                continue;
                            }
                        };
                        for sched in due {
                            tracing::info!(
                                "[scheduler] Running '{}' ({:?})",
                                sched.name,
                                sched.action
                            );
                            match &sched.action {
                                scheduler::ScheduleAction::StartInstance { instance_id } => {
                                    let mgr: tauri::State<'_, ProxyManagerState> =
                                        app_for_sched.state();
                                    let sett: tauri::State<'_, SettingsState> =
                                        app_for_sched.state();
                                    let ks: tauri::State<'_, KillSwitchStateWrapper> =
                                        app_for_sched.state();
                                    if let Err(e) = commands::start_instance(
                                        app_for_sched.clone(),
                                        mgr,
                                        sett,
                                        ks,
                                        instance_id.clone(),
                                        None,
                                        None,
                                        None,
                                    )
                                    .await
                                    {
                                        tracing::warn!(
                                            "[scheduler] start_instance failed: {}",
                                            e
                                        );
                                    }
                                }
                                scheduler::ScheduleAction::StopInstance { instance_id } => {
                                    let mgr: tauri::State<'_, ProxyManagerState> =
                                        app_for_sched.state();
                                    let sett: tauri::State<'_, SettingsState> =
                                        app_for_sched.state();
                                    let ks: tauri::State<'_, KillSwitchStateWrapper> =
                                        app_for_sched.state();
                                    if let Err(e) =
                                        commands::stop_instance(app_for_sched.clone(), mgr, sett, ks, instance_id.clone())
                                            .await
                                    {
                                        tracing::warn!(
                                            "[scheduler] stop_instance failed: {}",
                                            e
                                        );
                                    }
                                }
                                scheduler::ScheduleAction::ChangeIp { instance_id } => {
                                    let mgr: tauri::State<'_, ProxyManagerState> =
                                        app_for_sched.state();
                                    if let Err(e) =
                                        commands::change_ip(mgr, instance_id.clone()).await
                                    {
                                        tracing::warn!(
                                            "[scheduler] change_ip failed: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                });
            }

            {
                let app_handle2 = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                    let auto_start_ids = {
                        let state: tauri::State<'_, ProxyManagerState> = app_handle2.state();
                        let mgr = state.0.lock().await;
                        mgr.get_auto_start_ids()
                    };

                    for id in auto_start_ids {
                        let id_str = id.to_string();
                        tracing::info!("[*] Auto-starting instance {}", id_str);
                        let mgr_state: tauri::State<'_, ProxyManagerState> = app_handle2.state();
                        let settings_s: tauri::State<'_, SettingsState> = app_handle2.state();
                        let ks_state: tauri::State<'_, KillSwitchStateWrapper> = app_handle2.state();
                        if let Err(e) = commands::start_instance(
                            app_handle2.clone(),
                            mgr_state,
                            settings_s,
                            ks_state,
                            id_str.clone(),
                            None,
                            None,
                            None,
                        ).await {
                            tracing::warn!("[!] Failed to auto-start {}: {}", id_str, e);
                        }
                        refresh_tray_menu(&app_handle2).await;
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_instances,
            commands::get_instance,
            commands::create_instance,
            commands::start_instance,
            commands::stop_instance,
            commands::delete_instance,
            commands::rename_instance,
            commands::get_instance_logs,
            commands::toggle_auto_rotate,
            commands::update_auto_rotate_minutes,
            commands::test_connection,
            commands::change_ip,
            commands::fetch_proxies,
            commands::check_proxies_live,
            commands::get_proxy_cache_stats,
            commands::get_proxy_lists,
            commands::save_proxy_list,
            commands::delete_proxy_list,
            commands::update_instance_proxy_list,
            commands::get_settings,
            commands::update_settings,
            commands::toggle_auto_start_on_boot,
            commands::refresh_proxy_list,
            commands::check_proxy_anonymity,
            commands::check_ip_leak,
            commands::check_dns_leak,
            commands::run_full_leak_test,
            commands::get_real_ip,
            commands::activate_kill_switch,
            commands::deactivate_kill_switch,
            commands::get_kill_switch_status,
            commands::get_kill_switch_recovery_instruction,
            commands::toggle_kill_switch_enabled,
            commands::get_tls_fingerprint_hash,
            commands::get_plugins,
            commands::install_plugin,
            commands::uninstall_plugin,
            commands::enable_plugin,
            commands::disable_plugin,
            commands::get_plugin_settings_schema,
            commands::open_plugins_folder,
            commands::lookup_country,
            commands::lookup_host_country,
            commands::get_system_proxy_status,
            commands::set_as_system_proxy,
            commands::unset_system_proxy,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::load_profile,
            commands::list_split_tunnel_rules,
            commands::save_split_tunnel_rule,
            commands::delete_split_tunnel_rule,
            commands::get_notification_settings,
            commands::update_notification_settings,
            commands::list_schedules,
            commands::save_schedule,
            commands::delete_schedule,
            commands::export_config,
            commands::import_config,
            commands::would_stop_trigger_kill_switch,
            commands::get_bandwidth_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
