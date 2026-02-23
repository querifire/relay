use crate::proxy_type::{Proxy, ProxyProtocol};
use crate::upstream;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Maximum number of proxies in a chain (to avoid excessive latency and resource use).
pub const MAX_CHAIN_DEPTH: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyChainConfig {
    pub enabled: bool,
    pub proxies: Vec<Proxy>,
}

impl Default for ProxyChainConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            proxies: Vec::new(),
        }
    }
}

/// Connect through a chain of proxies to reach the target.
///
/// The chain works by connecting to the first proxy, then issuing a
/// CONNECT/SOCKS handshake through it to reach the second proxy,
/// and so on until the final hop connects to the actual target.
pub async fn connect_through_chain(
    chain: &[Proxy],
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    if chain.is_empty() {
        return Err(anyhow!("Proxy chain is empty"));
    }
    if chain.len() > MAX_CHAIN_DEPTH {
        return Err(anyhow!(
            "Proxy chain has {} hops; maximum is {}",
            chain.len(),
            MAX_CHAIN_DEPTH
        ));
    }

    if chain.len() == 1 {
        return upstream::connect_through_proxy(&chain[0], target_host, target_port).await;
    }

    // Connect to the first proxy in the chain.
    let first = &chain[0];
    let first_addr = format!("{}:{}", first.host, first.port);
    let mut stream = TcpStream::connect(&first_addr).await?;

    // For each proxy in the chain, do a handshake through it to the next hop
    // (next proxy in chain or final target).
    for i in 0..chain.len() {
        let current_proxy = &chain[i];
        let (next_host, next_port) = if i + 1 < chain.len() {
            let next = &chain[i + 1];
            (next.host.as_str(), next.port)
        } else {
            (target_host, target_port)
        };
        stream = do_proxy_handshake(stream, current_proxy, next_host, next_port).await?;
    }

    Ok(stream)
}

/// Perform a proxy handshake on an already-established TCP stream.
async fn do_proxy_handshake(
    mut stream: TcpStream,
    proxy: &Proxy,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    match proxy.protocol {
        ProxyProtocol::Http | ProxyProtocol::Https => {
            http_connect_handshake(&mut stream, target_host, target_port).await?;
            Ok(stream)
        }
        ProxyProtocol::Socks5 => {
            socks5_handshake(&mut stream, target_host, target_port).await?;
            Ok(stream)
        }
        ProxyProtocol::Socks4 => {
            socks4_handshake(&mut stream, target_host, target_port).await?;
            Ok(stream)
        }
        _ => Err(anyhow!("Unsupported protocol in chain: {:?}", proxy.protocol)),
    }
}

async fn http_connect_handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
) -> Result<()> {
    let request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n\r\n",
        target_host, target_port, target_host, target_port
    );
    stream.write_all(request.as_bytes()).await?;

    const HEADER_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    let mut response = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        tokio::time::timeout(HEADER_READ_TIMEOUT, stream.read_exact(&mut buf))
            .await
            .map_err(|_| anyhow!("HTTP CONNECT header read timeout in chain"))??;
        response.push(buf[0]);
        if response.len() >= 4 {
            let len = response.len();
            if &response[len - 4..len] == b"\r\n\r\n" {
                break;
            }
        }
        if response.len() > 8192 {
            return Err(anyhow!("HTTP CONNECT response too long"));
        }
    }

    let response_str = String::from_utf8_lossy(&response);
    let first_line = response_str.lines().next().unwrap_or("");
    if first_line.contains(" 200") {
        Ok(())
    } else {
        Err(anyhow!("HTTP CONNECT failed in chain: {}", first_line))
    }
}

async fn socks5_handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
) -> Result<()> {
    // Greeting: version 5, 1 method (no auth)
    stream.write_all(&[0x05, 0x01, 0x00]).await?;

    let mut response = [0u8; 2];
    stream.read_exact(&mut response).await?;
    if response[0] != 0x05 || response[1] != 0x00 {
        return Err(anyhow!("SOCKS5 handshake failed in chain"));
    }

    // Connect request
    let host_bytes = target_host.as_bytes();
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8];
    request.extend_from_slice(host_bytes);
    request.extend_from_slice(&target_port.to_be_bytes());
    stream.write_all(&request).await?;

    // Read response (minimum 10 bytes for IPv4)
    let mut resp_header = [0u8; 4];
    stream.read_exact(&mut resp_header).await?;
    if resp_header[1] != 0x00 {
        return Err(anyhow!(
            "SOCKS5 connect failed in chain (reply: {})",
            resp_header[1]
        ));
    }

    // Skip the bound address
    match resp_header[3] {
        0x01 => {
            let mut addr = [0u8; 6]; // 4 IP + 2 port
            stream.read_exact(&mut addr).await?;
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize + 2];
            stream.read_exact(&mut domain).await?;
        }
        0x04 => {
            let mut addr = [0u8; 18]; // 16 IP + 2 port
            stream.read_exact(&mut addr).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn socks4_handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
) -> Result<()> {
    // SOCKS4a: use domain name
    let port_bytes = target_port.to_be_bytes();
    let domain = target_host.as_bytes();

    // VN=4, CD=1 (CONNECT), port, IP=0.0.0.1 (SOCKS4a), USERID null, domain null
    let mut request = vec![0x04, 0x01, port_bytes[0], port_bytes[1], 0, 0, 0, 1, 0];
    request.extend_from_slice(domain);
    request.push(0);
    stream.write_all(&request).await?;

    let mut response = [0u8; 8];
    stream.read_exact(&mut response).await?;
    if response[1] != 0x5A {
        return Err(anyhow!("SOCKS4 connect failed in chain (reply: {})", response[1]));
    }

    Ok(())
}
