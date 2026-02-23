use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// DoH server presets.
pub const DOH_CLOUDFLARE: &str = "https://cloudflare-dns.com/dns-query";
pub const DOH_GOOGLE: &str = "https://dns.google/dns-query";
pub const DOH_QUAD9: &str = "https://dns.quad9.net:5053/dns-query";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResolverConfig {
    pub enabled: bool,
    pub primary_server: String,
    pub fallback_servers: Vec<String>,
}

impl Default for DnsResolverConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            primary_server: DOH_CLOUDFLARE.to_string(),
            fallback_servers: vec![DOH_GOOGLE.to_string()],
        }
    }
}

/// Global DoH resolver state shared across the application.
pub struct DohResolver {
    config: Arc<RwLock<DnsResolverConfig>>,
    client: reqwest::Client,
}

impl DohResolver {
    pub fn new(config: DnsResolverConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            config: Arc::new(RwLock::new(config)),
            client,
        }
    }

    pub async fn update_config(&self, config: DnsResolverConfig) {
        *self.config.write().await = config;
    }

    pub async fn is_enabled(&self) -> bool {
        self.config.read().await.enabled
    }

    /// Resolve a hostname to IP addresses using DNS-over-HTTPS (JSON API).
    pub async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        let config = self.config.read().await.clone();
        if !config.enabled {
            return Err(anyhow!("DoH resolver is disabled"));
        }

        // Try primary server first, then fallbacks.
        let mut servers = vec![config.primary_server.clone()];
        servers.extend(config.fallback_servers.iter().cloned());

        let mut last_error = anyhow!("No DoH servers configured");

        for server in &servers {
            match self.resolve_with_server(server, hostname).await {
                Ok(addrs) if !addrs.is_empty() => return Ok(addrs),
                Ok(_) => {
                    last_error = anyhow!("No addresses returned from {}", server);
                }
                Err(e) => {
                    tracing::debug!("DoH query to {} failed: {}", server, e);
                    last_error = e;
                }
            }
        }

        Err(last_error)
    }

    /// Perform a DoH query using the JSON API (RFC 8484 compatible).
    async fn resolve_with_server(&self, server: &str, hostname: &str) -> Result<Vec<IpAddr>> {
        let resp = self
            .client
            .get(server)
            .header("Accept", "application/dns-json")
            .query(&[("name", hostname), ("type", "A")])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("DoH server returned status {}", resp.status()));
        }

        let body: serde_json::Value = resp.json().await?;

        let answers = body
            .get("Answer")
            .and_then(|a| a.as_array())
            .map(|arr| arr.to_vec())
            .unwrap_or_default();

        let mut addrs = Vec::new();
        for answer in answers {
            // Type 1 = A record, Type 28 = AAAA record
            let rtype = answer.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
            if rtype == 1 || rtype == 28 {
                if let Some(data) = answer.get("data").and_then(|d| d.as_str()) {
                    if let Ok(addr) = data.parse::<IpAddr>() {
                        addrs.push(addr);
                    }
                }
            }
        }

        Ok(addrs)
    }

    /// Resolve a hostname to the first available IP address.
    pub async fn resolve_first(&self, hostname: &str) -> Result<IpAddr> {
        let addrs = self.resolve(hostname).await?;
        addrs
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No addresses found for {}", hostname))
    }
}

/// Check which DNS server is being used by making a query and inspecting the resolver.
pub async fn detect_dns_servers() -> Vec<String> {
    let mut servers = Vec::new();

    let test_services = [
        "https://1.1.1.1/cdn-cgi/trace",
        "https://dns.google/resolve?name=example.com&type=A",
    ];

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    for service in &test_services {
        if let Ok(resp) = client.get(*service).send().await {
            if let Ok(body) = resp.text().await {
                if body.contains("cloudflare") || body.contains("1.1.1.1") {
                    servers.push("1.1.1.1 (Cloudflare)".to_string());
                } else if body.contains("google") || body.contains("8.8.8.8") {
                    servers.push("8.8.8.8 (Google)".to_string());
                }
            }
        }
    }

    if servers.is_empty() {
        servers.push("System DNS".to_string());
    }

    servers
}
