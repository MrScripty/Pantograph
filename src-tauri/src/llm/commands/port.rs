//! Port management commands
//!
//! Commands for checking port availability and resolving conflicts.

use tauri::command;

use crate::constants::ports;
use crate::llm::port_manager::{
    check_port_available, find_available_port, resolve_port_conflict, PortConflictAction,
    PortStatus,
};

/// Check if a port is available and get info about blocking process
#[command]
pub async fn check_port_status(port: Option<u16>) -> Result<PortStatus, String> {
    let target_port = port.unwrap_or(ports::SERVER);
    Ok(check_port_available(target_port))
}

/// Resolve a port conflict using the specified action
#[command]
pub async fn resolve_conflict(
    port: u16,
    action: PortConflictAction,
) -> Result<u16, String> {
    let status = check_port_available(port);
    if status.available {
        return Ok(port);
    }

    resolve_port_conflict(&status, action)
}

/// Find the next available port in the configured range
#[command]
pub async fn find_alternate_port(start: Option<u16>) -> Result<u16, String> {
    let start_port = start.unwrap_or(ports::ALTERNATE_START);
    find_available_port(start_port, ports::ALTERNATE_RANGE)
        .ok_or_else(|| format!("No available ports found starting from {}", start_port))
}

/// Get the default server port
#[command]
pub fn get_default_port() -> u16 {
    ports::SERVER
}
