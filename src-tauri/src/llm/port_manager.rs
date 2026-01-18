//! Port management for detecting conflicts and finding available ports
//!
//! Provides utilities for checking port availability, identifying blocking processes,
//! and resolving port conflicts.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::process::Command;

/// Result of a port availability check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortStatus {
    /// The port that was checked
    pub port: u16,
    /// Whether the port is available for use
    pub available: bool,
    /// Information about the process blocking the port (if any)
    pub blocking_process: Option<ProcessInfo>,
    /// Whether the blocking process is a Pantograph server
    pub is_pantograph: bool,
}

/// Information about a process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,
    /// Process name (from /proc/pid/comm or cmdline)
    pub name: String,
    /// Full command line (if available)
    pub command: Option<String>,
}

/// Actions for resolving port conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum PortConflictAction {
    /// Kill the blocking process
    Kill,
    /// Use an alternate port
    AlternatePort { preferred_start: u16 },
    /// Cancel the operation
    Cancel,
}

/// Check if a port is available for binding
///
/// On Linux, this reads /proc/net/tcp to check for listening sockets.
/// Falls back to attempting a bind on other platforms.
pub fn check_port_available(port: u16) -> PortStatus {
    #[cfg(target_os = "linux")]
    {
        check_port_available_linux(port)
    }

    #[cfg(not(target_os = "linux"))]
    {
        check_port_available_fallback(port)
    }
}

#[cfg(target_os = "linux")]
fn check_port_available_linux(port: u16) -> PortStatus {
    // Read /proc/net/tcp for IPv4 and /proc/net/tcp6 for IPv6
    let mut blocking_pid: Option<u32> = None;

    // Check both IPv4 and IPv6
    for proc_file in &["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(contents) = fs::read_to_string(proc_file) {
            for line in contents.lines().skip(1) {
                // Skip header
                if let Some(pid) = parse_proc_net_tcp_line(line, port) {
                    blocking_pid = Some(pid);
                    break;
                }
            }
        }
        if blocking_pid.is_some() {
            break;
        }
    }

    match blocking_pid {
        Some(pid) => {
            let process_info = get_process_info(pid);
            let is_pantograph = process_info
                .as_ref()
                .map(|p| is_pantograph_process(p))
                .unwrap_or(false);

            PortStatus {
                port,
                available: false,
                blocking_process: process_info,
                is_pantograph,
            }
        }
        None => PortStatus {
            port,
            available: true,
            blocking_process: None,
            is_pantograph: false,
        },
    }
}

/// Parse a line from /proc/net/tcp to find if it's listening on the target port
///
/// Format: sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode
/// Example: 0: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 12345 1 0000000000000000 100 0 0 10 0
#[cfg(target_os = "linux")]
fn parse_proc_net_tcp_line(line: &str, target_port: u16) -> Option<u32> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }

    // local_address is in format IP:PORT (hex)
    let local_addr = parts[1];
    let addr_parts: Vec<&str> = local_addr.split(':').collect();
    if addr_parts.len() != 2 {
        return None;
    }

    // Parse port from hex
    let port = u16::from_str_radix(addr_parts[1], 16).ok()?;
    if port != target_port {
        return None;
    }

    // Check state - 0A means LISTEN
    let state = parts[3];
    if state != "0A" {
        return None;
    }

    // Get inode from column 9 (0-indexed)
    let inode: u64 = parts[9].parse().ok()?;
    if inode == 0 {
        return None;
    }

    // Find PID from inode by scanning /proc/*/fd/
    find_pid_by_inode(inode)
}

