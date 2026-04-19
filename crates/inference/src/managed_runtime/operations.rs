use super::archive::extract_archive;
use super::contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeJobState, ManagedRuntimeJobStatus,
    ManagedRuntimeReadinessState, ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus,
    ResolvedCommand,
};
use super::definitions::definition;
use super::paths::{
    extract_pid_file, managed_install_dir, managed_runtime_dir, managed_version_install_dir,
};
use super::state::{
    ensure_runtime_state_entry, load_managed_runtime_state, runtime_state_entry,
    runtime_state_entry_mut, save_managed_runtime_state, ManagedRuntimeHistoryEventKind,
    ManagedRuntimeInstallHistoryEntry, ManagedRuntimePersistedRuntime,
    ManagedRuntimePersistedVersion,
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
    let install_dir = runtime_install_dir_for_projection(app_data_dir, id)?;
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

pub fn select_managed_runtime_version(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: Option<&str>,
) -> Result<(), String> {
    update_runtime_selection(app_data_dir, id, version, SelectionTarget::Selected)
}

pub fn set_default_managed_runtime_version(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: Option<&str>,
) -> Result<(), String> {
    update_runtime_selection(app_data_dir, id, version, SelectionTarget::Default)
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
    let runtime_version = definition.release_version().to_string();
    if definition.system_command().is_some() {
        return Err(format!(
            "{} is already available from the system PATH",
            definition.display_name()
        ));
    }
    let runtime_root = managed_runtime_dir(app_data_dir);
    let install_dir = managed_version_install_dir(app_data_dir, id, &runtime_version);
    let release_asset = definition.release_asset()?;
    let download_url = definition.download_url(&release_asset);

    fs::create_dir_all(&runtime_root)
        .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    persist_active_job(
        app_data_dir,
        id,
        ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Queued,
            status: format!("Queued {} install", definition.display_name()),
            current: 0,
            total: 0,
            resumable: false,
            cancellable: false,
            error: None,
        },
    )?;

    on_progress(DownloadProgress {
        status: format!("Downloading {} binaries...", definition.display_name()),
        current: 0,
        total: 0,
        done: false,
        error: None,
    });

    persist_active_job(
        app_data_dir,
        id,
        ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Downloading,
            status: format!("Downloading {}", definition.display_name()),
            current: 0,
            total: 0,
            resumable: false,
            cancellable: false,
            error: None,
        },
    )?;

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

        persist_active_job(
            app_data_dir,
            id,
            ManagedRuntimeJobStatus {
                state: ManagedRuntimeJobState::Downloading,
                status: "Downloading".to_string(),
                current: downloaded,
                total: total_size,
                resumable: false,
                cancellable: false,
                error: None,
            },
        )?;
    }
    drop(file);

    on_progress(DownloadProgress {
        status: "Extracting...".to_string(),
        current: total_size,
        total: total_size,
        done: false,
        error: None,
    });

    persist_active_job(
        app_data_dir,
        id,
        ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Extracting,
            status: "Extracting".to_string(),
            current: total_size,
            total: total_size,
            resumable: false,
            cancellable: false,
            error: None,
        },
    )?;

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
        persist_failed_job(
            app_data_dir,
            id,
            &runtime_version,
            "Install failed".to_string(),
            error.clone(),
        )?;
        return Err(error);
    }

    persist_active_job(
        app_data_dir,
        id,
        ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Validating,
            status: "Validating".to_string(),
            current: total_size,
            total: total_size,
            resumable: false,
            cancellable: false,
            error: None,
        },
    )?;

    let missing = definition.validate_installation(&staging_dir);
    if let Some(first_missing) = missing.first() {
        let _ = fs::remove_dir_all(&staging_dir);
        let error = format!(
            "{} extraction completed but runtime file is still missing: {}",
            definition.display_name(),
            first_missing
        );
        persist_failed_job(
            app_data_dir,
            id,
            &runtime_version,
            "Validation failed".to_string(),
            error.clone(),
        )?;
        return Err(error);
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

    persist_install_success(app_data_dir, id, &runtime_version, &install_dir)?;
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
    })?;

    persist_remove_success(app_data_dir, id)?;
    Ok(())
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

    let install_dir = resolve_runtime_install_dir(app_data_dir, id)?;
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

fn persist_active_job(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    job: ManagedRuntimeJobStatus,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = Some(job);
    save_managed_runtime_state(app_data_dir, &state)
}

