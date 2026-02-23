pub mod other_sources;
pub mod proxyscrape;

use crate::proxy_instance::{push_to_sink, LogSink};
use crate::proxy_type::{Proxy, ProxyProtocol};
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

/// Fetch proxies of the specified protocol from all sources.
/// If `log_sink` is provided, per-source progress is written there.
pub async fn fetch_proxies(protocol: ProxyProtocol, log_sink: Option<&LogSink>) -> Result<Vec<Proxy>> {
    match protocol {
        ProxyProtocol::Socks5 => fetch_socks5_proxies(log_sink).await,
        ProxyProtocol::Socks4 => fetch_socks4_proxies(log_sink).await,
        ProxyProtocol::Http | ProxyProtocol::Https => fetch_http_proxies(log_sink).await,
        ProxyProtocol::Tor => Ok(Vec::new()),
    }
}

fn process_source_result(
    name: &str,
    result: Result<Vec<Proxy>>,
    all_proxies: &mut Vec<Proxy>,
    log_sink: Option<&LogSink>,
) {
    match result {
        Ok(proxies) if !proxies.is_empty() => {
            let msg = format!("  {} -> {} proxies", name, proxies.len());
            tracing::info!("{}", msg);
            if let Some(sink) = log_sink {
                push_to_sink(sink, &msg);
            }
            all_proxies.extend(proxies);
        }
        Ok(_) => {
            tracing::debug!("  {} -> 0 прокси", name);
        }
        Err(e) => {
            tracing::debug!("  {} -> ошибка: {}", name, e);
        }
    }
}

type SourceFuture = Pin<Box<dyn Future<Output = (&'static str, Result<Vec<Proxy>>)> + Send>>;

/// Fetch SOCKS5 proxies from all sources (in parallel).
pub async fn fetch_socks5_proxies(log_sink: Option<&LogSink>) -> Result<Vec<Proxy>> {
    let mut all_proxies = Vec::new();

    let tasks: Vec<SourceFuture> = vec![
        Box::pin(async { ("ProxyScrape", proxyscrape::fetch_socks5_proxies().await) }),
        Box::pin(async { ("OpenProxyList", other_sources::fetch_free_proxy_list().await) }),
        Box::pin(async { ("TheSpeedX", other_sources::fetch_proxy_list_download().await) }),
        Box::pin(async { ("Hookzof", other_sources::fetch_hookzof().await) }),
        Box::pin(async { ("Monosans", other_sources::fetch_monosans().await) }),
        Box::pin(async { ("ManuGit", other_sources::fetch_manu_git().await) }),
        Box::pin(async { ("ProxySpace", other_sources::fetch_proxyspace().await) }),
        Box::pin(async { ("Sunny9577", other_sources::fetch_sunny9577().await) }),
        Box::pin(async { ("Zaeem20", other_sources::fetch_zaeem20().await) }),
        Box::pin(async { ("Jetkai", other_sources::fetch_jetkai().await) }),
        Box::pin(async { ("Roosterkid", other_sources::fetch_roosterkid().await) }),
        Box::pin(async { ("Prxchk", other_sources::fetch_prxchk().await) }),
        Box::pin(async { ("Vakhov", other_sources::fetch_vakhov().await) }),
        Box::pin(async { ("ErcinDedeoglu", other_sources::fetch_ercindedeoglu().await) }),
        Box::pin(async { ("ProxyListOrg", other_sources::fetch_proxylist_org().await) }),
    ];
    let results = futures::future::join_all(tasks).await;

    for (name, result) in results {
        process_source_result(name, result, &mut all_proxies, log_sink);
    }

    Ok(all_proxies)
}

/// Fetch SOCKS4 proxies from all sources (in parallel).
pub async fn fetch_socks4_proxies(log_sink: Option<&LogSink>) -> Result<Vec<Proxy>> {
    let mut all_proxies = Vec::new();

    let tasks: Vec<SourceFuture> = vec![
        Box::pin(async { ("ProxyScrape/S4", proxyscrape::fetch_socks4_proxies().await) }),
        Box::pin(async { ("OpenProxyList/S4", other_sources::fetch_free_proxy_list_socks4().await) }),
        Box::pin(async { ("TheSpeedX/S4", other_sources::fetch_proxy_list_download_socks4().await) }),
        Box::pin(async { ("Monosans/S4", other_sources::fetch_monosans_socks4().await) }),
        Box::pin(async { ("Jetkai/S4", other_sources::fetch_jetkai_socks4().await) }),
        Box::pin(async { ("ErcinDedeoglu/S4", other_sources::fetch_ercindedeoglu_socks4().await) }),
        Box::pin(async { ("Zaeem20/S4", other_sources::fetch_zaeem20_socks4().await) }),
        Box::pin(async { ("Roosterkid/S4", other_sources::fetch_roosterkid_socks4().await) }),
        Box::pin(async { ("Prxchk/S4", other_sources::fetch_prxchk_socks4().await) }),
        Box::pin(async { ("Vakhov/S4", other_sources::fetch_vakhov_socks4().await) }),
    ];
    let results = futures::future::join_all(tasks).await;

    for (name, result) in results {
        process_source_result(name, result, &mut all_proxies, log_sink);
    }

    Ok(all_proxies)
}

/// Fetch HTTP proxies from all sources (in parallel).
pub async fn fetch_http_proxies(log_sink: Option<&LogSink>) -> Result<Vec<Proxy>> {
    let mut all_proxies = Vec::new();

    let tasks: Vec<SourceFuture> = vec![
        Box::pin(async { ("ProxyScrape/HTTP", proxyscrape::fetch_http_proxies().await) }),
        Box::pin(async { ("OpenProxyList/HTTP", other_sources::fetch_free_proxy_list_http().await) }),
        Box::pin(async { ("TheSpeedX/HTTP", other_sources::fetch_proxy_list_download_http().await) }),
        Box::pin(async { ("Monosans/HTTP", other_sources::fetch_monosans_http().await) }),
        Box::pin(async { ("Jetkai/HTTP", other_sources::fetch_jetkai_http().await) }),
        Box::pin(async { ("ErcinDedeoglu/HTTP", other_sources::fetch_ercindedeoglu_http().await) }),
        Box::pin(async { ("Zaeem20/HTTP", other_sources::fetch_zaeem20_http().await) }),
        Box::pin(async { ("Roosterkid/HTTP", other_sources::fetch_roosterkid_http().await) }),
        Box::pin(async { ("Prxchk/HTTP", other_sources::fetch_prxchk_http().await) }),
        Box::pin(async { ("Vakhov/HTTP", other_sources::fetch_vakhov_http().await) }),
        Box::pin(async { ("ProxyListOrg/HTTP", other_sources::fetch_proxylist_org_http().await) }),
    ];
    let results = futures::future::join_all(tasks).await;

    for (name, result) in results {
        process_source_result(name, result, &mut all_proxies, log_sink);
    }

    Ok(all_proxies)
}
