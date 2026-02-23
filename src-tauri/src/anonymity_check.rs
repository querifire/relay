use crate::proxy_type::{Proxy, ProxyProtocol};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnonymityLevel {
    Transparent,
    Anonymous,
    Elite,
}

impl std::fmt::Display for AnonymityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnonymityLevel::Transparent => write!(f, "Transparent"),
            AnonymityLevel::Anonymous => write!(f, "Anonymous"),
            AnonymityLevel::Elite => write!(f, "Elite"),
        }
    }
}

const PROXY_REVEALING_HEADERS: &[&str] = &[
    "x-forwarded-for",
    "x-real-ip",
    "via",
    "proxy-connection",
    "x-proxy-id",
    "forwarded",
    "x-forwarded-proto",
    "x-forwarded-host",
    "x-forwarded-server",
];

/// Check the anonymity level of a proxy by sending a request through it
/// to an HTTP headers echo service and analyzing the response.
pub async fn check_anonymity(proxy: &Proxy) -> Option<AnonymityLevel> {
    let proxy_url = match proxy.protocol {
        ProxyProtocol::Http | ProxyProtocol::Https => {
            format!("http://{}:{}", proxy.host, proxy.port)
        }
        ProxyProtocol::Socks5 => {
            format!("socks5://{}:{}", proxy.host, proxy.port)
        }
        _ => return None,
    };

    let reqwest_proxy = reqwest::Proxy::all(&proxy_url).ok()?;
    let client = reqwest::Client::builder()
        .proxy(reqwest_proxy)
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let resp = client
        .get("https://httpbin.org/headers")
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    let body = resp.text().await.ok()?;

    let headers_json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let headers = headers_json.get("headers")?.as_object()?;

    let header_keys_lower: Vec<String> = headers.keys().map(|k| k.to_lowercase()).collect();

    let has_forwarded_for = header_keys_lower.iter().any(|k| k == "x-forwarded-for");
    let has_via = header_keys_lower.iter().any(|k| k == "via");
    let has_any_proxy_header = header_keys_lower
        .iter()
        .any(|k| PROXY_REVEALING_HEADERS.contains(&k.as_str()));

    if has_forwarded_for {
        Some(AnonymityLevel::Transparent)
    } else if has_via || has_any_proxy_header {
        Some(AnonymityLevel::Anonymous)
    } else {
        Some(AnonymityLevel::Elite)
    }
}

/// Check anonymity of a proxy with a timeout, returning None on failure.
pub async fn check_anonymity_safe(proxy: &Proxy) -> Option<AnonymityLevel> {
    match tokio::time::timeout(Duration::from_secs(15), check_anonymity(proxy)).await {
        Ok(result) => result,
        Err(_) => {
            tracing::debug!("Anonymity check timed out for {}", proxy);
            None
        }
    }
}
