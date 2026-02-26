use crate::settings::NotificationSettings;
use tauri_plugin_notification::NotificationExt;

fn can_send(settings: &NotificationSettings, key: &str) -> bool {
    match key {
        "proxy_start" => settings.proxy_start,
        "proxy_stop" => settings.proxy_stop,
        "proxy_error" => settings.proxy_error,
        "ip_changed" => settings.ip_changed,
        "kill_switch" => settings.kill_switch,
        "leak" => settings.leak,
        "tor" => settings.tor,
        _ => false,
    }
}

pub fn send(
    app: &tauri::AppHandle,
    settings: &NotificationSettings,
    key: &str,
    title: &str,
    body: &str,
) {
    if !settings.enabled || !can_send(settings, key) {
        return;
    }
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}