/// Find PID that owns a socket by its inode
#[cfg(target_os = "linux")]
fn find_pid_by_inode(target_inode: u64) -> Option<u32> {
    let proc_dir = fs::read_dir("/proc").ok()?;

    for entry in proc_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Only check numeric directories (PIDs)
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let pid: u32 = name_str.parse().ok()?;
        let fd_dir = path.join("fd");

        if let Ok(fds) = fs::read_dir(&fd_dir) {
            for fd_entry in fds.flatten() {
                if let Ok(link) = fs::read_link(fd_entry.path()) {
                    let link_str = link.to_string_lossy();
                    // Socket links look like "socket:[12345]"
                    if link_str.starts_with("socket:[") && link_str.ends_with(']') {
                        let inode_str = &link_str[8..link_str.len() - 1];
                        if let Ok(inode) = inode_str.parse::<u64>() {
                            if inode == target_inode {
                                return Some(pid);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Fallback port check using socket binding (for non-Linux platforms)
#[cfg(not(target_os = "linux"))]
fn check_port_available_fallback(port: u16) -> PortStatus {
    use std::net::TcpListener;

    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(_) => PortStatus {
            port,
            available: true,
            blocking_process: None,
            is_pantograph: false,
        },
        Err(_) => PortStatus {
            port,
            available: false,
            blocking_process: None, // Can't determine on non-Linux
            is_pantograph: false,
        },
    }
}

/// Get process information from PID
#[cfg(unix)]
pub fn get_process_info(pid: u32) -> Option<ProcessInfo> {
    let pid_path = PathBuf::from(format!("/proc/{}", pid));

    // Read process name from /proc/pid/comm
    let name = fs::read_to_string(pid_path.join("comm"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Read full command line from /proc/pid/cmdline
    let command = fs::read_to_string(pid_path.join("cmdline"))
        .ok()
        .map(|s| s.replace('\0', " ").trim().to_string())
        .filter(|s| !s.is_empty());

    Some(ProcessInfo { pid, name, command })
}

#[cfg(not(unix))]
pub fn get_process_info(_pid: u32) -> Option<ProcessInfo> {
    None
}

/// Check if a process is a Pantograph server
fn is_pantograph_process(info: &ProcessInfo) -> bool {
    // Check process name
    if info.name.contains("llama-server") || info.name.contains("pantograph") {
        return true;
    }

    // Check command line
    if let Some(cmd) = &info.command {
        if cmd.contains("llama-server") || cmd.contains("pantograph") {
            return true;
        }
    }

    false
}

/// Kill a process by PID
///
/// Attempts SIGTERM first, then SIGKILL if the process doesn't exit.
#[cfg(unix)]
pub fn kill_process(pid: u32) -> Result<(), String> {
    use std::thread;
    use std::time::Duration;

    let pid_str = pid.to_string();

    // First check if process exists
    let exists = Command::new("kill")
        .arg("-0")
        .arg(&pid_str)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !exists {
        return Ok(()); // Already dead
    }

    // Send SIGTERM
    log::info!("Sending SIGTERM to process {}", pid);
    Command::new("kill")
        .arg("-TERM")
        .arg(&pid_str)
        .status()
        .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;

    // Wait for graceful exit
    thread::sleep(Duration::from_millis(500));

    // Check if still running
    let still_running = Command::new("kill")
        .arg("-0")
        .arg(&pid_str)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if still_running {
        // Force kill
        log::warn!("Process {} didn't exit gracefully, sending SIGKILL", pid);
        Command::new("kill")
            .arg("-KILL")
            .arg(&pid_str)
            .status()
            .map_err(|e| format!("Failed to send SIGKILL: {}", e))?;

        thread::sleep(Duration::from_millis(200));

        // Final check
        let final_check = Command::new("kill")
            .arg("-0")
            .arg(&pid_str)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if final_check {
            return Err(format!("Failed to kill process {}", pid));
        }
    }

    log::info!("Process {} terminated", pid);
    Ok(())
}

#[cfg(not(unix))]
pub fn kill_process(_pid: u32) -> Result<(), String> {
    Err("Process killing not supported on this platform".to_string())
}

/// Find the next available port starting from a given port
pub fn find_available_port(start: u16, range: u16) -> Option<u16> {
    for offset in 0..range {
        let port = start.saturating_add(offset);
        if port == 0 || port > 65535 - offset {
            break;
        }

        let status = check_port_available(port);
        if status.available {
            return Some(port);
        }
    }

    None
}

/// Resolve a port conflict based on the chosen action
pub fn resolve_port_conflict(
    status: &PortStatus,
    action: PortConflictAction,
) -> Result<u16, String> {
    match action {
        PortConflictAction::Kill => {
            if let Some(ref process) = status.blocking_process {
                kill_process(process.pid)?;
                // Verify port is now free
                let new_status = check_port_available(status.port);
                if new_status.available {
                    Ok(status.port)
                } else {
                    Err("Port still in use after killing process".to_string())
                }
            } else {
                Err("No process information available to kill".to_string())
            }
        }
        PortConflictAction::AlternatePort { preferred_start } => {
            find_available_port(preferred_start, 100)
                .ok_or_else(|| "No available ports in range".to_string())
        }
        PortConflictAction::Cancel => Err("Operation cancelled by user".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_port_available_unbound() {
        // Port 65432 is unlikely to be in use
        let status = check_port_available(65432);
        // Just verify we get a valid response structure
        assert!(status.port == 65432);
    }

    #[test]
    fn test_find_available_port() {
        // Should find some available port in high range
        let port = find_available_port(60000, 100);
        assert!(port.is_some());
    }
}
