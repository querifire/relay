use crate::proxy_instance::{push_to_sink, LogSink};
use crate::proxy_type::{Proxy, ProxyProtocol};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyListConfig {
    pub id: String,
    pub name: String,
    
    pub urls: Vec<String>,
    
    pub inline_proxies: Vec<String>,
}

fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join("relay").join("proxy_lists.json")
}

pub async fn load_all() -> Vec<ProxyListConfig> {
    let path = config_path();
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub async fn save_all(lists: &[ProxyListConfig]) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(lists)?;
    crate::atomic_write::atomic_write_async(&path, &json).await?;
    Ok(())
}

pub async fn find_by_id(id: &str) -> Option<ProxyListConfig> {
    load_all().await.into_iter().find(|l| l.id == id)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            !a.is_loopback() && !a.is_private() && !a.is_link_local() && !a.is_unspecified()
        }
        IpAddr::V6(a) => {
            !a.is_loopback()
                && !a.is_unspecified()
                
                && (a.segments()[0] & 0xfe00) != 0xfc00
        }
    }
}

fn is_proxy_list_url_allowed(url_str: &str) -> Result<bool, String> {
    let url = url::Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Ok(false);
    }
    let host = url.host_str().ok_or("URL has no host")?;
    let host_lower = host.to_lowercase();
    if host_lower == "localhost" || host_lower == "::1" {
        return Ok(false);
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn resolve_and_validate_url(
    url_str: &str,
) -> Result<Option<Vec<SocketAddr>>, String> {
    let url = url::Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;
    let host = url.host_str().ok_or("URL has no host")?;

    if host.parse::<IpAddr>().is_ok() {
        return Ok(None);
    }

    let default_port: u16 = if url.scheme() == "https" { 443 } else { 80 };
    let port = url.port().unwrap_or(default_port);

    let addrs: Vec<SocketAddr> = tokio::net::lookup_host(format!("{}:{}", host, port))
        .await
        .map_err(|e| format!("DNS resolution failed for '{}': {}", host, e))?
        .collect();

    if addrs.is_empty() {
        return Err(format!("DNS resolution returned no addresses for '{}'", host));
    }

    for addr in &addrs {
        if !is_public_ip(addr.ip()) {
            return Err(format!(
                "DNS rebinding protection: '{}' resolves to a private/local IP address ({}). \
                 Request blocked.",
                host,
                addr.ip()
            ));
        }
    }

    Ok(Some(addrs))
}

pub async fn fetch_from_config(
    config: &ProxyListConfig,
    protocol: ProxyProtocol,
    log_sink: Option<&LogSink>,
) -> Vec<Proxy> {
    let mut all = Vec::new();

    if !config.inline_proxies.is_empty() {
        let mut count = 0u32;
        for line in &config.inline_proxies {
            if let Some(proxy) = parse_proxy_line(line, &protocol) {
                all.push(proxy);
                count += 1;
            }
        }
        if count > 0 {
            let msg = format!("  {} (inline) -> {} proxies", config.name, count);
            tracing::info!("{}", msg);
            if let Some(sink) = log_sink {
                push_to_sink(sink, &msg);
            }
        }
    }

    if !config.urls.is_empty() {
        for url in &config.urls {
            
            match is_proxy_list_url_allowed(url) {
                Ok(false) => {
                    let msg = format!("  {} -> URL blocked (localhost/private/metadata): {}", config.name, url);
                    tracing::warn!("{}", msg);
                    if let Some(sink) = log_sink {
                        push_to_sink(sink, &msg);
                    }
                    continue;
                }
                Err(e) => {
                    let msg = format!("  {} -> URL invalid: {} ({})", config.name, url, e);
                    tracing::debug!("{}", msg);
                    if let Some(sink) = log_sink {
                        push_to_sink(sink, &msg);
                    }
                    continue;
                }
                Ok(true) => {}
            }

            let resolved_addrs = match resolve_and_validate_url(url).await {
                Ok(addrs) => addrs,
                Err(e) => {
                    let msg = format!("  {} -> DNS blocked: {}", config.name, e);
                    tracing::warn!("{}", msg);
                    if let Some(sink) = log_sink {
                        push_to_sink(sink, &msg);
                    }
                    continue;
                }
            };

            let parsed = match url::Url::parse(url) {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!("URL parse error {}: {}", url, e);
                    continue;
                }
            };
            let host = match parsed.host_str() {
                Some(h) => h.to_string(),
                None => continue,
            };
            let default_port: u16 = if parsed.scheme() == "https" { 443 } else { 80 };
            let port = parsed.port().unwrap_or(default_port);

            let mut client_builder = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

            if let Some(addrs) = resolved_addrs {
                
                let socket_addrs: Vec<SocketAddr> = addrs
                    .into_iter()
                    .map(|a| SocketAddr::new(a.ip(), port))
                    .collect();
                client_builder = client_builder.resolve_to_addrs(&host, &socket_addrs);
            }

            let client = match client_builder.build() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to build HTTP client for {}: {}", url, e);
                    continue;
                }
            };

            match client.get(url).send().await {
                Ok(response) => match response.text().await {
                    Ok(text) => {
                        let proxies = parse_proxy_text(&text, &protocol);
                        let msg =
                            format!("  {} -> {} proxies", config.name, proxies.len());
                        tracing::info!("{}", msg);
                        if let Some(sink) = log_sink {
                            push_to_sink(sink, &msg);
                        }
                        all.extend(proxies);
                    }
                    Err(e) => {
                        let msg = format!("  {} -> read error: {}", config.name, e);
                        tracing::debug!("{}", msg);
                        if let Some(sink) = log_sink {
                            push_to_sink(sink, &msg);
                        }
                    }
                },
                Err(e) => {
                    let msg = format!("  {} -> fetch error: {}", config.name, e);
                    tracing::debug!("{}", msg);
                    if let Some(sink) = log_sink {
                        push_to_sink(sink, &msg);
                    }
                }
            }
        }
    }

    all
}

fn parse_proxy_text(text: &str, default_protocol: &ProxyProtocol) -> Vec<Proxy> {
    text.lines()
        .filter_map(|line| parse_proxy_line(line, default_protocol))
        .collect()
}

fn parse_proxy_line(line: &str, default_protocol: &ProxyProtocol) -> Option<Proxy> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let (protocol, rest) = if let Some(after) = line.strip_prefix("socks5://") {
        (ProxyProtocol::Socks5, after)
    } else if let Some(after) = line.strip_prefix("socks4://") {
        (ProxyProtocol::Socks4, after)
    } else if let Some(after) = line.strip_prefix("http://") {
        (ProxyProtocol::Http, after)
    } else if let Some(after) = line.strip_prefix("https://") {
        (ProxyProtocol::Http, after)
    } else {
        (default_protocol.clone(), line)
    };

    let host_port_part = if let Some(at_pos) = rest.rfind('@') {
        &rest[at_pos + 1..]
    } else {
        rest
    };

    let colon = host_port_part.rfind(':')?;
    let host = host_port_part[..colon].trim().to_string();
    let port = host_port_part[colon + 1..].trim().parse::<u16>().ok()?;

    if host.is_empty() {
        return None;
    }

    Some(Proxy::new(host, port, protocol))
}
