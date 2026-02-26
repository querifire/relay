use crate::proxy_chain;
use crate::proxy_instance::{LogSink, ProxyStats};
use crate::proxy_type::{Proxy, ProxyProtocol};
use crate::upstream;
use anyhow::{anyhow, Result};
use base64::Engine;
use hmac::{Hmac, Mac};
use parking_lot::RwLock;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

type HmacSha256 = Hmac<Sha256>;

static AUTH_HMAC_KEY: OnceLock<[u8; 32]> = OnceLock::new();

fn auth_hmac_key() -> &'static [u8; 32] {
    AUTH_HMAC_KEY.get_or_init(|| {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::rng().fill_bytes(&mut key);
        key
    })
}

/// Constant-time string equality that is safe against timing side-channels
/// regardless of string length differences.
///
/// Both strings are HMAC-SHA256'd with the same per-session random key; the

fn ct_str_eq(a: &str, b: &str) -> bool {
    let key = auth_hmac_key();

    let mut mac_a = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac_a.update(a.as_bytes());
    let digest_a = mac_a.finalize().into_bytes();

    let mut mac_b = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac_b.update(b.as_bytes());
    let digest_b = mac_b.finalize().into_bytes();

    digest_a.ct_eq(&digest_b).into()
}

const SOCKS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

const MAX_CONCURRENT_CONNECTIONS: usize = 500;

#[derive(Default)]
pub struct AuthRateLimiter(parking_lot::Mutex<HashMap<String, (u32, Instant)>>);

const AUTH_FAIL_DELAY: Duration = Duration::from_secs(2);
const AUTH_FAIL_WINDOW: Duration = Duration::from_secs(60);
const AUTH_FAIL_BLOCK_AFTER: u32 = 5;
const AUTH_FAIL_BLOCK_FOR: Duration = Duration::from_secs(60);
const AUTH_FAIL_CLEANUP_AGE: Duration = Duration::from_secs(3600);

impl AuthRateLimiter {
    
    pub fn delay_after_failure(&self, ip: &str) -> Duration {
        let now = Instant::now();
        let mut guard = self.0.lock();
        
        guard.retain(|_, (_, first)| now.duration_since(*first) < AUTH_FAIL_CLEANUP_AGE);
        let (count, first) = guard
            .entry(ip.to_string())
            .and_modify(|(c, _)| *c += 1)
            .or_insert((1, now));
        if *count >= AUTH_FAIL_BLOCK_AFTER && now.duration_since(*first) < AUTH_FAIL_WINDOW {
            AUTH_FAIL_BLOCK_FOR
        } else {
            AUTH_FAIL_DELAY
        }
    }
}

pub type AuthCredentials = Option<(String, String)>;

pub type ChainProxies = Option<Arc<Vec<Proxy>>>;

async fn connect_upstream(
    upstream: &Proxy,
    chain: &ChainProxies,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    if let Some(chain_proxies) = chain {
        if !chain_proxies.is_empty() {
            let mut full_chain = chain_proxies.as_ref().clone();
            full_chain.push(upstream.clone());
            return proxy_chain::connect_through_chain(&full_chain, target_host, target_port).await;
        }
    }
    upstream::connect_through_proxy(upstream, target_host, target_port).await
}

pub fn start_local_server(
    protocol: ProxyProtocol,
    bind_addr: String,
    upstream_proxy: Arc<RwLock<Proxy>>,
    cancel_token: CancellationToken,
    log_sink: LogSink,
    auth: AuthCredentials,
    stats: Arc<ProxyStats>,
    chain: ChainProxies,
) -> JoinHandle<Result<()>> {
    let conn_limit = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
    let rate_limiter = Arc::new(AuthRateLimiter::default());
    match protocol {
        ProxyProtocol::Http | ProxyProtocol::Https => {
            let limiter = rate_limiter.clone();
            tokio::spawn(async move {
                run_http_proxy_server(bind_addr, upstream_proxy, cancel_token, log_sink, auth, stats, chain, conn_limit, limiter).await
            })
        }
        ProxyProtocol::Socks4 => {
            
            if auth.is_some() {
                push_to_sink(
                    &log_sink,
                    "WARNING: SOCKS4 does not support authentication — credentials are ignored. \
                     Switch to SOCKS5 or HTTP to enforce a password.",
                );
                tracing::warn!(
                    "SOCKS4 proxy started with auth credentials configured — \
                     credentials will be silently ignored."
                );
            }
            tokio::spawn(async move {
                run_socks4_server(bind_addr, upstream_proxy, cancel_token, log_sink, stats, chain, conn_limit).await
            })
        }
        ProxyProtocol::Socks5 => {
            let limiter = rate_limiter.clone();
            tokio::spawn(async move {
                run_socks5_server(bind_addr, upstream_proxy, cancel_token, log_sink, auth, stats, chain, conn_limit, limiter).await
            })
        }
        ProxyProtocol::Tor => {
            tokio::spawn(async move {
                Err(anyhow!("Tor local server is not yet implemented"))
            })
        }
    }
}

