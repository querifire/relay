use crate::dns_resolver::DnsResolverConfig;
use crate::kill_switch::KillSwitchConfig;
use crate::tls_fingerprint::TlsFingerprintConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BridgeType {
    #[default]
    Obfs4,
    MeekAzure,
    Snowflake,
    WebTunnel,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorConfig {
    pub binary_path: Option<String>,
    pub socks_port: u16,
    pub use_bridges: bool,
    pub bridge_type: BridgeType,
    pub custom_bridges: Vec<String>,
    pub exit_nodes: Option<String>,
    pub entry_nodes: Option<String>,
    pub exclude_nodes: Option<String>,
    pub strict_nodes: bool,
    pub custom_torrc: Option<String>,
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            binary_path: None,
            socks_port: 9050,
            use_bridges: false,
            bridge_type: BridgeType::Obfs4,
            custom_bridges: Vec::new(),
            exit_nodes: None,
            entry_nodes: None,
            exclude_nodes: None,
            strict_nodes: false,
            custom_torrc: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub proxy_start: bool,
    pub proxy_stop: bool,
    pub proxy_error: bool,
    pub ip_changed: bool,
    pub kill_switch: bool,
    pub leak: bool,
    pub tor: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            proxy_start: true,
            proxy_stop: false,
            proxy_error: true,
            ip_changed: false,
            kill_switch: true,
            leak: true,
            tor: false,
        }
    }
}

const BLOCKED_TORRC_DIRECTIVES: &[&str] = &[
    "controlport",
    "controllistenaddress",
    "controlsocket",
    "hashedcontrolpassword",
    "cookieauthentication",
    "cookieauthfile",
    "cookieauthfilegrouplydateable",
    "socks5proxy",
    "socks4proxy",
    "httpsproxy",
    "translistenaddress",
    "transport",
    "dnslistenaddress",
    "dnsport",
    "__owningcontrollerprocess",
    "__discardlogsfromrunningtested",
];

fn validate_custom_torrc_line(line: &str) -> Result<(), String> {
    let lower = line.to_ascii_lowercase();
    let directive = lower.split_whitespace().next().unwrap_or("");
    for &blocked in BLOCKED_TORRC_DIRECTIVES {
        if directive == blocked {
            return Err(format!(
                "Forbidden torrc directive '{}'. \
                 This directive cannot be set via custom_torrc for security reasons.",
                directive
            ));
        }
    }
    Ok(())
}

fn sanitize_torrc_field(value: &str, field_name: &str) -> Result<String, String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!(
            "Invalid value for '{}': newline characters are not allowed in torrc field values.",
            field_name
        ));
    }
    Ok(value.to_string())
}

pub fn generate_torrc(
    config: &TorConfig,
    socks_addr: &str,
    socks_port: u16,
    data_dir: &Path,
) -> Result<String, String> {
    let mut lines = vec![
        format!("SocksPort {}:{}", socks_addr, socks_port),
        format!("DataDirectory {}", data_dir.display()),
    ];

    if config.use_bridges {
        lines.push("UseBridges 1".to_string());
        match config.bridge_type {
            BridgeType::Obfs4 => {
                lines.push(
                    "ClientTransportPlugin obfs4 exec obfs4proxy".to_string(),
                );
            }
            BridgeType::MeekAzure => {
                lines.push(
                    "ClientTransportPlugin meek_lite exec obfs4proxy".to_string(),
                );
            }
            BridgeType::Snowflake => {
                lines.push(
                    "ClientTransportPlugin snowflake exec snowflake-client".to_string(),
                );
            }
            BridgeType::WebTunnel => {
                lines.push(
                    "ClientTransportPlugin webtunnel exec webtunnel-client".to_string(),
                );
            }
            BridgeType::Custom => {}
        }
        for (i, bridge) in config.custom_bridges.iter().enumerate() {
            let trimmed = bridge.trim();
            if !trimmed.is_empty() {
                let safe = sanitize_torrc_field(trimmed, &format!("custom_bridges[{}]", i))?;
                lines.push(format!("Bridge {}", safe));
            }
        }
    }

    if let Some(exit) = &config.exit_nodes {
        let t = exit.trim();
        if !t.is_empty() {
            let safe = sanitize_torrc_field(t, "exit_nodes")?;
            lines.push(format!("ExitNodes {}", safe));
        }
    }
    if let Some(entry) = &config.entry_nodes {
        let t = entry.trim();
        if !t.is_empty() {
            let safe = sanitize_torrc_field(t, "entry_nodes")?;
            lines.push(format!("EntryNodes {}", safe));
        }
    }
    if let Some(exclude) = &config.exclude_nodes {
        let t = exclude.trim();
        if !t.is_empty() {
            let safe = sanitize_torrc_field(t, "exclude_nodes")?;
            lines.push(format!("ExcludeNodes {}", safe));
        }
    }
    if config.strict_nodes {
        lines.push("StrictNodes 1".to_string());
    }
    if let Some(custom) = &config.custom_torrc {
        for raw_line in custom.lines() {
            let t = raw_line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            validate_custom_torrc_line(t)?;
            lines.push(t.to_string());
        }
    }

    Ok(lines.join("\n"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    
    pub theme: Theme,
    
    pub default_port: u16,
    
    pub default_bind: String,
    
    pub concurrency: usize,
    
    pub auto_rotate_minutes: Option<u64>,

    #[serde(default)]
    pub tor_config: TorConfig,

    #[serde(default)]
    pub dns_protection: DnsResolverConfig,
    
    #[serde(default)]
    pub kill_switch: KillSwitchConfig,
    
    #[serde(default)]
    pub tls_fingerprint: TlsFingerprintConfig,

    #[serde(default)]
    pub start_hidden: bool,

    #[serde(default)]
    pub notifications: NotificationSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            default_port: 9051,
            default_bind: "127.0.0.1".into(),
            concurrency: 100,
            auto_rotate_minutes: None,
            tor_config: TorConfig::default(),
            dns_protection: DnsResolverConfig::default(),
            kill_switch: KillSwitchConfig::default(),
            tls_fingerprint: TlsFingerprintConfig::default(),
            start_hidden: false,
            notifications: NotificationSettings::default(),
        }
    }
}

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
