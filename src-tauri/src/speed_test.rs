use crate::proxy_type::{Proxy, ProxyProtocol};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{timeout, Instant};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const TEST_TARGET: &str = "1.1.1.1";
const TEST_PORT: u16 = 443;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyWithSpeed {
    pub proxy: Proxy,
    #[serde(with = "duration_millis")]
    pub latency: Duration,
}

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

pub async fn test_proxy(proxy: &Proxy) -> Option<Duration> {
    let start = Instant::now();

    let result = match proxy.protocol {
        ProxyProtocol::Http | ProxyProtocol::Https => test_http_proxy(proxy).await,
        ProxyProtocol::Socks4 | ProxyProtocol::Socks5 => test_socks_proxy(proxy).await,
        ProxyProtocol::Tor => return None, // Tor testing not yet implemented
    };

    match result {
        Ok(_) => {
            let latency = start.elapsed();
            tracing::debug!("[+] {} - {}ms", proxy, latency.as_millis());
            Some(latency)
        }
        Err(e) => {
            tracing::debug!("[-] {} - {}", proxy, e);
            None
        }
    }
}

async fn test_http_proxy(proxy: &Proxy) -> Result<()> {
    let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
    let mut stream = tokio::net::TcpStream::connect(&proxy_addr).await?;

    let connect_request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n\r\n",
        TEST_TARGET, TEST_PORT, TEST_TARGET, TEST_PORT
    );

    let test_future = async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        stream.write_all(connect_request.as_bytes()).await?;

        let mut response = Vec::new();
        let mut buf = [0u8; 1];

        loop {
            stream.read_exact(&mut buf).await?;
            response.push(buf[0]);

            if response.len() >= 4 {
                let len = response.len();
                if &response[len - 4..len] == b"\r\n\r\n" {
                    break;
                }
            }

            if response.len() > 8192 {
                return Err(anyhow::anyhow!("Response too long"));
            }
        }

        let response_str = String::from_utf8_lossy(&response);
        let first_line = response_str.lines().next().unwrap_or("");

        if first_line.contains(" 200") {
            Ok(())
        } else {
            Err(anyhow::anyhow!("CONNECT failed: {}", first_line))
        }
    };

    timeout(TEST_TIMEOUT, test_future).await??;

    Ok(())
}

async fn test_socks_proxy(proxy: &Proxy) -> Result<()> {
    let addr = format!("{}:{}", proxy.host, proxy.port);

    let connect_future = async {
        let stream = tokio::net::TcpStream::connect(&addr).await?;

        let target_addr = format!("{}:{}", TEST_TARGET, TEST_PORT);

        match proxy.protocol {
            ProxyProtocol::Socks5 => {
                let _socks_stream =
                    tokio_socks::tcp::Socks5Stream::connect_with_socket(stream, target_addr)
                        .await?;
            }
            ProxyProtocol::Socks4 => {
                let _socks_stream =
                    tokio_socks::tcp::Socks4Stream::connect_with_socket(stream, target_addr)
                        .await?;
            }
            _ => {}
        }

        Ok::<(), anyhow::Error>(())
    };

    timeout(TEST_TIMEOUT, connect_future).await??;

    Ok(())
}

pub async fn test_proxies_parallel(proxies: Vec<Proxy>, concurrency: usize) -> Vec<ProxyWithSpeed> {
    use futures::stream::{self, StreamExt};

    let results: Vec<Option<ProxyWithSpeed>> = stream::iter(proxies)
        .map(|proxy| async move {
            let latency = test_proxy(&proxy).await?;
            Some(ProxyWithSpeed { proxy, latency })
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    results.into_iter().flatten().collect()
}

pub fn select_fastest(mut proxies: Vec<ProxyWithSpeed>) -> Option<ProxyWithSpeed> {
    proxies.sort_by_key(|p| p.latency);
    proxies.into_iter().next()
}
