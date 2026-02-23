use crate::proxy_type::Proxy;
use crate::upstream;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn run_socks5_server(bind_addr: String, upstream_proxy: Arc<RwLock<Proxy>>) -> Result<()> {
    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!("[OK] SOCKS5 сервер запущен");
    tracing::info!("     Подключайтесь: socks5://{}", bind_addr);

    loop {
        match listener.accept().await {
            Ok((client_stream, client_addr)) => {
                let upstream_proxy = upstream_proxy.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_socks5_client(client_stream, upstream_proxy).await {
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

async fn handle_socks5_client(mut client_stream: TcpStream, upstream_proxy: Arc<RwLock<Proxy>>) -> Result<()> {
    let mut header = [0u8; 2];
    client_stream.read_exact(&mut header).await?;

    if header[0] != 0x05 {
        return Err(anyhow!("Неверная версия SOCKS5"));
    }

    let nmethods = header[1] as usize;
    let mut methods = vec![0u8; nmethods];
    client_stream.read_exact(&mut methods).await?;

    client_stream.write_all(&[0x05, 0x00]).await?;

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

    let current_proxy = upstream_proxy.read().clone();
    
    tracing::debug!("SOCKS5 {} -> {}:{}", current_proxy, target_host, target_port);

    match upstream::connect_through_proxy(&current_proxy, &target_host, target_port).await {
        Ok(mut upstream_stream) => {
            client_stream
                .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await?;

            let (mut client_read, mut client_write) = client_stream.split();
            let (mut upstream_read, mut upstream_write) = upstream_stream.split();

            let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
            let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);

            tokio::select! {
                _ = client_to_upstream => {},
                _ = upstream_to_client => {},
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