async fn run_http_proxy_server(
    bind_addr: String,
    upstream_proxy: Arc<RwLock<Proxy>>,
    cancel_token: CancellationToken,
    log_sink: LogSink,
    auth: AuthCredentials,
    stats: Arc<ProxyStats>,
    chain: ChainProxies,
    conn_limit: Arc<Semaphore>,
    rate_limiter: Arc<AuthRateLimiter>,
) -> Result<()> {
    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!("[OK] HTTP прокси-сервер запущен на {}", bind_addr);

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::info!("[*] HTTP прокси-сервер на {} останавливается", bind_addr);
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((client_stream, client_addr)) => {
                        let upstream = upstream_proxy.clone();
                        let sink = log_sink.clone();
                        let auth = auth.clone();
                        let stats = stats.clone();
                        let chain = chain.clone();
                        let permit = conn_limit.clone();
                        let limiter = rate_limiter.clone();
                        let addr_str = client_addr.to_string();
                        let addr_str_log = addr_str.clone();
                        tokio::spawn(async move {
                            let _guard = match permit.acquire().await {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                            if let Err(e) = handle_http_client(client_stream, addr_str, upstream, sink, auth, stats, chain, limiter).await {
                                tracing::debug!("Ошибка HTTP клиента {}: {}", addr_str_log, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Ошибка принятия соединения: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_http_client(
    mut client_stream: TcpStream,
    client_addr: String,
    upstream_proxy: Arc<RwLock<Proxy>>,
    log_sink: LogSink,
    auth: AuthCredentials,
    stats: Arc<ProxyStats>,
    chain: ChainProxies,
    rate_limiter: Arc<AuthRateLimiter>,
) -> Result<()> {
    
    const HEADER_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1];

    loop {
        tokio::time::timeout(HEADER_READ_TIMEOUT, client_stream.read_exact(&mut tmp))
            .await
            .map_err(|_| anyhow!("HTTP header read timeout"))??;
        buf.push(tmp[0]);

        if buf.len() >= 4 {
            let len = buf.len();
            if &buf[len - 4..len] == b"\r\n\r\n" {
                break;
            }
        }

        if buf.len() > 16384 {
            return Err(anyhow!("HTTP запрос слишком длинный"));
        }
    }

    let request_str = String::from_utf8_lossy(&buf).to_string();
    let first_line = request_str.lines().next().unwrap_or("").to_string();

    if let Some((expected_user, expected_pass)) = &auth {
        let authenticated = request_str
            .lines()
            .find(|line| line.to_lowercase().starts_with("proxy-authorization:"))
            .and_then(|line| {
                let value = line.splitn(2, ':').nth(1)?.trim();
                if value.to_lowercase().starts_with("basic ") {
                    let encoded = value[6..].trim();
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .ok()?;
                    let cred = String::from_utf8(decoded).ok()?;
                    let mut parts = cred.splitn(2, ':');
                    let user = parts.next()?;
                    let pass = parts.next().unwrap_or("");
                    let ok = ct_str_eq(user, expected_user) & ct_str_eq(pass, expected_pass);
                    Some(ok)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        if !authenticated {
            let delay = rate_limiter.delay_after_failure(&client_addr);
            tokio::time::sleep(delay).await;
            let response = concat!(
                "HTTP/1.1 407 Proxy Authentication Required\r\n",
                "Proxy-Authenticate: Basic realm=\"Relay Proxy\"\r\n",
                "Content-Length: 0\r\n\r\n"
            );
            client_stream.write_all(response.as_bytes()).await?;
            return Err(anyhow!("Аутентификация не пройдена"));
        }
    }

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
        client_stream.write_all(response.as_bytes()).await?;
        return Err(anyhow!("Некорректный HTTP запрос"));
    }

    let method = parts[0].to_uppercase();

    if method == "CONNECT" {
        let target = parts[1];
        let (target_host, target_port) = parse_host_port(target, 443)?;

        {
            let msg = format!("CONNECT {}:{}", target_host, target_port);
            let mut logs = log_sink.lock();
            logs.push_back(msg);
            while logs.len() > 500 {
                logs.pop_front();
            }
        }

        let current_proxy = upstream_proxy.read().clone();
        stats.total_requests.fetch_add(1, Ordering::Relaxed);
        let connect_start = std::time::Instant::now();

        match connect_upstream(&current_proxy, &chain, &target_host, target_port).await {
            Ok(mut upstream_stream) => {
                let latency_ms = connect_start.elapsed().as_millis() as u64;
                stats.successful_requests.fetch_add(1, Ordering::Relaxed);
                stats.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
                stats.last_request_latency_ms.store(latency_ms, Ordering::Relaxed);

                let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
                client_stream.write_all(response.as_bytes()).await?;

                let (mut client_read, mut client_write) = client_stream.split();
                let (mut upstream_read, mut upstream_write) = upstream_stream.split();

                let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
                let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);

                tokio::select! {
                    r = client_to_upstream => {
                        if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                    },
                    r = upstream_to_client => {
                        if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                    },
                }

                Ok(())
            }
            Err(e) => {
                let response = "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n";
                client_stream.write_all(response.as_bytes()).await?;
                Err(e)
            }
        }
    } else {
        let url = parts[1];
        let (target_host, target_port, path) = parse_http_url(url)?;

        {
            let msg = format!("{} {}:{}{}", method, target_host, target_port, path);
            let mut logs = log_sink.lock();
            logs.push_back(msg);
            while logs.len() > 500 {
                logs.pop_front();
            }
        }

        let current_proxy = upstream_proxy.read().clone();
        stats.total_requests.fetch_add(1, Ordering::Relaxed);
        let connect_start = std::time::Instant::now();

        match connect_upstream(&current_proxy, &chain, &target_host, target_port).await {
            Ok(mut upstream_stream) => {
                let latency_ms = connect_start.elapsed().as_millis() as u64;
                stats.successful_requests.fetch_add(1, Ordering::Relaxed);
                stats.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
                stats.last_request_latency_ms.store(latency_ms, Ordering::Relaxed);

                let rewritten_first_line = format!(
                    "{} {} {}",
                    method,
                    path,
                    parts.get(2).unwrap_or(&"HTTP/1.1")
                );
                let mut rewritten_request = rewritten_first_line;
                rewritten_request.push_str("\r\n");

                for line in request_str.lines().skip(1) {
                    if line.is_empty() {
                        break;
                    }
                    if !line.to_lowercase().starts_with("proxy-authorization:") {
                        rewritten_request.push_str(line);
                        rewritten_request.push_str("\r\n");
                    }
                }
                rewritten_request.push_str("\r\n");

                upstream_stream
                    .write_all(rewritten_request.as_bytes())
                    .await?;

                let (mut client_read, mut client_write) = client_stream.split();
                let (mut upstream_read, mut upstream_write) = upstream_stream.split();

                let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
                let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);

                tokio::select! {
                    r = client_to_upstream => {
                        if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                    },
                    r = upstream_to_client => {
                        if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                    },
                }

                Ok(())
            }
            Err(e) => {
                let response = "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n";
                client_stream.write_all(response.as_bytes()).await?;
                Err(e)
            }
        }
    }
}

fn parse_host_port(target: &str, default_port: u16) -> Result<(String, u16)> {
    if let Some(colon_pos) = target.rfind(':') {
        let host = target[..colon_pos].to_string();
        let port = target[colon_pos + 1..]
            .parse::<u16>()
            .unwrap_or(default_port);
        Ok((host, port))
    } else {
        Ok((target.to_string(), default_port))
    }
}

fn parse_http_url(url: &str) -> Result<(String, u16, String)> {
    let url = if url.starts_with("http://") {
        &url[7..]
    } else if url.starts_with("https://") {
        &url[8..]
    } else {
        url
    };

    let (host_port, path) = if let Some(slash_pos) = url.find('/') {
        (&url[..slash_pos], &url[slash_pos..])
    } else {
        (url, "/")
    };

    let (host, port) = parse_host_port(host_port, 80)?;
    Ok((host, port, path.to_string()))
}

async fn run_socks4_server(
    bind_addr: String,
    upstream_proxy: Arc<RwLock<Proxy>>,
    cancel_token: CancellationToken,
    log_sink: LogSink,
    stats: Arc<ProxyStats>,
    chain: ChainProxies,
    conn_limit: Arc<Semaphore>,
) -> Result<()> {
    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!("[OK] SOCKS4 сервер запущен на {}", bind_addr);

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::info!("[*] SOCKS4 сервер на {} останавливается", bind_addr);
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((client_stream, client_addr)) => {
                        let upstream = upstream_proxy.clone();
                        let sink = log_sink.clone();
                        let stats = stats.clone();
                        let chain = chain.clone();
                        let permit = conn_limit.clone();
                        tokio::spawn(async move {
                            let _guard = match permit.acquire().await {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                            if let Err(e) = handle_socks4_client(client_stream, upstream, sink, stats, chain).await {
                                tracing::debug!("Ошибка SOCKS4 клиента {}: {}", client_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Ошибка принятия соединения: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn socks4_handshake(
    client_stream: &mut TcpStream,
) -> Result<(String, u16, [u8; 8])> {
    
    let mut header = [0u8; 8];
    client_stream.read_exact(&mut header).await?;

    let version = header[0];
    if version != 0x04 {
        return Err(anyhow!("Неверная версия SOCKS4: {}", version));
    }

    let cmd = header[1];
    if cmd != 0x01 {
        let reply = [0x00u8, 0x5B, 0, 0, 0, 0, 0, 0];
        client_stream.write_all(&reply).await?;
        return Err(anyhow!("SOCKS4: поддерживается только CONNECT (cmd={})", cmd));
    }

    let target_port = u16::from_be_bytes([header[2], header[3]]);
    let dst_ip = [header[4], header[5], header[6], header[7]];

    const MAX_USERID_LEN: usize = 255;
    let mut _userid = Vec::new();
    loop {
        let mut b = [0u8; 1];
        client_stream.read_exact(&mut b).await?;
        if b[0] == 0 {
            break;
        }
        if _userid.len() >= MAX_USERID_LEN {
            let reply = [0x00u8, 0x5B, 0, 0, 0, 0, 0, 0];
            client_stream.write_all(&reply).await?;
            return Err(anyhow!("SOCKS4 USERID too long"));
        }
        _userid.push(b[0]);
    }

    const MAX_DOMAIN_LEN: usize = 255;
    let target_host = if dst_ip[0] == 0 && dst_ip[1] == 0 && dst_ip[2] == 0 && dst_ip[3] != 0 {
        let mut domain = Vec::new();
        loop {
            let mut b = [0u8; 1];
            client_stream.read_exact(&mut b).await?;
            if b[0] == 0 {
                break;
            }
            if domain.len() >= MAX_DOMAIN_LEN {
                let reply = [0x00u8, 0x5B, 0, 0, 0, 0, 0, 0];
                client_stream.write_all(&reply).await?;
                return Err(anyhow!("SOCKS4a domain too long"));
            }
            domain.push(b[0]);
        }
        String::from_utf8(domain)?
    } else {
        format!("{}.{}.{}.{}", dst_ip[0], dst_ip[1], dst_ip[2], dst_ip[3])
    };

    Ok((target_host, target_port, header))
}

async fn handle_socks4_client(
    mut client_stream: TcpStream,
    upstream_proxy: Arc<RwLock<Proxy>>,
    log_sink: LogSink,
    stats: Arc<ProxyStats>,
    chain: ChainProxies,
) -> Result<()> {
    let (target_host, target_port, header) =
        tokio::time::timeout(SOCKS_HANDSHAKE_TIMEOUT, socks4_handshake(&mut client_stream))
            .await
            .map_err(|_| anyhow!("SOCKS4 handshake timeout"))??;

    let dst_ip = [header[4], header[5], header[6], header[7]];

    {
        let msg = format!("CONNECT {}:{}", target_host, target_port);
        let mut logs = log_sink.lock();
        logs.push_back(msg);
        while logs.len() > 500 {
            logs.pop_front();
        }
    }

    let current_proxy = upstream_proxy.read().clone();
    stats.total_requests.fetch_add(1, Ordering::Relaxed);
    let connect_start = std::time::Instant::now();

    tracing::debug!(
        "SOCKS4 {} -> {}:{}",
        current_proxy,
        target_host,
        target_port
    );

    match connect_upstream(&current_proxy, &chain, &target_host, target_port).await {
        Ok(mut upstream_stream) => {
            let latency_ms = connect_start.elapsed().as_millis() as u64;
            stats.successful_requests.fetch_add(1, Ordering::Relaxed);
            stats.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
            stats.last_request_latency_ms.store(latency_ms, Ordering::Relaxed);

            let reply = [0x00, 0x5A, header[2], header[3], dst_ip[0], dst_ip[1], dst_ip[2], dst_ip[3]];
            client_stream.write_all(&reply).await?;

            let (mut client_read, mut client_write) = client_stream.split();
            let (mut upstream_read, mut upstream_write) = upstream_stream.split();

            let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
            let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);

            tokio::select! {
                r = client_to_upstream => {
                    if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                },
                r = upstream_to_client => {
                    if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                },
            }

            Ok(())
        }
        Err(e) => {
            let reply = [0x00, 0x5B, 0, 0, 0, 0, 0, 0];
            client_stream.write_all(&reply).await?;
            Err(e)
        }
    }
}

async fn run_socks5_server(
    bind_addr: String,
    upstream_proxy: Arc<RwLock<Proxy>>,
    cancel_token: CancellationToken,
    log_sink: LogSink,
    auth: AuthCredentials,
    stats: Arc<ProxyStats>,
    chain: ChainProxies,
    conn_limit: Arc<Semaphore>,
    rate_limiter: Arc<AuthRateLimiter>,
) -> Result<()> {
    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!("[OK] SOCKS5 сервер запущен на {}", bind_addr);

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::info!("[*] SOCKS5 сервер на {} останавливается", bind_addr);
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((client_stream, client_addr)) => {
                        let upstream = upstream_proxy.clone();
                        let sink = log_sink.clone();
                        let auth = auth.clone();
                        let stats = stats.clone();
                        let chain = chain.clone();
                        let permit = conn_limit.clone();
                        let limiter = rate_limiter.clone();
                        tokio::spawn(async move {
                            let _guard = match permit.acquire().await {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                            if let Err(e) = handle_socks5_client(client_stream, upstream, sink, auth, stats, chain, limiter).await {
                                tracing::debug!("Ошибка SOCKS5 клиента {}: {}", client_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Ошибка принятия соединения: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn socks5_handshake(
    client_stream: &mut TcpStream,
    auth: &AuthCredentials,
    rate_limiter: &AuthRateLimiter,
    client_addr: &str,
) -> Result<(String, u16)> {
    let mut header = [0u8; 2];
    client_stream.read_exact(&mut header).await?;

    if header[0] != 0x05 {
        return Err(anyhow!("Неверная версия SOCKS5"));
    }

    let nmethods = header[1] as usize;
    let mut methods = vec![0u8; nmethods];
    client_stream.read_exact(&mut methods).await?;

    if auth.is_some() {
        if !methods.contains(&0x02) {
            client_stream.write_all(&[0x05, 0xFF]).await?;
            return Err(anyhow!("Клиент не поддерживает аутентификацию"));
        }
        client_stream.write_all(&[0x05, 0x02]).await?;
        
        let mut auth_ver = [0u8; 1];
        client_stream.read_exact(&mut auth_ver).await?;
        if auth_ver[0] != 0x01 {
            return Err(anyhow!("Неверная версия аутентификации SOCKS5"));
        }

        let mut ulen = [0u8; 1];
        client_stream.read_exact(&mut ulen).await?;
        let mut uname = vec![0u8; ulen[0] as usize];
        client_stream.read_exact(&mut uname).await?;

        let mut plen = [0u8; 1];
        client_stream.read_exact(&mut plen).await?;
        let mut passwd = vec![0u8; plen[0] as usize];
        client_stream.read_exact(&mut passwd).await?;

        let username = String::from_utf8_lossy(&uname).to_string();
        let password = String::from_utf8_lossy(&passwd).to_string();

        let (expected_user, expected_pass) = auth.as_ref().unwrap();
        let ok = ct_str_eq(&username, expected_user) & ct_str_eq(&password, expected_pass);
        if !ok {
            let delay = rate_limiter.delay_after_failure(client_addr);
            tokio::time::sleep(delay).await;
            client_stream.write_all(&[0x01, 0x01]).await?;
            return Err(anyhow!("SOCKS5 аутентификация не пройдена"));
        }
        client_stream.write_all(&[0x01, 0x00]).await?;
    } else {
        client_stream.write_all(&[0x05, 0x00]).await?;
    }

    let mut request = [0u8; 4];
    client_stream.read_exact(&mut request).await?;

    if request[0] != 0x05 {
        return Err(anyhow!("Неверная версия в запросе"));
    }

    let cmd = request[1];
    if cmd != 0x01 {
        client_stream
            .write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await?;
        return Err(anyhow!("Поддерживается только CONNECT"));
    }

    let atyp = request[3];
    let (target_host, target_port) = match atyp {
        0x01 => {
            let mut addr = [0u8; 4];
            client_stream.read_exact(&mut addr).await?;
            let ip = format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]);
            let mut port_bytes = [0u8; 2];
            client_stream.read_exact(&mut port_bytes).await?;
            let port = u16::from_be_bytes(port_bytes);
            (ip, port)
        }
        0x03 => {
            let mut len = [0u8; 1];
            client_stream.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            client_stream.read_exact(&mut domain).await?;
            let domain = String::from_utf8(domain)?;
            let mut port_bytes = [0u8; 2];
            client_stream.read_exact(&mut port_bytes).await?;
            let port = u16::from_be_bytes(port_bytes);
            (domain, port)
        }
        0x04 => {
            let mut addr = [0u8; 16];
            client_stream.read_exact(&mut addr).await?;
            let mut port_bytes = [0u8; 2];
            client_stream.read_exact(&mut port_bytes).await?;
            let port = u16::from_be_bytes(port_bytes);
            let ip = format!(
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                u16::from_be_bytes([addr[0], addr[1]]),
                u16::from_be_bytes([addr[2], addr[3]]),
                u16::from_be_bytes([addr[4], addr[5]]),
                u16::from_be_bytes([addr[6], addr[7]]),
                u16::from_be_bytes([addr[8], addr[9]]),
                u16::from_be_bytes([addr[10], addr[11]]),
                u16::from_be_bytes([addr[12], addr[13]]),
                u16::from_be_bytes([addr[14], addr[15]])
            );
            (ip, port)
        }
        _ => {
            client_stream
                .write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await?;
            return Err(anyhow!("Неподдерживаемый тип адреса"));
        }
    };

    Ok((target_host, target_port))
}

async fn handle_socks5_client(
    mut client_stream: TcpStream,
    upstream_proxy: Arc<RwLock<Proxy>>,
    log_sink: LogSink,
    auth: AuthCredentials,
    stats: Arc<ProxyStats>,
    chain: ChainProxies,
    rate_limiter: Arc<AuthRateLimiter>,
) -> Result<()> {
    let client_addr = client_stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into());

    let (target_host, target_port) = tokio::time::timeout(
        SOCKS_HANDSHAKE_TIMEOUT,
        socks5_handshake(&mut client_stream, &auth, &rate_limiter, &client_addr),
    )
    .await
    .map_err(|_| anyhow!("SOCKS5 handshake timeout"))??;

    {
        let msg = format!("CONNECT {}:{}", target_host, target_port);
        let mut logs = log_sink.lock();
        logs.push_back(msg);
        while logs.len() > 500 {
            logs.pop_front();
        }
    }

    let current_proxy = upstream_proxy.read().clone();
    stats.total_requests.fetch_add(1, Ordering::Relaxed);
    let connect_start = std::time::Instant::now();

    tracing::debug!(
        "SOCKS5 {} -> {}:{}",
        current_proxy,
        target_host,
        target_port
    );

    match connect_upstream(&current_proxy, &chain, &target_host, target_port).await {
        Ok(mut upstream_stream) => {
            let latency_ms = connect_start.elapsed().as_millis() as u64;
            stats.successful_requests.fetch_add(1, Ordering::Relaxed);
            stats.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
            stats.last_request_latency_ms.store(latency_ms, Ordering::Relaxed);

            client_stream
                .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await?;

            let (mut client_read, mut client_write) = client_stream.split();
            let (mut upstream_read, mut upstream_write) = upstream_stream.split();

            let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
            let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);

            tokio::select! {
                r = client_to_upstream => {
                    if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                },
                r = upstream_to_client => {
                    if let Ok(bytes) = r { stats.total_bytes.fetch_add(bytes, Ordering::Relaxed); }
                },
            }

            Ok(())
        }
        Err(e) => {
            client_stream
                .write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await?;
            Err(e)
        }
    }
}
