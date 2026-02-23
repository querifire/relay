use crate::proxy_type::{Proxy, ProxyProtocol};
use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn connect_through_proxy(
    proxy: &Proxy,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    match proxy.protocol {
        ProxyProtocol::Http | ProxyProtocol::Https => {
            connect_through_http(proxy, target_host, target_port).await
        }
        ProxyProtocol::Socks4 => connect_through_socks4(proxy, target_host, target_port).await,
        ProxyProtocol::Socks5 => connect_through_socks5(proxy, target_host, target_port).await,
    }
}

async fn connect_through_http(
    proxy: &Proxy,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
    let mut stream = TcpStream::connect(&proxy_addr).await?;

    let connect_request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n\r\n",
        target_host, target_port, target_host, target_port
    );

    stream.write_all(connect_request.as_bytes()).await?;

    let mut response = Vec::new();
    let mut buf = [0u8; 1];
    
    loop {
        stream.read_exact(&mut buf).await?;
        response.push(buf[0]);
        
        if response.len() >= 4 {
            let len = response.len();
            if &response[len-4..len] == b"\r\n\r\n" {
                break;
            }
        }
        
        if response.len() > 8192 {
            return Err(anyhow!("HTTP response слишком длинный"));
        }
    }

    let response_str = String::from_utf8_lossy(&response);
    let first_line = response_str.lines().next().unwrap_or("");

    tracing::debug!("Upstream HTTP ответ: {}", first_line);

    if first_line.contains(" 200") {
        Ok(stream)
    } else {
        Err(anyhow!("HTTP CONNECT failed: {}", first_line))
    }
}

async fn connect_through_socks4(
    proxy: &Proxy,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
    let stream = TcpStream::connect(&proxy_addr).await?;

    let target_addr = format!("{}:{}", target_host, target_port);
    let socks_stream = tokio_socks::tcp::Socks4Stream::connect_with_socket(
        stream,
        target_addr,
    )
    .await?;

    Ok(socks_stream.into_inner())
}

async fn connect_through_socks5(
    proxy: &Proxy,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
    let stream = TcpStream::connect(&proxy_addr).await?;

    let target_addr = format!("{}:{}", target_host, target_port);
    let socks_stream = tokio_socks::tcp::Socks5Stream::connect_with_socket(stream, target_addr)
        .await?;

    Ok(socks_stream.into_inner())
}
