use super::archive::extract_archive;
use super::contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeReadinessState, ManagedRuntimeSnapshot,
    ManagedRuntimeVersionStatus, ResolvedCommand,
};
use super::definitions::definition;
use super::paths::{extract_pid_file, managed_install_dir, managed_runtime_dir};
use super::state::{
    load_managed_runtime_state, runtime_state_entry, ManagedRuntimePersistedRuntime,
};
use futures_util::TryStreamExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

static TRANSITION_LOCKS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<tokio::sync::Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn transition_lock(id: ManagedBinaryId) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = TRANSITION_LOCKS.lock();
    locks
        .entry(id)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

pub async fn check_binary_status(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<BinaryStatus, String> {
    let capability = binary_capability(app_data_dir, id)?;
    Ok(BinaryStatus {
        available: capability.available,
        missing_files: capability.missing_files,
    })
}

pub fn binary_capability(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<ManagedBinaryCapability, String> {
    let definition = definition(id);
    let install_dir = managed_install_dir(app_data_dir, id);
    let has_managed_install = install_dir.exists();

    if definition.system_command().is_some() {
        return Ok(ManagedBinaryCapability {
            id,
            display_name: definition.display_name().to_string(),
            install_state: ManagedBinaryInstallState::SystemProvided,
            available: true,
            can_install: false,
            can_remove: has_managed_install,
            missing_files: Vec::new(),
            unavailable_reason: None,
        });
    }

    let release_asset = definition.release_asset();
    if let Err(reason) = release_asset {
        return Ok(ManagedBinaryCapability {
            id,
            display_name: definition.display_name().to_string(),
            install_state: ManagedBinaryInstallState::Unsupported,
            available: false,
            can_install: false,
            can_remove: has_managed_install,
            missing_files: Vec::new(),
            unavailable_reason: Some(reason),
        });
    }

    let missing_files = definition.validate_installation(&install_dir);
    let install_state = if missing_files.is_empty() {
        ManagedBinaryInstallState::Installed
    } else {
        ManagedBinaryInstallState::Missing
    };

    Ok(ManagedBinaryCapability {
        id,
        display_name: definition.display_name().to_string(),
        install_state,
        available: missing_files.is_empty(),
        can_install: !missing_files.is_empty(),
        can_remove: has_managed_install,
        missing_files,
        unavailable_reason: None,
    })
}

pub fn list_binary_capabilities(
    app_data_dir: &Path,
) -> Result<Vec<ManagedBinaryCapability>, String> {
    ManagedBinaryId::all()
        .iter()
        .copied()
        .map(|id| binary_capability(app_data_dir, id))
        .collect()
}

pub fn managed_runtime_snapshot(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<ManagedRuntimeSnapshot, String> {
    let capability = binary_capability(app_data_dir, id)?;
    let state = load_managed_runtime_state(app_data_dir)?;
    Ok(snapshot_from_capability(
        &capability,
        runtime_state_entry(&state, id),
    ))
}

pub fn list_managed_runtime_snapshots(
    app_data_dir: &Path,
) -> Result<Vec<ManagedRuntimeSnapshot>, String> {
    let state = load_managed_runtime_state(app_data_dir)?;
    list_binary_capabilities(app_data_dir).map(|capabilities| {
        capabilities
            .iter()
            .map(|capability| {
                snapshot_from_capability(capability, runtime_state_entry(&state, capability.id))
            })
            .collect::<Vec<_>>()
    })
}

pub async fn download_binary<F>(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(DownloadProgress),
{
    let lock = transition_lock(id);
    let _guard = lock.lock().await;
    let definition = definition(id);
    if definition.system_command().is_some() {
        return Err(format!(
            "{} is already available from the system PATH",
            definition.display_name()
        ));
    }
    let runtime_root = managed_runtime_dir(app_data_dir);
    let install_dir = managed_install_dir(app_data_dir, id);
    let release_asset = definition.release_asset()?;
    let download_url = definition.download_url(&release_asset);

    fs::create_dir_all(&runtime_root)
        .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    on_progress(DownloadProgress {
        status: format!("Downloading {} binaries...", definition.display_name()),
        current: 0,
        total: 0,
        done: false,
        error: None,
    });

    log::info!(
        "Downloading {} from: {}",
        definition.display_name(),
        download_url
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let temp_path = runtime_root.join(format!(
        ".{}-{}",
        uuid::Uuid::new_v4(),
        release_asset.archive_name
    ));
    let mut file =
        fs::File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded = 0_u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream
        .try_next()
        .await
        .map_err(|e| format!("Download error: {}", e))?
    {
        use std::io::Write;

        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;
        downloaded += chunk.len() as u64;

        on_progress(DownloadProgress {
            status: "Downloading...".to_string(),
            current: downloaded,
            total: total_size,
            done: false,
            error: None,
        });
    }
    drop(file);

    on_progress(DownloadProgress {
        status: "Extracting...".to_string(),
        current: total_size,
        total: total_size,
        done: false,
        error: None,
    });

    let extract_dir = runtime_root.join(format!(
        ".{}-extract-{}",
        id.install_dir_name(),
        uuid::Uuid::new_v4()
    ));
    let staging_dir = runtime_root.join(format!(
        ".{}-staging-{}",
        id.install_dir_name(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create staging directory: {}", e))?;

    let extraction_result = extract_archive(&temp_path, &extract_dir, release_asset.archive_kind)
        .and_then(|_| definition.install_distribution(&extract_dir, &staging_dir));

    let _ = fs::remove_dir_all(&extract_dir);
    let _ = fs::remove_file(&temp_path);

    if let Err(error) = extraction_result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(error);
    }

    let missing = definition.validate_installation(&staging_dir);
    if let Some(first_missing) = missing.first() {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(format!(
            "{} extraction completed but runtime file is still missing: {}",
            definition.display_name(),
            first_missing
        ));
    }

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)
            .map_err(|e| format!("Failed to replace existing install: {}", e))?;
    }
    if let Some(parent) = install_dir.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create install directory: {}", e))?;
    }
    fs::rename(&staging_dir, &install_dir)
        .map_err(|e| format!("Failed to finalize install: {}", e))?;

    on_progress(DownloadProgress {
        status: "Complete".to_string(),
        current: total_size,
        total: total_size,
        done: true,
        error: None,
    });

    log::info!(
        "{} binaries downloaded and extracted successfully",
        definition.display_name()
    );
    Ok(())
}

pub async fn remove_binary(app_data_dir: &Path, id: ManagedBinaryId) -> Result<(), String> {
    let lock = transition_lock(id);
    let _guard = lock.lock().await;

    let install_dir = managed_install_dir(app_data_dir, id);
    if !install_dir.exists() {
        return Ok(());
    }

    fs::remove_dir_all(&install_dir).map_err(|e| {
        format!(
            "Failed to remove {} runtime directory {:?}: {}",
            definition(id).display_name(),
            install_dir,
            e
        )
    })
}

pub fn resolve_binary_command(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    args: &[&str],
) -> Result<ResolvedCommand, String> {
    let definition = definition(id);

    if let Some(executable_path) = definition.system_command() {
        let (args, pid_file) = extract_pid_file(args);
        let working_directory = executable_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        return Ok(ResolvedCommand {
            executable_path,
            working_directory,
            args,
            env_overrides: Vec::new(),
            pid_file,
        });
    }

    let install_dir = managed_install_dir(app_data_dir, id);
    let missing = definition.validate_installation(&install_dir);
    if let Some(first_missing) = missing.first() {
        return Err(format!(
            "{} binaries are not installed for the current platform (missing {})",
            definition.display_name(),
            first_missing
        ));
    }

    definition.resolve_command(&install_dir, args)
}

fn snapshot_from_capability(
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
) -> ManagedRuntimeSnapshot {
    let selection = persisted_runtime
        .map(|runtime| runtime.selection.clone())
        .unwrap_or_default();
    let active_job = persisted_runtime.and_then(|runtime| runtime.active_job.clone());
    let versions = persisted_runtime
        .filter(|runtime| !runtime.versions.is_empty())
        .map(|runtime| {
            runtime
                .versions
                .iter()
                .map(|version| ManagedRuntimeVersionStatus {
                    version: Some(version.version.clone()),
                    display_label: version.version.clone(),
                    install_state: capability.install_state,
                    readiness_state: version.readiness_state,
                    selected: selection.selected_version.as_deref()
                        == Some(version.version.as_str()),
                    active: selection.active_version.as_deref() == Some(version.version.as_str()),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![version_status_for_capability(capability)]);

    ManagedRuntimeSnapshot {
        id: capability.id,
        display_name: capability.display_name.clone(),
        install_state: capability.install_state,
        readiness_state: readiness_state_for_capability(capability),
        available: capability.available,
        can_install: capability.can_install,
        can_remove: capability.can_remove,
        missing_files: capability.missing_files.clone(),
        unavailable_reason: capability.unavailable_reason.clone(),
        versions,
        selection,
        active_job,
    }
}

fn readiness_state_for_capability(
    capability: &ManagedBinaryCapability,
) -> ManagedRuntimeReadinessState {
    match capability.install_state {
        ManagedBinaryInstallState::Installed | ManagedBinaryInstallState::SystemProvided => {
            ManagedRuntimeReadinessState::Ready
        }
        ManagedBinaryInstallState::Missing => ManagedRuntimeReadinessState::Missing,
        ManagedBinaryInstallState::Unsupported => ManagedRuntimeReadinessState::Unsupported,
    }
}

fn version_status_for_capability(
    capability: &ManagedBinaryCapability,
) -> ManagedRuntimeVersionStatus {
    ManagedRuntimeVersionStatus {
        version: None,
        display_label: match capability.install_state {
            ManagedBinaryInstallState::SystemProvided => "System provided".to_string(),
            ManagedBinaryInstallState::Installed => "Managed install".to_string(),
            ManagedBinaryInstallState::Missing => "No managed install".to_string(),
            ManagedBinaryInstallState::Unsupported => "Unsupported platform".to_string(),
        },
        install_state: capability.install_state,
        readiness_state: readiness_state_for_capability(capability),
        selected: false,
        active: capability.available,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        readiness_state_for_capability, snapshot_from_capability, ManagedBinaryCapability,
        ManagedBinaryId, ManagedBinaryInstallState, ManagedRuntimeReadinessState,
    };

    fn capability(install_state: ManagedBinaryInstallState) -> ManagedBinaryCapability {
        ManagedBinaryCapability {
            id: ManagedBinaryId::LlamaCpp,
            display_name: "llama.cpp".to_string(),
            install_state,
            available: matches!(
                install_state,
                ManagedBinaryInstallState::Installed | ManagedBinaryInstallState::SystemProvided
            ),
            can_install: install_state == ManagedBinaryInstallState::Missing,
            can_remove: install_state != ManagedBinaryInstallState::Unsupported,
            missing_files: Vec::new(),
            unavailable_reason: None,
        }
    }

    #[test]
    fn readiness_state_maps_installed_runtime_to_ready() {
        let readiness =
            readiness_state_for_capability(&capability(ManagedBinaryInstallState::Installed));
        assert_eq!(readiness, ManagedRuntimeReadinessState::Ready);
    }

    #[test]
    fn readiness_state_maps_missing_runtime_to_missing() {
        let readiness =
            readiness_state_for_capability(&capability(ManagedBinaryInstallState::Missing));
        assert_eq!(readiness, ManagedRuntimeReadinessState::Missing);
    }

    #[test]
    fn snapshot_carries_additive_versions_and_selection_contracts() {
        let snapshot =
            snapshot_from_capability(&capability(ManagedBinaryInstallState::Installed), None);

        assert_eq!(snapshot.versions.len(), 1);
        assert_eq!(snapshot.selection.selected_version, None);
        assert_eq!(
            snapshot.readiness_state,
            ManagedRuntimeReadinessState::Ready
        );
        assert!(snapshot.active_job.is_none());
    }
}