fn persist_failed_job(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    status: String,
    error: String,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Failed,
        status,
        current: 0,
        total: 0,
        resumable: false,
        cancellable: false,
        error: Some(error.clone()),
    });
    upsert_persisted_version(
        runtime,
        ManagedRuntimePersistedVersion {
            version: version.to_string(),
            readiness_state: ManagedRuntimeReadinessState::Failed,
            install_root: None,
            last_ready_at_ms: None,
            last_error: Some(error.clone()),
        },
    );
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            event: ManagedRuntimeHistoryEventKind::ValidationFailed,
            version: Some(version.to_string()),
            at_ms: current_unix_timestamp_ms(),
            detail: Some(error),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

fn persist_install_success(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    install_dir: &Path,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = None;
    upsert_persisted_version(
        runtime,
        ManagedRuntimePersistedVersion {
            version: version.to_string(),
            readiness_state: ManagedRuntimeReadinessState::Ready,
            install_root: Some(install_dir.display().to_string()),
            last_ready_at_ms: Some(current_unix_timestamp_ms()),
            last_error: None,
        },
    );
    if runtime.selection.selected_version.is_none() {
        runtime.selection.selected_version = Some(version.to_string());
    }
    if runtime.selection.default_version.is_none() {
        runtime.selection.default_version = Some(version.to_string());
    }
    runtime.selection.active_version = Some(version.to_string());
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            event: ManagedRuntimeHistoryEventKind::Installed,
            version: Some(version.to_string()),
            at_ms: current_unix_timestamp_ms(),
            detail: Some(format!("Installed into {}", install_dir.display())),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

fn persist_remove_success(app_data_dir: &Path, id: ManagedBinaryId) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let Some(runtime) = runtime_state_entry_mut(&mut state, id) else {
        return Ok(());
    };

    let removed_versions = runtime
        .versions
        .iter()
        .map(|version| version.version.clone())
        .collect::<Vec<_>>();
    runtime.versions.clear();
    runtime.active_job = None;
    if runtime
        .selection
        .selected_version
        .as_ref()
        .is_some_and(|selected| removed_versions.contains(selected))
    {
        runtime.selection.selected_version = None;
    }
    if runtime
        .selection
        .default_version
        .as_ref()
        .is_some_and(|default| removed_versions.contains(default))
    {
        runtime.selection.default_version = None;
    }
    runtime.selection.active_version = None;
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            event: ManagedRuntimeHistoryEventKind::Removed,
            version: removed_versions.first().cloned(),
            at_ms: current_unix_timestamp_ms(),
            detail: Some("Managed install removed".to_string()),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

fn upsert_persisted_version(
    runtime: &mut ManagedRuntimePersistedRuntime,
    version: ManagedRuntimePersistedVersion,
) {
    if let Some(existing) = runtime
        .versions
        .iter_mut()
        .find(|existing| existing.version == version.version)
    {
        *existing = version;
        return;
    }

    runtime.versions.push(version);
}

fn current_unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

enum SelectionTarget {
    Selected,
    Default,
}

fn update_runtime_selection(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: Option<&str>,
    target: SelectionTarget,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);

    if let Some(version) = version {
        if !runtime
            .versions
            .iter()
            .any(|entry| entry.version == version)
        {
            return Err(format!(
                "{} version '{}' is not installed",
                id.display_name(),
                version
            ));
        }
    }

    let version = version.map(str::to_string);
    match target {
        SelectionTarget::Selected => {
            runtime.selection.selected_version = version.clone();
        }
        SelectionTarget::Default => {
            runtime.selection.default_version = version.clone();
        }
    }

    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            event: ManagedRuntimeHistoryEventKind::SelectionUpdated,
            version,
            at_ms: current_unix_timestamp_ms(),
            detail: Some(selection_target_label(target).to_string()),
        });

    save_managed_runtime_state(app_data_dir, &state)
}

