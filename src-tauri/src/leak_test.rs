use crate::proxy_type::Proxy;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpLeakResult {
    pub real_ip: Option<String>,
    pub proxy_ip: Option<String>,
    pub leak_detected: bool,
    pub proxy_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsLeakResult {
    pub dns_servers: Vec<String>,
    pub leak_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakTestResult {
    pub ip: IpLeakResult,
    pub dns: DnsLeakResult,
}

const IP_CHECK_SERVICES: &[&str] = &[
    "https://api.ipify.org?format=text",
    "https://icanhazip.com",
    "https://ifconfig.me/ip",
];

pub async fn get_real_ip() -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .no_proxy()
        .build()?;

    for service in IP_CHECK_SERVICES {
        match client.get(*service).send().await {
            Ok(resp) => {
                if let Ok(text) = resp.text().await {
                    let ip = text.trim().to_string();
                    if !ip.is_empty() && ip.len() < 50 {
                        return Ok(ip);
                    }
                }
            }
            Err(e) => {
                tracing::debug!("IP check service {} failed: {}", service, e);
            }
        }
    }

    Err(anyhow::anyhow!("Failed to determine real IP"))
}

pub async fn get_proxy_ip(proxy: &Proxy) -> Result<String> {
    let proxy_url = match proxy.protocol {
        crate::proxy_type::ProxyProtocol::Http | crate::proxy_type::ProxyProtocol::Https => {
            format!("http://{}:{}", proxy.host, proxy.port)
        }
        crate::proxy_type::ProxyProtocol::Socks5 | crate::proxy_type::ProxyProtocol::Tor => {
            format!("socks5://{}:{}", proxy.host, proxy.port)
        }
        _ => return Err(anyhow::anyhow!("Unsupported proxy protocol for IP check")),
    };

    let reqwest_proxy = reqwest::Proxy::all(&proxy_url)?;
    let client = reqwest::Client::builder()
        .proxy(reqwest_proxy)
        .timeout(Duration::from_secs(15))
        .build()?;

    for service in IP_CHECK_SERVICES {
        match client.get(*service).send().await {
            Ok(resp) => {
                if let Ok(text) = resp.text().await {
                    let ip = text.trim().to_string();
                    if !ip.is_empty() && ip.len() < 50 {
                        return Ok(ip);
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Proxy IP check via {} failed: {}", service, e);
            }
        }
    }

    Err(anyhow::anyhow!("Failed to determine proxy IP"))
}

pub async fn check_ip_leak(proxy: Option<&Proxy>) -> IpLeakResult {
    let real_ip = get_real_ip().await.ok();

    let (proxy_ip, proxy_desc) = match proxy {
        Some(p) => {
            let ip = get_proxy_ip(p).await.ok();
            let desc = Some(format!("{}://{}:{}", p.protocol, p.host, p.port));
            (ip, desc)
        }
        None => (None, None),
    };

    let leak_detected = match (&real_ip, &proxy_ip) {
        (Some(real), Some(proxied)) => real == proxied,
        (Some(_), None) => true,
        _ => false,
    };

    IpLeakResult {
        real_ip,
        proxy_ip,
        leak_detected,
        proxy_used: proxy_desc,
    }
}

pub async fn check_dns_leak() -> DnsLeakResult {
    let dns_servers = crate::dns_resolver::detect_dns_servers().await;

    let leak_detected = dns_servers.iter().any(|s| s.contains("System DNS"));

    DnsLeakResult {
        dns_servers,
        leak_detected,
    }
}

pub async fn run_full_leak_test(proxy: Option<&Proxy>) -> LeakTestResult {
    let (ip_result, dns_result) = tokio::join!(check_ip_leak(proxy), check_dns_leak());

    LeakTestResult {
        ip: ip_result,
        dns: dns_result,
    }
}
