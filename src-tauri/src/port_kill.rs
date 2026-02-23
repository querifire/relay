//! Kill any process listening on a given port (so Tor or another server can bind).
//! Used before starting Tor when the port is already in use.

/// System/critical ports we must not kill (could break OS or critical services).
const PROTECTED_PORTS: &[u16] = &[
    21, 22, 23, 25, 53, 80, 110, 143, 443, 445, 465, 587, 993, 995, 3306, 5432, 6379, 8080, 8443,
];

/// Returns true if something is listening on `port`, false otherwise. Use this to fail with a
/// user-facing error instead of killing processes automatically.
pub fn port_is_in_use(port: u16) -> Result<bool, String> {
    let pids = pids_listening_on_port(port)?;
    Ok(!pids.is_empty())
}

/// Try to kill process(es) listening on `port`.
/// Refuses to touch protected/system ports. On Windows uses netstat + taskkill; on Unix uses lsof + kill.
/// Returns Ok(true) if at least one process was killed, Ok(false) if port was free, Err on failure.
/// Prefer failing with a clear error and asking the user to free the port instead of calling this.
pub fn kill_process_on_port(port: u16) -> Result<bool, String> {
    if PROTECTED_PORTS.contains(&port) {
        return Err(format!(
            "Port {} is protected (system/critical); refusing to kill process",
            port
        ));
    }
    if port < 1024 {
        return Err(format!(
            "Port {} is in the system range (1-1023); refusing to kill process",
            port
        ));
    }
    let pids = pids_listening_on_port(port)?;
    if pids.is_empty() {
        return Ok(false);
    }
    for pid in pids {
        kill_pid(pid)?;
    }
    Ok(true)
}

#[cfg(windows)]
fn pids_listening_on_port(port: u16) -> Result<Vec<u32>, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let port_str = format!(":{}", port);
    let output = Command::new("netstat")
        .args(["-ano"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("netstat failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pids = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if !line.contains("LISTENING") {
            continue;
        }
        // Match port as whole token (e.g. :905 not :9050)
        if !port_match_in_line(line, &port_str) {
            continue;
        }
        // Last column is PID (e.g. "  TCP    127.0.0.1:905    0.0.0.0:0    LISTENING    12345")
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(last) = parts.last() {
            if let Ok(pid) = last.parse::<u32>() {
                pids.push(pid);
            }
        }
    }
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

#[cfg(windows)]
fn port_match_in_line(line: &str, port_str: &str) -> bool {
    let mut start = 0;
    while let Some(i) = line[start..].find(port_str) {
        let pos = start + i;
        let after_port = pos + port_str.len();
        let next_char = line[after_port..].chars().next();
        if next_char.map(|c| !c.is_ascii_digit()).unwrap_or(true) {
            return true;
        }
        start = after_port;
    }
    false
}

#[cfg(windows)]
fn kill_pid(pid: u32) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .creation_flags(0x08000000)
        .status()
        .map_err(|e| format!("taskkill failed: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("taskkill returned {}", status))
    }
}

#[cfg(unix)]
fn pids_listening_on_port(port: u16) -> Result<Vec<u32>, String> {
    use std::process::Command;
    let output = Command::new("lsof")
        .args(["-i", &format!(":{}", port), "-t"])
        .output()
        .map_err(|e| format!("lsof failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<u32> = stdout
        .lines()
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    Ok(pids)
}

#[cfg(unix)]
fn kill_pid(pid: u32) -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .map_err(|e| format!("kill failed: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("kill returned {}", status))
    }
}
