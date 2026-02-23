use crate::proxy_type::{Proxy, ProxyProtocol};
use anyhow::Result;

const PROXYSCRAPE_SOCKS5: &str = "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=socks5&timeout=10000&country=all";
const PROXYSCRAPE_SOCKS5_ANON: &str = "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=socks5&timeout=5000&country=all&anonymity=elite";
const PROXYSCRAPE_SOCKS4: &str = "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=socks4&timeout=10000&country=all";
const PROXYSCRAPE_SOCKS4_ANON: &str = "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=socks4&timeout=5000&country=all&anonymity=elite";
const PROXYSCRAPE_HTTP: &str = "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=http&timeout=10000&country=all";
const PROXYSCRAPE_HTTP_ANON: &str = "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=http&timeout=5000&country=all&anonymity=elite";

pub async fn fetch_socks5_proxies() -> Result<Vec<Proxy>> {
    fetch_by_protocol(ProxyProtocol::Socks5).await
}

pub async fn fetch_socks4_proxies() -> Result<Vec<Proxy>> {
    fetch_by_protocol(ProxyProtocol::Socks4).await
}

pub async fn fetch_http_proxies() -> Result<Vec<Proxy>> {
    fetch_by_protocol(ProxyProtocol::Http).await
}

async fn fetch_by_protocol(protocol: ProxyProtocol) -> Result<Vec<Proxy>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut all_proxies = Vec::new();

    let sources = match protocol {
        ProxyProtocol::Socks5 => vec![PROXYSCRAPE_SOCKS5, PROXYSCRAPE_SOCKS5_ANON],
        ProxyProtocol::Socks4 => vec![PROXYSCRAPE_SOCKS4, PROXYSCRAPE_SOCKS4_ANON],
        ProxyProtocol::Http | ProxyProtocol::Https => vec![PROXYSCRAPE_HTTP, PROXYSCRAPE_HTTP_ANON],
        _ => vec![],
    };

    for url in sources {
        match fetch_from_url(&client, url, protocol.clone()).await {
            Ok(proxies) => {
                all_proxies.extend(proxies);
            }
            Err(e) => {
                tracing::debug!("Ошибка загрузки {:?}: {}", protocol, e);
            }
        }
    }

    Ok(all_proxies)
}

async fn fetch_from_url(
    client: &reqwest::Client,
    url: &str,
    protocol: ProxyProtocol,
) -> Result<Vec<Proxy>> {
    let response = client.get(url).send().await?;
    let text = response.text().await?;

    let proxies = parse_proxy_list(&text, protocol);
    Ok(proxies)
}

fn parse_proxy_list(text: &str, protocol: ProxyProtocol) -> Vec<Proxy> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 2 {
                return None;
            }

            let host = parts[0].to_string();
            let port = parts[1].parse::<u16>().ok()?;

            Some(Proxy::new(host, port, protocol.clone()))
        })
        .collect()
}
