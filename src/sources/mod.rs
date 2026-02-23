pub mod proxyscrape;
pub mod other_sources;

use crate::proxy_type::Proxy;
use anyhow::Result;

pub async fn fetch_socks5_proxies() -> Result<Vec<Proxy>> {
    let mut all_proxies = Vec::new();
    
    // Макрос для упрощения вызовов
    macro_rules! fetch_source {
        ($name:expr, $func:expr) => {
            match $func.await {
                Ok(proxies) if !proxies.is_empty() => {
                    tracing::info!("  {} -> {} прокси", $name, proxies.len());
                    all_proxies.extend(proxies);
                }
                Ok(_) => {
                    tracing::debug!("  {} -> 0 прокси", $name);
                }
                Err(e) => {
                    tracing::debug!("  {} -> ошибка: {}", $name, e);
                }
            }
        };
    }

    fetch_source!("ProxyScrape", proxyscrape::fetch_socks5_proxies());
    fetch_source!("OpenProxyList", other_sources::fetch_free_proxy_list());
    fetch_source!("TheSpeedX", other_sources::fetch_proxy_list_download());
    fetch_source!("Hookzof", other_sources::fetch_hookzof());
    fetch_source!("Monosans", other_sources::fetch_monosans());
    fetch_source!("ManuGit", other_sources::fetch_manu_git());
    fetch_source!("ProxySpace", other_sources::fetch_proxyspace());
    fetch_source!("Sunny9577", other_sources::fetch_sunny9577());
    fetch_source!("Zaeem20", other_sources::fetch_zaeem20());
    fetch_source!("Jetkai", other_sources::fetch_jetkai());
    fetch_source!("Roosterkid", other_sources::fetch_roosterkid());
    fetch_source!("Prxchk", other_sources::fetch_prxchk());
    fetch_source!("Vakhov", other_sources::fetch_vakhov());
    fetch_source!("ErcinDedeoglu", other_sources::fetch_ercindedeoglu());
    fetch_source!("ProxyListOrg", other_sources::fetch_proxylist_org());
    
    Ok(all_proxies)
}
