use std::path::PathBuf;

use tauri::{AppHandle, Manager};

fn bundled_binaries_dir_candidates() -> Vec<PathBuf> {
    [
        std::env::current_dir().ok().map(|p| p.join("binaries")),
        std::env::current_dir()
            .ok()
            .map(|p| p.join("src-tauri").join("binaries")),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|p| p.join("binaries"))
        }),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|p| p.join("src-tauri").join("binaries"))
        }),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("binaries"))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn managed_binaries_dir_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("binaries"))
        .map_err(|e| format!("Failed to get app data dir: {}", e))
}

/// Resolve all binary roots in search order.
pub(crate) fn get_binary_search_roots(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    let managed = managed_binaries_dir_path(app)?;
    roots.push(managed);

    for candidate in bundled_binaries_dir_candidates() {
        if !roots.contains(&candidate) {
            roots.push(candidate);
        }
    }

    Ok(roots)
}

/// Resolve the runtime binaries directory currently preferred for execution.
pub(crate) fn get_binaries_dir(app: &AppHandle) -> Result<PathBuf, String> {
    for candidate in get_binary_search_roots(app)? {
        if candidate.exists() {
            log::debug!("Found binaries dir at: {:?}", candidate);
            return Ok(candidate);
        }
    }

    let fallback = get_managed_binaries_dir(app)?;
    log::warn!("Binaries dir not found, using fallback: {:?}", fallback);
    Ok(fallback)
}

/// Resolve the app-data directory used for managed runtime installs.
pub(crate) fn get_managed_binaries_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let binaries_dir = managed_binaries_dir_path(app)?;
    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    log::debug!("Managed binaries dir: {:?}", binaries_dir);
    Ok(binaries_dir)
}
