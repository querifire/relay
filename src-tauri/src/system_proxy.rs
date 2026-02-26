use anyhow::{anyhow, Result};
use std::process::Command;

const INTERNET_SETTINGS_KEY: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[derive(Debug, Clone)]
pub struct SystemProxyStatus {
    pub enabled: bool,
    pub server: Option<String>,
}

#[cfg(windows)]
fn reg_command(args: &[&str]) -> Result<String> {
    let output = Command::new("reg").args(args).output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "reg command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(not(windows))]
fn reg_command(_args: &[&str]) -> Result<String> {
    Err(anyhow!(
        "System proxy via registry is supported only on Windows"
    ))
}

fn parse_proxy_enable(output: &str) -> bool {
    output.contains("0x1") || output.contains("0x00000001") || output.contains(" 1")
}

fn parse_proxy_server(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.contains("ProxyServer"))
        .and_then(|line| line.split_whitespace().last())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn get_system_proxy_status() -> Result<SystemProxyStatus> {
    let enabled_out = reg_command(&["query", INTERNET_SETTINGS_KEY, "/v", "ProxyEnable"])?;
    let server_out = reg_command(&["query", INTERNET_SETTINGS_KEY, "/v", "ProxyServer"])?;

    Ok(SystemProxyStatus {
        enabled: parse_proxy_enable(&enabled_out),
        server: parse_proxy_server(&server_out),
    })
}

pub fn set_system_proxy(addr: &str, port: u16) -> Result<()> {
    let server = format!("{}:{}", addr.trim(), port);
    reg_command(&[
        "add",
        INTERNET_SETTINGS_KEY,
        "/v",
        "ProxyServer",
        "/t",
        "REG_SZ",
        "/d",
        &server,
        "/f",
    ])?;
    reg_command(&[
        "add",
        INTERNET_SETTINGS_KEY,
        "/v",
        "ProxyEnable",
        "/t",
        "REG_DWORD",
        "/d",
        "1",
        "/f",
    ])?;
    Ok(())
}

pub fn unset_system_proxy() -> Result<()> {
    reg_command(&[
        "add",
        INTERNET_SETTINGS_KEY,
        "/v",
        "ProxyEnable",
        "/t",
        "REG_DWORD",
        "/d",
        "0",
        "/f",
    ])?;
    Ok(())
}

pub fn is_enabled_for(server: &str) -> Result<bool> {
    let status = get_system_proxy_status()?;
    Ok(status.enabled && status.server.as_deref() == Some(server))
}
