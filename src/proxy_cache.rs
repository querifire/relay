use crate::proxy_type::Proxy;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

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
    let mut path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    
    path.push("autoproxy_cache.json");
    path
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
        tracing::info!("Кэш устарел (возраст: {} часов), будет обновлён", age_hours);
        return Ok(Vec::new());
    }
    
    tracing::info!("Загружено {} прокси из кэша (возраст: {} часов)", cache.proxies.len(), age_hours);
    Ok(cache.proxies)
}
