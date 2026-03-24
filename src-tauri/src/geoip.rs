use crate::speed_test::ProxyWithSpeed;
use maxminddb::geoip2;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryInfo {
    pub country_code: String,
    pub country_name: Option<String>,
}

fn db_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("relay")
        .join("geoip")
        .join("GeoLite2-Country.mmdb")
}

pub fn lookup_country(ip: &str) -> Option<CountryInfo> {
    let db = db_path();
    if !db.exists() {
        return None;
    }

    let ip_addr: IpAddr = ip.parse().ok()?;
    let reader = maxminddb::Reader::open_readfile(db).ok()?;
    let country: geoip2::Country<'_> = reader.lookup(ip_addr).ok()?;
    let country_code = country
        .country
        .as_ref()
        .and_then(|c| c.iso_code)
        .map(str::to_uppercase)?;
    let country_name = country
        .country
        .as_ref()
        .and_then(|c| c.names.as_ref())
        .and_then(|n| n.get("en"))
        .map(|s| s.to_string());

    Some(CountryInfo {
        country_code,
        country_name,
    })
}

async fn resolve_ip(host: &str) -> Option<IpAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(ip);
    }
    let mut addrs = tokio::net::lookup_host((host, 0)).await.ok()?;
    addrs.next().map(|addr| addr.ip())
}

pub async fn lookup_host_country(host: &str) -> Option<CountryInfo> {
    let ip = resolve_ip(host).await?;
    lookup_country(&ip.to_string())
}

pub async fn filter_by_countries(
    tested: Vec<ProxyWithSpeed>,
    country_codes: &[String],
) -> Vec<ProxyWithSpeed> {
    let allowed: HashSet<String> = country_codes
        .iter()
        .map(|c| c.trim().to_uppercase())
        .filter(|c| !c.is_empty())
        .collect();
    if allowed.is_empty() {
        return tested;
    }

    let mut filtered = Vec::with_capacity(tested.len());
    for proxy in tested {
        let Some(country) = lookup_host_country(&proxy.proxy.host).await else {
            continue;
        };
        if allowed.contains(&country.country_code) {
            filtered.push(proxy);
        }
    }
    filtered
}

/// Filter plain proxies by GeoIP country (used when latency is unknown).
pub async fn filter_plain_proxies_by_countries(
    proxies: Vec<crate::proxy_type::Proxy>,
    country_codes: &[String],
) -> Vec<crate::proxy_type::Proxy> {
    let wrapped: Vec<ProxyWithSpeed> = proxies
        .into_iter()
        .map(|proxy| ProxyWithSpeed {
            proxy,
            latency: std::time::Duration::ZERO,
        })
        .collect();
    filter_by_countries(wrapped, country_codes)
        .await
        .into_iter()
        .map(|p| p.proxy)
        .collect()
}
