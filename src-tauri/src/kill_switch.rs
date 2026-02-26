#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const KILLSWITCH_RECOVERY_INSTRUCTION: &str = "If Relay was closed while kill-switch was active, \
run in an elevated Command Prompt (Run as Administrator):\n\n\
netsh advfirewall firewall delete rule name=RelayKillSwitch_BlockAll\n\
netsh advfirewall firewall delete rule name=RelayKillSwitch_AllowLoopback\n\
netsh advfirewall firewall delete rule name=RelayKillSwitch_AllowUpstreamIPs\n\
for %p in (1024 9050 9051 9060 9150) do @netsh advfirewall firewall delete rule name=RelayKillSwitch_AllowProxy_%p\n\n\
Or delete all Relay kill-switch rules at once:\n\
for /f %n in ('netsh advfirewall firewall show rule name=all ^| findstr /i RelayKillSwitch') do @netsh advfirewall firewall delete rule name=%n";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchConfig {
    pub enabled: bool,
    pub active: bool,
}

impl Default for KillSwitchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
        }
    }
}

pub struct KillSwitchState {
    
    enabled: AtomicBool,
    
    active: AtomicBool,
    
    allowed_ports: Arc<parking_lot::RwLock<Vec<u16>>>,
    
    allowed_upstream_ips: Arc<parking_lot::RwLock<Vec<String>>>,
}

impl KillSwitchState {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            active: AtomicBool::new(false),
            allowed_ports: Arc::new(parking_lot::RwLock::new(Vec::new())),
            allowed_upstream_ips: Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_allowed_ports(&self, ports: Vec<u16>) {
        *self.allowed_ports.write() = ports;
    }

    pub fn set_allowed_upstream_ips(&self, ips: Vec<String>) {
        *self.allowed_upstream_ips.write() = ips;
    }

    pub fn get_config(&self) -> KillSwitchConfig {
        KillSwitchConfig {
            enabled: self.is_enabled(),
            active: self.is_active(),
        }
    }

    pub fn activate(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(anyhow!("Kill-switch is not enabled"));
        }
        if self.is_active() {
            return Ok(());
        }

        let ports = self.allowed_ports.read().clone();
        let upstream_ips = self.allowed_upstream_ips.read().clone();
        tracing::info!(
            "Activating kill-switch, allowed ports: {:?}, upstream IPs: {:?}",
            ports, upstream_ips
        );

        apply_firewall_rules(&ports, &upstream_ips)?;

        self.active.store(true, Ordering::Relaxed);
        tracing::info!("Kill-switch activated");
        Ok(())
    }

    pub fn deactivate(&self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }

        tracing::info!("Deactivating kill-switch");
        remove_firewall_rules()?;

        self.active.store(false, Ordering::Relaxed);
        tracing::info!("Kill-switch deactivated");
        Ok(())
    }
}

impl Drop for KillSwitchState {
    fn drop(&mut self) {
        if self.is_active() {
            tracing::info!("Cleaning up kill-switch on shutdown");
            let _ = remove_firewall_rules();
        }
    }
}

const RULE_NAME_PREFIX: &str = "RelayKillSwitch";

fn relay_config_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("relay")
}

fn rules_state_path() -> std::path::PathBuf {
    relay_config_dir().join("killswitch_rules.json")
}

fn recovery_instruction_path() -> std::path::PathBuf {
    relay_config_dir().join("KILLSWITCH_RECOVERY.txt")
}

#[cfg(target_os = "windows")]
fn write_recovery_instruction_file() {
    let path = recovery_instruction_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, KILLSWITCH_RECOVERY_INSTRUCTION);
}

#[cfg(not(target_os = "windows"))]
fn write_recovery_instruction_file() {}

pub fn cleanup_orphaned_rules() {
    if let Err(e) = remove_firewall_rules() {
        tracing::debug!("Kill-switch cleanup (orphaned rules): {}", e);
    }
}

