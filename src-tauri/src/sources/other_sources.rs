use crate::proxy_type::{Proxy, ProxyProtocol};
use anyhow::Result;

fn create_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?)
}

async fn fetch_from_url(url: &str, protocol: ProxyProtocol) -> Result<Vec<Proxy>> {
    let client = create_client()?;
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    Ok(parse_proxy_list(&text, protocol))
}

pub async fn fetch_free_proxy_list() -> Result<Vec<Proxy>> {
    fetch_from_url("https://api.openproxylist.xyz/socks5.txt", ProxyProtocol::Socks5).await
}

pub async fn fetch_proxy_list_download() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_hookzof() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/hookzof/socks5_list/master/proxy.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_monosans() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_manu_git() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/manuGMG/proxy-365/main/SOCKS5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_proxyspace() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/ProxySpace/socks5-proxy-list/main/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_sunny9577() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/sunny9577/proxy-scraper/master/proxies.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_zaeem20() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_jetkai() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_roosterkid() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/roosterkid/openproxylist/main/SOCKS5_RAW.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_prxchk() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/prxchk/proxy-list/main/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_vakhov() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/vakhov/fresh-proxy-list/master/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_ercindedeoglu() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/ErcinDedeoglu/proxies/main/proxies/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_proxylist_org() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/proxy4parsing/proxy-list/main/socks5.txt",
        ProxyProtocol::Socks5,
    )
    .await
}

pub async fn fetch_free_proxy_list_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url("https://api.openproxylist.xyz/socks4.txt", ProxyProtocol::Socks4).await
}

pub async fn fetch_proxy_list_download_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/socks4.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_monosans_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks4.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_jetkai_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-socks4.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_ercindedeoglu_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/ErcinDedeoglu/proxies/main/proxies/socks4.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_zaeem20_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/socks4.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_roosterkid_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/roosterkid/openproxylist/main/SOCKS4_RAW.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_prxchk_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/prxchk/proxy-list/main/socks4.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_vakhov_socks4() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/vakhov/fresh-proxy-list/master/socks4.txt",
        ProxyProtocol::Socks4,
    )
    .await
}

pub async fn fetch_free_proxy_list_http() -> Result<Vec<Proxy>> {
    fetch_from_url("https://api.openproxylist.xyz/http.txt", ProxyProtocol::Http).await
}

pub async fn fetch_proxy_list_download_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/http.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_monosans_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_jetkai_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-http.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_ercindedeoglu_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/ErcinDedeoglu/proxies/main/proxies/http.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_zaeem20_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/Zaeem20/FREE_PROXIES_LIST/master/http.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_roosterkid_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/roosterkid/openproxylist/main/HTTP_RAW.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_prxchk_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/prxchk/proxy-list/main/http.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_vakhov_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/vakhov/fresh-proxy-list/master/http.txt",
        ProxyProtocol::Http,
    )
    .await
}

pub async fn fetch_proxylist_org_http() -> Result<Vec<Proxy>> {
    fetch_from_url(
        "https://raw.githubusercontent.com/proxy4parsing/proxy-list/main/http.txt",
        ProxyProtocol::Http,
    )
    .await
}

fn parse_proxy_list(text: &str, protocol: ProxyProtocol) -> Vec<Proxy> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 2 {
                return None;
            }

            let host = parts[0].trim().to_string();
            let port = parts[1]
                .trim()
                .split_whitespace()
                .next()?
                .parse::<u16>()
                .ok()?;

            if host.is_empty() || !host.chars().next()?.is_numeric() {
                return None;
            }

            Some(Proxy::new(host, port, protocol.clone()))
        })
        .collect()
}