fn resolve_runtime_install_dir(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<PathBuf, String> {
    resolve_runtime_install_dir_with_mode(app_data_dir, id, InstallDirResolutionMode::Strict)
}

fn runtime_install_dir_for_projection(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<PathBuf, String> {
    resolve_runtime_install_dir_with_mode(app_data_dir, id, InstallDirResolutionMode::BestEffort)
}

enum InstallDirResolutionMode {
    Strict,
    BestEffort,
}

fn resolve_runtime_install_dir_with_mode(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    mode: InstallDirResolutionMode,
) -> Result<PathBuf, String> {
    let state = load_managed_runtime_state(app_data_dir)?;
    let fallback_install_dir = managed_install_dir(app_data_dir, id);
    let Some(runtime) = runtime_state_entry(&state, id) else {
        return Ok(fallback_install_dir);
    };

    let preferred_version = runtime
        .selection
        .selected_version
        .as_deref()
        .or(runtime.selection.active_version.as_deref())
        .or(runtime.selection.default_version.as_deref());

    let Some(version) = preferred_version else {
        return Ok(fallback_install_dir);
    };

    let Some(persisted_version) = runtime
        .versions
        .iter()
        .find(|entry| entry.version == version)
    else {
        return match mode {
            InstallDirResolutionMode::Strict => Err(format!(
                "{} selected version '{}' is not installed",
                id.display_name(),
                version
            )),
            InstallDirResolutionMode::BestEffort => Ok(fallback_install_dir),
        };
    };

    Ok(persisted_version
        .install_root
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or(fallback_install_dir))
}

fn selection_target_label(target: SelectionTarget) -> &'static str {
    match target {
        SelectionTarget::Selected => "selected_version_updated",
        SelectionTarget::Default => "default_version_updated",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        binary_capability, persist_install_success, persist_remove_success,
        readiness_state_for_capability, resolve_runtime_install_dir,
        runtime_install_dir_for_projection, select_managed_runtime_version,
        set_default_managed_runtime_version, snapshot_from_capability, ManagedBinaryCapability,
        ManagedBinaryId, ManagedBinaryInstallState, ManagedRuntimeReadinessState,
    };
    use crate::managed_runtime::{
        load_managed_runtime_state, save_managed_runtime_state, ManagedRuntimePersistedVersion,
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

    #[test]
    fn persist_install_success_records_ready_version_and_selection() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");

        let state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");

        assert_eq!(runtime.versions.len(), 1);
        assert_eq!(runtime.versions[0].version, "b8248");
        assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
        assert_eq!(runtime.selection.active_version.as_deref(), Some("b8248"));
    }

    #[test]
    fn persist_remove_success_clears_versions_and_selection() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");
        persist_remove_success(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect("persist remove success");

        let state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");

        assert!(runtime.versions.is_empty());
        assert_eq!(runtime.selection.selected_version, None);
        assert_eq!(runtime.selection.active_version, None);
    }

    #[test]
    fn select_managed_runtime_version_updates_persisted_selection() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");
        select_managed_runtime_version(temp_dir.path(), ManagedBinaryId::LlamaCpp, Some("b8248"))
            .expect("select runtime version");
        set_default_managed_runtime_version(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            Some("b8248"),
        )
        .expect("set default runtime version");

        let state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");

        assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
        assert_eq!(runtime.selection.default_version.as_deref(), Some("b8248"));
    }

    #[test]
    fn select_managed_runtime_version_rejects_unknown_version() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");
        let error = select_managed_runtime_version(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            Some("other"),
        )
        .expect_err("unknown version should fail");

        assert!(error.contains("is not installed"));
    }

    #[test]
    fn resolve_runtime_install_dir_uses_selected_version_install_root() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");
        select_managed_runtime_version(temp_dir.path(), ManagedBinaryId::LlamaCpp, Some("b8248"))
            .expect("select runtime version");

        let resolved_install_dir =
            resolve_runtime_install_dir(temp_dir.path(), ManagedBinaryId::LlamaCpp)
                .expect("resolve install dir");

        assert_eq!(resolved_install_dir, install_dir);
    }

    #[test]
    fn resolve_runtime_install_dir_rejects_missing_selected_version() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = state
            .runtimes
            .iter_mut()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");
        runtime.selection.selected_version = Some("other".to_string());
        runtime.versions = vec![ManagedRuntimePersistedVersion {
            version: "b8248".to_string(),
            readiness_state: ManagedRuntimeReadinessState::Ready,
            install_root: Some(install_dir.display().to_string()),
            last_ready_at_ms: None,
            last_error: None,
        }];
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let error = resolve_runtime_install_dir(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect_err("missing selected version should fail");

        assert!(error.contains("selected version 'other' is not installed"));
    }

    #[test]
    fn runtime_install_dir_for_projection_falls_back_when_selected_version_is_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = state
            .runtimes
            .iter_mut()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");
        runtime.selection.selected_version = Some("other".to_string());
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let resolved_install_dir =
            runtime_install_dir_for_projection(temp_dir.path(), ManagedBinaryId::LlamaCpp)
                .expect("resolve projection install dir");

        assert_eq!(
            resolved_install_dir,
            temp_dir.path().join("runtimes/llama-cpp")
        );
    }

    #[test]
    fn binary_capability_tolerates_stale_selected_version_state() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

        persist_install_success(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            &install_dir,
        )
        .expect("persist install success");

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = state
            .runtimes
            .iter_mut()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");
        runtime.selection.selected_version = Some("other".to_string());
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let capability =
            binary_capability(temp_dir.path(), ManagedBinaryId::LlamaCpp).expect("read capability");

        assert_eq!(capability.install_state, ManagedBinaryInstallState::Missing);
    }
}
