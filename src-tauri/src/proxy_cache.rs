use crate::proxy_type::{Proxy, ProxyProtocol};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCacheStats {
    pub total: usize,
    pub socks5: usize,
    pub socks4: usize,
    pub http: usize,
    pub last_updated: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyCache {
    pub proxies: Vec<Proxy>,
    pub last_updated: u64,
}

impl ProxyCache {
    pub fn new(proxies: Vec<Proxy>) -> Self {
        Self {
            proxies,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

fn get_cache_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("relay")
        .join("proxy_cache.json")
}

pub async fn save_cache(proxies: &[Proxy]) -> Result<()> {
    let cache = ProxyCache::new(proxies.to_vec());
    let json = serde_json::to_string_pretty(&cache)?;
    let path = get_cache_path();

    fs::write(path, json).await?;

    Ok(())
}

pub async fn load_cache() -> Result<Vec<Proxy>> {
    let path = get_cache_path();

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).await?;
    let cache: ProxyCache = serde_json::from_str(&content)?;

    let age_hours = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - cache.last_updated)
        / 3600;

    if age_hours > 24 {
        tracing::info!(
            "Кэш устарел (возраст: {} часов), будет обновлён",
            age_hours
        );
        return Ok(Vec::new());
    }

    tracing::info!(
        "Загружено {} прокси из кэша (возраст: {} часов)",
        cache.proxies.len(),
        age_hours
    );
    Ok(cache.proxies)
}

pub async fn load_cache_stats() -> ProxyCacheStats {
    let path = get_cache_path();
    if !path.exists() {
        return ProxyCacheStats {
            total: 0,
            socks5: 0,
            socks4: 0,
            http: 0,
            last_updated: 0,
        };
    }

    let content = match fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => {
            return ProxyCacheStats {
                total: 0,
                socks5: 0,
                socks4: 0,
                http: 0,
                last_updated: 0,
            }
        }
    };

    let cache: ProxyCache = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => {
            return ProxyCacheStats {
                total: 0,
                socks5: 0,
                socks4: 0,
                http: 0,
                last_updated: 0,
            }
        }
    };

    let socks5 = cache
        .proxies
        .iter()
        .filter(|p| p.protocol == ProxyProtocol::Socks5)
        .count();
    let socks4 = cache
        .proxies
        .iter()
        .filter(|p| p.protocol == ProxyProtocol::Socks4)
        .count();
    let http = cache
        .proxies
        .iter()
        .filter(|p| p.protocol == ProxyProtocol::Http || p.protocol == ProxyProtocol::Https)
        .count();

    ProxyCacheStats {
        total: cache.proxies.len(),
        socks5,
        socks4,
        http,
        last_updated: cache.last_updated,
    }
}
