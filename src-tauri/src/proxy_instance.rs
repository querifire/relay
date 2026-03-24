use crate::anonymity_check::AnonymityLevel;
use crate::cred_encrypt;
use crate::geoip::CountryInfo;
use crate::proxy_chain::ProxyChainConfig;
use crate::proxy_type::{Proxy, ProxyMode, ProxyProtocol};
use parking_lot::{Mutex as SyncMutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_LOGS: usize = 500;

pub fn push_to_sink(sink: &LogSink, msg: impl Into<String>) {
    let mut logs = sink.lock();
    logs.push_back(msg.into());
    while logs.len() > MAX_LOGS {
        logs.pop_front();
    }
}

pub type LogSink = Arc<SyncMutex<VecDeque<String>>>;
pub type ConnectionSink = Arc<SyncMutex<VecDeque<ConnectionLogEntry>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLogEntry {
    pub timestamp_ms: u64,
    pub target_host: String,
    pub protocol: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
}

pub fn push_connection_entry(sink: &ConnectionSink, entry: ConnectionLogEntry) {
    let mut logs = sink.lock();
    logs.push_back(entry);
    while logs.len() > MAX_LOGS {
        logs.pop_front();
    }
}

#[derive(Debug)]
pub struct ProxyStats {
    pub total_requests: AtomicU64,
    pub successful_requests: AtomicU64,
    pub total_latency_ms: AtomicU64,
    pub total_bytes: AtomicU64,
    
    pub last_request_latency_ms: AtomicU64,
}

impl ProxyStats {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            last_request_latency_ms: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.total_latency_ms.store(0, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.last_request_latency_ms.store(0, Ordering::Relaxed);
    }

    pub fn to_info(&self) -> ProxyStatsInfo {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let total_lat = self.total_latency_ms.load(Ordering::Relaxed);
        let avg_latency_ms = if successful > 0 {
            total_lat / successful
        } else {
            0
        };
        let success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            0.0
        };
        ProxyStatsInfo {
            total_requests: total,
            successful_requests: successful,
            avg_latency_ms,
            success_rate,
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            last_request_latency_ms: self.last_request_latency_ms.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatsInfo {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
    pub total_bytes: u64,
    
    pub last_request_latency_ms: u64,
}

#[derive(Debug)]
pub enum ProxyStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyStatusInfo {
    Stopped,
    Starting,
    Running,
    Error(String),
}

impl From<&ProxyStatus> for ProxyStatusInfo {
    fn from(status: &ProxyStatus) -> Self {
        match status {
            ProxyStatus::Stopped => ProxyStatusInfo::Stopped,
            ProxyStatus::Starting => ProxyStatusInfo::Starting,
            ProxyStatus::Running => ProxyStatusInfo::Running,
            ProxyStatus::Error(msg) => ProxyStatusInfo::Error(msg.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInstanceInfo {
    pub id: String,
    pub name: String,
    pub bind_addr: String,
    pub port: u16,
    pub mode: ProxyMode,
    pub status: ProxyStatusInfo,
    pub upstream: Option<Proxy>,
    pub local_protocol: ProxyProtocol,
    
    pub has_auth: bool,
    pub auto_rotate: bool,
    pub auto_rotate_minutes: Option<u64>,
    pub proxy_list: String,
    pub stats: ProxyStatsInfo,
    
    pub upstream_latency_ms: u64,
    
    pub auto_start_on_boot: bool,
    
    pub anonymity_level: Option<AnonymityLevel>,
    
    pub proxy_chain: Option<ProxyChainConfig>,

    /// GeoIP for current upstream (filled by API layer, not persisted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_country: Option<CountryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedInstance {
    pub id: String,
    pub name: String,
    pub bind_addr: String,
    pub port: u16,
    pub mode: ProxyMode,
    #[serde(default = "default_local_protocol")]
    pub local_protocol: ProxyProtocol,
    #[serde(default)]
    pub auth_username: Option<String>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_password_encrypted: Option<String>,
    
    #[serde(default, alias = "auth_password")]
    pub auth_password_legacy: Option<String>,
    #[serde(default)]
    pub auto_rotate: bool,
    #[serde(default)]
    pub auto_rotate_minutes: Option<u64>,
    #[serde(default = "default_proxy_list")]
    pub proxy_list: String,
    
    #[serde(default = "default_created_at")]
    pub created_at: u64,
    #[serde(default)]
    pub auto_start_on_boot: bool,
    
    #[serde(default)]
    pub proxy_chain: Option<ProxyChainConfig>,
}

fn default_created_at() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn default_local_protocol() -> ProxyProtocol {
    ProxyProtocol::Socks5
}

fn default_proxy_list() -> String {
    "default".to_string()
}

#[derive(Debug)]
pub struct ProxyInstance {
    pub id: Uuid,
    pub name: String,
    pub bind_addr: String,
    pub port: u16,
    pub mode: ProxyMode,
    pub status: ProxyStatus,
    pub local_protocol: ProxyProtocol,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
    pub auto_rotate: bool,
    pub auto_rotate_minutes: Option<u64>,
    pub proxy_list: String,
    pub created_at: u64,
    pub auto_start_on_boot: bool,
    pub anonymity_level: Arc<RwLock<Option<AnonymityLevel>>>,
    pub proxy_chain: Option<ProxyChainConfig>,
    pub upstream: Arc<RwLock<Option<Proxy>>>,
    pub upstream_latency_ms: Arc<AtomicU64>,
    pub cancel_token: Option<CancellationToken>,
    pub rotation_token: Option<CancellationToken>,
    pub discovery_token: Option<CancellationToken>,
    pub handle: Option<JoinHandle<anyhow::Result<()>>>,
    pub logs: LogSink,
    pub connection_logs: ConnectionSink,
    pub stats: Arc<ProxyStats>,
}

impl ProxyInstance {
    pub fn new(
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
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            bind_addr,
            port,
            mode,
            local_protocol,
            auth_username,
            auth_password,
            auto_rotate,
            auto_rotate_minutes,
            proxy_list,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            auto_start_on_boot,
            anonymity_level: Arc::new(RwLock::new(None)),
            proxy_chain,
            status: ProxyStatus::Stopped,
            upstream: Arc::new(RwLock::new(None)),
            upstream_latency_ms: Arc::new(AtomicU64::new(0)),
            cancel_token: None,
            rotation_token: None,
            discovery_token: None,
            handle: None,
            logs: Arc::new(SyncMutex::new(VecDeque::new())),
            connection_logs: Arc::new(SyncMutex::new(VecDeque::new())),
            stats: Arc::new(ProxyStats::new()),
        }
    }

    pub fn from_saved(saved: SavedInstance) -> Self {
        let auth_password = saved
            .auth_password_encrypted
            .as_ref()
            .and_then(|e| cred_encrypt::decrypt_password(e).ok())
            .or(saved.auth_password_legacy);
        let id = Uuid::parse_str(&saved.id).unwrap_or_else(|_| Uuid::new_v4());
        Self {
            id,
            name: saved.name,
            bind_addr: saved.bind_addr,
            port: saved.port,
            mode: saved.mode,
            local_protocol: saved.local_protocol,
            auth_username: saved.auth_username,
            auth_password,
            auto_rotate: saved.auto_rotate,
            auto_rotate_minutes: saved.auto_rotate_minutes,
            proxy_list: saved.proxy_list,
            created_at: saved.created_at,
            auto_start_on_boot: saved.auto_start_on_boot,
            anonymity_level: Arc::new(RwLock::new(None)),
            proxy_chain: saved.proxy_chain,
            status: ProxyStatus::Stopped,
            upstream: Arc::new(RwLock::new(None)),
            upstream_latency_ms: Arc::new(AtomicU64::new(0)),
            cancel_token: None,
            rotation_token: None,
            discovery_token: None,
            handle: None,
            logs: Arc::new(SyncMutex::new(VecDeque::new())),
            connection_logs: Arc::new(SyncMutex::new(VecDeque::new())),
            stats: Arc::new(ProxyStats::new()),
        }
    }

    pub fn to_info(&self) -> ProxyInstanceInfo {
        let has_auth = self.auth_username.is_some() && self.auth_password.is_some();
        ProxyInstanceInfo {
            id: self.id.to_string(),
            name: self.name.clone(),
            bind_addr: self.bind_addr.clone(),
            port: self.port,
            mode: self.mode.clone(),
            status: ProxyStatusInfo::from(&self.status),
            upstream: self.upstream.read().clone(),
            local_protocol: self.local_protocol.clone(),
            has_auth,
            auto_rotate: self.auto_rotate,
            auto_rotate_minutes: self.auto_rotate_minutes,
            proxy_list: self.proxy_list.clone(),
            stats: self.stats.to_info(),
            upstream_latency_ms: self.upstream_latency_ms.load(Ordering::Relaxed),
            auto_start_on_boot: self.auto_start_on_boot,
            anonymity_level: self.anonymity_level.read().clone(),
            proxy_chain: self.proxy_chain.clone(),
            upstream_country: None,
        }
    }

    pub fn to_saved(&self) -> SavedInstance {
        let auth_password_encrypted = self
            .auth_password
            .as_ref()
            .and_then(|p| cred_encrypt::encrypt_password(p).ok());
        SavedInstance {
            id: self.id.to_string(),
            name: self.name.clone(),
            bind_addr: self.bind_addr.clone(),
            port: self.port,
            mode: self.mode.clone(),
            local_protocol: self.local_protocol.clone(),
            auth_username: self.auth_username.clone(),
            auth_password_encrypted,
            auth_password_legacy: None,
            auto_rotate: self.auto_rotate,
            auto_rotate_minutes: self.auto_rotate_minutes,
            proxy_list: self.proxy_list.clone(),
            created_at: self.created_at,
            auto_start_on_boot: self.auto_start_on_boot,
            proxy_chain: self.proxy_chain.clone(),
        }
    }

    pub fn push_log(&self, msg: String) {
        let mut logs = self.logs.lock();
        logs.push_back(msg);
        while logs.len() > MAX_LOGS {
            logs.pop_front();
        }
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.lock().iter().cloned().collect()
    }

    pub fn get_connection_logs(&self, limit: Option<usize>) -> Vec<ConnectionLogEntry> {
        let guard = self.connection_logs.lock();
        let mut v: Vec<ConnectionLogEntry> = guard.iter().rev().cloned().collect();
        if let Some(l) = limit {
            v.truncate(l);
        }
        v
    }

    pub fn clear_connection_logs(&self) {
        self.connection_logs.lock().clear();
    }
}