#[cfg(target_os = "windows")]
fn apply_firewall_rules(allowed_ports: &[u16], allowed_upstream_ips: &[String]) -> Result<()> {
    use std::net::IpAddr;

    let _ = remove_firewall_rules();

    let mut rule_names: Vec<String> = Vec::new();

    let block_output = std::process::Command::new("netsh")
        .args([
            "advfirewall", "firewall", "add", "rule",
            &format!("name={}_BlockAll", RULE_NAME_PREFIX),
            "dir=out",
            "action=block",
            "enable=yes",
        ])
        .creation_flags(0x08000000)
        .output()?;

    if block_output.status.success() {
        rule_names.push(format!("{}_BlockAll", RULE_NAME_PREFIX));
    } else {
        let stderr = String::from_utf8_lossy(&block_output.stderr);
        tracing::warn!("Failed to add block rule: {}", stderr);
    }

    let loopback_rule = format!("{}_AllowLoopback", RULE_NAME_PREFIX);
    let _ = std::process::Command::new("netsh")
        .args([
            "advfirewall", "firewall", "add", "rule",
            &loopback_rule,
            "dir=out",
            "action=allow",
            "remoteip=127.0.0.1,::1",
            "enable=yes",
        ])
        .creation_flags(0x08000000)
        .output()?;
    rule_names.push(loopback_rule);

    let validated_ips: Vec<&str> = allowed_upstream_ips
        .iter()
        .filter_map(|ip| {
            if ip.parse::<IpAddr>().is_ok() {
                Some(ip.as_str())
            } else {
                tracing::warn!(
                    "[kill-switch] Skipping invalid IP in netsh rules (possible injection attempt): {:?}",
                    ip
                );
                None
            }
        })
        .collect();

    if !validated_ips.is_empty() {
        let remoteip = validated_ips.join(",");
        let upstream_rule = format!("{}_AllowUpstreamIPs", RULE_NAME_PREFIX);
        let _ = std::process::Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                &upstream_rule,
                "dir=out",
                "action=allow",
                "protocol=tcp",
                &format!("remoteip={}", remoteip),
                "enable=yes",
            ])
            .creation_flags(0x08000000)
            .output()?;
        rule_names.push(upstream_rule);
    }

    for port in allowed_ports {
        let rule_name = format!("{}_AllowProxy_{}", RULE_NAME_PREFIX, port);
        let _ = std::process::Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                &rule_name,
                "dir=out",
                "action=allow",
                "protocol=tcp",
                &format!("localport={}", port),
                "enable=yes",
            ])
            .creation_flags(0x08000000)
            .output()?;
        rule_names.push(rule_name);
    }

    if let Some(parent) = rules_state_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(&rule_names) {
        let _ = std::fs::write(rules_state_path(), json);
    }
    write_recovery_instruction_file();

    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_firewall_rules() -> Result<()> {
    let path = rules_state_path();
    let rule_names: Vec<String> = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    if rule_names.is_empty() {
        
        for suffix in &["BlockAll", "AllowLoopback", "AllowUpstreamIPs"] {
            let _ = std::process::Command::new("netsh")
                .args([
                    "advfirewall", "firewall", "delete", "rule",
                    &format!("name={}_{}", RULE_NAME_PREFIX, suffix),
                ])
                .creation_flags(0x08000000)
                .output();
        }
        let _ = std::process::Command::new("netsh")
            .args([
                "advfirewall", "firewall", "delete", "rule",
                &format!("name={}_AllowProxy_*", RULE_NAME_PREFIX),
            ])
            .creation_flags(0x08000000)
            .output();
    } else {
        for rule_name in &rule_names {
            let _ = std::process::Command::new("netsh")
                .args(["advfirewall", "firewall", "delete", "rule", &format!("name={}", rule_name)])
                .creation_flags(0x08000000)
                .output();
        }
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(recovery_instruction_path());
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_firewall_rules(allowed_ports: &[u16], allowed_upstream_ips: &[String]) -> Result<()> {
    use std::net::IpAddr;

    let _ = remove_firewall_rules();

    let _ = std::process::Command::new("iptables")
        .args(["-N", RULE_NAME_PREFIX])
        .output();

    let _ = std::process::Command::new("iptables")
        .args(["-A", RULE_NAME_PREFIX, "-o", "lo", "-j", "ACCEPT"])
        .output()?;

    let _ = std::process::Command::new("iptables")
        .args(["-A", RULE_NAME_PREFIX, "-m", "state", "--state", "ESTABLISHED,RELATED", "-j", "ACCEPT"])
        .output()?;

    for ip in allowed_upstream_ips {
        
        if ip.parse::<IpAddr>().is_err() {
            tracing::warn!(
                "[kill-switch] Skipping invalid IP in iptables rules (possible injection attempt): {:?}",
                ip
            );
            continue;
        }
        let _ = std::process::Command::new("iptables")
            .args(["-A", RULE_NAME_PREFIX, "-p", "tcp", "-d", ip, "-j", "ACCEPT"])
            .output()?;
    }

    for port in allowed_ports {
        let _ = std::process::Command::new("iptables")
            .args(["-A", RULE_NAME_PREFIX, "-p", "tcp", "--sport", &port.to_string(), "-j", "ACCEPT"])
            .output()?;
    }

    let _ = std::process::Command::new("iptables")
        .args(["-A", RULE_NAME_PREFIX, "-j", "DROP"])
        .output()?;

    let _ = std::process::Command::new("iptables")
        .args(["-I", "OUTPUT", "-j", RULE_NAME_PREFIX])
        .output()?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn remove_firewall_rules() -> Result<()> {
    let _ = std::process::Command::new("iptables")
        .args(["-D", "OUTPUT", "-j", RULE_NAME_PREFIX])
        .output();
    let _ = std::process::Command::new("iptables")
        .args(["-F", RULE_NAME_PREFIX])
        .output();
    let _ = std::process::Command::new("iptables")
        .args(["-X", RULE_NAME_PREFIX])
        .output();
    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_firewall_rules(allowed_ports: &[u16], allowed_upstream_ips: &[String]) -> Result<()> {
    let _ = remove_firewall_rules();

    let anchor_name = RULE_NAME_PREFIX;
    let mut rules = String::new();

    rules.push_str("pass out on lo0 all\n");
    for ip in allowed_upstream_ips {
        
        if ip.parse::<std::net::IpAddr>().is_err() {
            tracing::warn!(
                "[kill-switch] Skipping invalid IP in pfctl rules (possible injection attempt): {:?}",
                ip
            );
            continue;
        }
        rules.push_str(&format!("pass out proto tcp to {}\n", ip));
    }
    for port in allowed_ports {
        rules.push_str(&format!("pass out proto tcp from any port {}\n", port));
    }

    rules.push_str("block out all\n");

    let rules_path = std::env::temp_dir().join("relay_killswitch.conf");
    std::fs::write(&rules_path, &rules)?;

    let _ = std::process::Command::new("pfctl")
        .args(["-a", anchor_name, "-f", &rules_path.to_string_lossy()])
        .output()?;

    let _ = std::process::Command::new("pfctl")
        .args(["-e"])
        .output();

    Ok(())
}

#[cfg(target_os = "macos")]
fn remove_firewall_rules() -> Result<()> {
    let _ = std::process::Command::new("pfctl")
        .args(["-a", RULE_NAME_PREFIX, "-F", "all"])
        .output();
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn apply_firewall_rules(_allowed_ports: &[u16], _allowed_upstream_ips: &[String]) -> Result<()> {
    Err(anyhow!("Kill-switch is not supported on this platform"))
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn remove_firewall_rules() -> Result<()> {
    Ok(())
}
