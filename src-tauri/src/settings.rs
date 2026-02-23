use crate::dns_resolver::DnsResolverConfig;
use crate::kill_switch::KillSwitchConfig;
use crate::tls_fingerprint::TlsFingerprintConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// UI theme.
    pub theme: Theme,
    /// Default port for new proxy instances.
    pub default_port: u16,
    /// Default bind address for new proxy instances.
    pub default_bind: String,
    /// Number of concurrent proxy tests.
    pub concurrency: usize,
    /// Auto-rotate upstream proxy every N minutes (None = disabled).
    pub auto_rotate_minutes: Option<u64>,

    /// Path to the Tor binary.
    pub tor_binary_path: Option<String>,
    /// SOCKS port exposed by the Tor process.
    pub tor_socks_port: Option<u16>,

    /// DNS-over-HTTPS protection.
    #[serde(default)]
    pub dns_protection: DnsResolverConfig,
    /// Kill-switch configuration.
    #[serde(default)]
    pub kill_switch: KillSwitchConfig,
    /// TLS fingerprint randomization.
    #[serde(default)]
    pub tls_fingerprint: TlsFingerprintConfig,

    /// Start minimized to tray when launched via autostart.
    #[serde(default)]
    pub start_hidden: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            default_port: 9051,
            default_bind: "127.0.0.1".into(),
            concurrency: 100,
            auto_rotate_minutes: None,
            tor_binary_path: None,
            tor_socks_port: Some(9050),
            dns_protection: DnsResolverConfig::default(),
            kill_switch: KillSwitchConfig::default(),
            tls_fingerprint: TlsFingerprintConfig::default(),
            start_hidden: false,
        }
    }
}

/// Return the path to the settings JSON file.
///
/// Uses the platform-specific config directory (via the `dirs` crate) with a
/// fallback next to the executable.
fn settings_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        });

    base.join("relay").join("settings.json")
}

impl AppSettings {
    /// Load settings from disk or return defaults.
    pub async fn load() -> Self {
        let path = settings_path();
        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path).await {
            Ok(content) => {
                let mut s: Self = serde_json::from_str(&content).unwrap_or_default();
                s.concurrency = s.concurrency.clamp(1, 1000);
                s
            }
            Err(e) => {
                tracing::warn!("Не удалось прочитать настройки: {}", e);
                Self::default()
            }
        }
    }

    /// Persist current settings to disk.
    pub async fn save(&self) -> Result<()> {
        let path = settings_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(self)?;
        crate::atomic_write::atomic_write_async(&path, &json).await?;

        tracing::info!("Настройки сохранены в {:?}", path);
        Ok(())
    }
}
