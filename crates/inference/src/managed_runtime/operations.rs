use super::archive::extract_archive;
use super::contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeJobState, ManagedRuntimeJobStatus,
    ManagedRuntimeJobArtifactStatus, ManagedRuntimeReadinessState, ManagedRuntimeSnapshot,
    ManagedRuntimeVersionStatus, ResolvedCommand,
};
use super::definitions::definition;
use super::paths::{
    extract_pid_file, managed_install_dir, managed_runtime_dir, managed_version_install_dir,
};
use super::state::{
    ensure_runtime_state_entry, load_managed_runtime_state, runtime_state_entry,
    runtime_state_entry_mut, save_managed_runtime_state, ManagedRuntimeHistoryEventKind,
    ManagedRuntimeInstallHistoryEntry, ManagedRuntimePersistedJobArtifact,
    ManagedRuntimePersistedRuntime, ManagedRuntimePersistedVersion,
};
use futures_util::TryStreamExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static TRANSITION_LOCKS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<tokio::sync::Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CANCELLATION_REQUESTS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PAUSE_REQUESTS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedRuntimeDownloadArtifact {
    temp_path: PathBuf,
    downloaded_bytes: u64,
    total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DownloadResponseMode {
    Fresh { total_size: u64 },
    Resume { total_size: u64 },
}

fn transition_lock(id: ManagedBinaryId) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = TRANSITION_LOCKS.lock();
    locks
        .entry(id)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

fn cancellation_request(id: ManagedBinaryId) -> Arc<AtomicBool> {
    let mut requests = CANCELLATION_REQUESTS.lock();
    requests
        .entry(id)
        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

fn clear_cancellation_request(id: ManagedBinaryId) {
    cancellation_request(id).store(false, Ordering::SeqCst);
}

fn pause_request(id: ManagedBinaryId) -> Arc<AtomicBool> {
    let mut requests = PAUSE_REQUESTS.lock();
    requests
        .entry(id)
        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

fn clear_pause_request(id: ManagedBinaryId) {
    pause_request(id).store(false, Ordering::SeqCst);
}

fn request_cancellation(id: ManagedBinaryId) {
    cancellation_request(id).store(true, Ordering::SeqCst);
}

fn request_pause(id: ManagedBinaryId) {
    pause_request(id).store(true, Ordering::SeqCst);
}

fn take_cancellation_request(id: ManagedBinaryId) -> bool {
    cancellation_request(id).swap(false, Ordering::SeqCst)
}

fn take_pause_request(id: ManagedBinaryId) -> bool {
    pause_request(id).swap(false, Ordering::SeqCst)
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
        app_data_dir,
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
                snapshot_from_capability(
                    app_data_dir,
                    capability,
                    runtime_state_entry(&state, capability.id),
                )
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

pub fn cancel_binary_download(app_data_dir: &Path, id: ManagedBinaryId) -> Result<(), String> {
    let state = load_managed_runtime_state(app_data_dir)?;
    let Some(runtime) = runtime_state_entry(&state, id) else {
        return Err(format!(
            "{} does not have an active managed runtime job",
            id.display_name()
        ));
    };
    let Some(active_job) = runtime.active_job.as_ref() else {
        return Err(format!(
            "{} does not have an active managed runtime job",
            id.display_name()
        ));
    };
    if active_job.state == ManagedRuntimeJobState::Paused && runtime.active_job_artifact.is_some() {
        discard_retained_job_artifact(app_data_dir, id, runtime)?;
        return Ok(());
    }
    if !active_job.cancellable {
        return Err(format!(
            "{} does not have a cancellable managed runtime job",
            id.display_name()
        ));
    }

    request_cancellation(id);
    Ok(())
}

pub fn pause_binary_download(app_data_dir: &Path, id: ManagedBinaryId) -> Result<(), String> {
    let state = load_managed_runtime_state(app_data_dir)?;
    let Some(runtime) = runtime_state_entry(&state, id) else {
        return Err(format!(
            "{} does not have an active managed runtime job",
            id.display_name()
        ));
    };
    let Some(active_job) = runtime.active_job.as_ref() else {
        return Err(format!(
            "{} does not have an active managed runtime job",
            id.display_name()
        ));
    };
    if !active_job.cancellable {
        return Err(format!(
            "{} does not have a pausable managed runtime job",
            id.display_name()
        ));
    }

    request_pause(id);
    Ok(())
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
    clear_cancellation_request(id);
    clear_pause_request(id);
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

    let retained_artifact = existing_download_artifact(
        app_data_dir,
        id,
        &runtime_version,
        &release_asset.archive_name,
    )?;
    let temp_path = retained_artifact
        .as_ref()
        .map(|artifact| artifact.temp_path.clone())
        .unwrap_or_else(|| {
            runtime_root.join(format!(
                ".{}-{}",
                uuid::Uuid::new_v4(),
                release_asset.archive_name
            ))
        });
    let mut downloaded = retained_artifact
        .as_ref()
        .map(|artifact| artifact.downloaded_bytes)
        .unwrap_or(0);
    let mut total_size = retained_artifact
        .as_ref()
        .map(|artifact| artifact.total_bytes)
        .unwrap_or(0);
    let initial_artifact = retained_artifact.as_ref().map(|artifact| ManagedRuntimePersistedJobArtifact {
        version: runtime_version.clone(),
        archive_name: release_asset.archive_name.clone(),
        archive_path: artifact.temp_path.display().to_string(),
        downloaded_bytes: artifact.downloaded_bytes,
        total_bytes: artifact.total_bytes,
    });

    persist_active_job_with_artifact(
        app_data_dir,
        id,
        ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Queued,
            status: format!("Queued {} install", definition.display_name()),
            current: downloaded,
            total: total_size,
            resumable: downloaded > 0,
            cancellable: true,
            error: None,
        },
        initial_artifact.clone(),
    )?;

    on_progress(DownloadProgress {
        status: if downloaded > 0 {
            format!("Resuming {} download...", definition.display_name())
        } else {
            format!("Downloading {} binaries...", definition.display_name())
        },
        current: downloaded,
        total: total_size,
        done: false,
        error: None,
    });

    persist_active_job_with_artifact(
        app_data_dir,
        id,
        ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Downloading,
            status: if downloaded > 0 {
                format!("Resuming {}", definition.display_name())
            } else {
                format!("Downloading {}", definition.display_name())
            },
            current: downloaded,
            total: total_size,
            resumable: downloaded > 0,
            cancellable: true,
            error: None,
        },
        initial_artifact,
    )?;

    log::info!(
        "Downloading {} from: {}",
        definition.display_name(),
        download_url
    );

    if total_size == 0 || downloaded < total_size {
        let client = reqwest::Client::new();
        let requested_resume_from = downloaded;
        let mut request = client.get(&download_url);
        if requested_resume_from > 0 {
            request = request.header(RANGE, format!("bytes={requested_resume_from}-"));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to start download: {}", e))?;
        let response_mode = download_response_mode(
            requested_resume_from,
            response.status(),
            response.content_length(),
            response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok()),
        )?;

        let mut file = match response_mode {
            DownloadResponseMode::Fresh { total_size: response_total } => {
                total_size = response_total;
                downloaded = 0;
                fs::File::create(&temp_path)
                    .map_err(|e| format!("Failed to create temp file: {}", e))?
            }
            DownloadResponseMode::Resume { total_size: response_total } => {
                total_size = response_total;
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&temp_path)
                    .map_err(|e| format!("Failed to open resumable temp file: {}", e))?
            }
        };

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

            let job_artifact = ManagedRuntimePersistedJobArtifact {
                version: runtime_version.clone(),
                archive_name: release_asset.archive_name.clone(),
                archive_path: temp_path.display().to_string(),
                downloaded_bytes: downloaded,
                total_bytes: total_size,
            };

            persist_active_job_with_artifact(
                app_data_dir,
                id,
                ManagedRuntimeJobStatus {
                    state: ManagedRuntimeJobState::Downloading,
                    status: if requested_resume_from > 0 {
                        "Resuming".to_string()
                    } else {
                        "Downloading".to_string()
                    },
                    current: downloaded,
                    total: total_size,
                    resumable: downloaded > 0,
                    cancellable: true,
                    error: None,
                },
                Some(job_artifact.clone()),
            )?;

            if finish_requested_cancellation(
                app_data_dir,
                id,
                &runtime_version,
                downloaded,
                total_size,
                None,
            )? {
                drop(file);
                let _ = fs::remove_file(&temp_path);
                on_progress(DownloadProgress {
                    status: "Cancelled".to_string(),
                    current: downloaded,
                    total: total_size,
                    done: true,
                    error: None,
                });
                return Ok(());
            }

            if finish_requested_pause(
                app_data_dir,
                id,
                &runtime_version,
                downloaded,
                total_size,
                job_artifact,
            )? {
                drop(file);
                on_progress(DownloadProgress {
                    status: "Paused".to_string(),
                    current: downloaded,
                    total: total_size,
                    done: true,
                    error: None,
                });
                return Ok(());
            }

            on_progress(DownloadProgress {
                status: if requested_resume_from > 0 {
                    "Resuming...".to_string()
                } else {
                    "Downloading...".to_string()
                },
                current: downloaded,
                total: total_size,
                done: false,
                error: None,
            });
        }
    }

    let download_artifact = ManagedRuntimePersistedJobArtifact {
        version: runtime_version.clone(),
        archive_name: release_asset.archive_name.clone(),
        archive_path: temp_path.display().to_string(),
        downloaded_bytes: downloaded,
        total_bytes: total_size,
    };

    if finish_requested_cancellation(
        app_data_dir,
        id,
        &runtime_version,
        downloaded,
        total_size,
        None,
    )? {
        let _ = fs::remove_file(&temp_path);
        on_progress(DownloadProgress {
            status: "Cancelled".to_string(),
            current: downloaded,
            total: total_size,
            done: true,
            error: None,
        });
        return Ok(());
    }

    if finish_requested_pause(
        app_data_dir,
        id,
        &runtime_version,
        downloaded,
        total_size,
        download_artifact.clone(),
    )? {
        on_progress(DownloadProgress {
            status: "Paused".to_string(),
            current: downloaded,
            total: total_size,
            done: true,
            error: None,
        });
        return Ok(());
    }

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

    if finish_requested_cancellation(
        app_data_dir,
        id,
        &runtime_version,
        total_size,
        total_size,
        None,
    )? {
        let _ = fs::remove_dir_all(&staging_dir);
        on_progress(DownloadProgress {
            status: "Cancelled".to_string(),
            current: total_size,
            total: total_size,
            done: true,
            error: None,
        });
        return Ok(());
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

    if finish_requested_cancellation(
        app_data_dir,
        id,
        &runtime_version,
        total_size,
        total_size,
        None,
    )? {
        let _ = fs::remove_dir_all(&staging_dir);
        on_progress(DownloadProgress {
            status: "Cancelled".to_string(),
            current: total_size,
            total: total_size,
            done: true,
            error: None,
        });
        return Ok(());
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
    clear_cancellation_request(id);
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
    app_data_dir: &Path,
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
) -> ManagedRuntimeSnapshot {
    let definition = definition(capability.id);
    let selection = persisted_runtime
        .map(|runtime| runtime.selection.clone())
        .unwrap_or_default();
    let active_job = persisted_runtime.and_then(|runtime| runtime.active_job.clone());
    let job_artifact =
        persisted_runtime.and_then(projected_job_artifact_status);
    let versions = persisted_runtime
        .filter(|runtime| !runtime.versions.is_empty())
        .map(|runtime| {
            runtime
                .versions
                .iter()
                .map(|version| ManagedRuntimeVersionStatus {
                    version: Some(version.version.clone()),
                    display_label: version.version.clone(),
                    runtime_key: version
                        .runtime_key
                        .clone()
                        .unwrap_or_else(|| capability.id.key().to_string()),
                    platform_key: version
                        .platform_key
                        .clone()
                        .unwrap_or_else(|| definition.platform_key().to_string()),
                    install_root: version.install_root.clone(),
                    executable_name: definition.executable_name().to_string(),
                    executable_ready: version
                        .install_root
                        .as_deref()
                        .map(|install_root| {
                            definition
                                .validate_installation(Path::new(install_root))
                                .is_empty()
                        })
                        .unwrap_or(false),
                    install_state: capability.install_state,
                    readiness_state: version.readiness_state,
                    selected: selection.selected_version.as_deref()
                        == Some(version.version.as_str()),
                    active: selection.active_version.as_deref() == Some(version.version.as_str()),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![version_status_for_capability(app_data_dir, capability)]);
    let readiness_state = projected_snapshot_readiness_state(capability, persisted_runtime);
    let available = projected_snapshot_available(capability, readiness_state);
    let unavailable_reason = projected_snapshot_unavailable_reason(capability, persisted_runtime);

    ManagedRuntimeSnapshot {
        id: capability.id,
        display_name: capability.display_name.clone(),
        install_state: capability.install_state,
        readiness_state,
        available,
        can_install: capability.can_install,
        can_remove: capability.can_remove,
        missing_files: capability.missing_files.clone(),
        unavailable_reason,
        versions,
        selection,
        active_job,
        job_artifact,
    }
}

fn projected_job_artifact_status(
    runtime: &ManagedRuntimePersistedRuntime,
) -> Option<ManagedRuntimeJobArtifactStatus> {
    let artifact = runtime.active_job_artifact.as_ref()?;
    Some(ManagedRuntimeJobArtifactStatus {
        version: artifact.version.clone(),
        archive_name: artifact.archive_name.clone(),
        downloaded_bytes: artifact.downloaded_bytes,
        total_bytes: artifact.total_bytes,
        retained: Path::new(&artifact.archive_path).exists(),
    })
}

fn existing_download_artifact(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    runtime_version: &str,
    archive_name: &str,
) -> Result<Option<ManagedRuntimeDownloadArtifact>, String> {
    let state = load_managed_runtime_state(app_data_dir)?;
    let Some(runtime) = runtime_state_entry(&state, id) else {
        return Ok(None);
    };
    let artifact = runtime.active_job_artifact.as_ref();
    let Some(artifact) = artifact else {
        return Ok(None);
    };
    if artifact.version != runtime_version || artifact.archive_name != archive_name {
        return Ok(None);
    }

    let temp_path = PathBuf::from(&artifact.archive_path);
    let metadata = match fs::metadata(&temp_path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    let downloaded_bytes = metadata.len();
    if downloaded_bytes == 0 {
        return Ok(None);
    }

    Ok(Some(ManagedRuntimeDownloadArtifact {
        temp_path,
        downloaded_bytes,
        total_bytes: artifact.total_bytes.max(downloaded_bytes),
    }))
}

fn parse_content_range_total(content_range: Option<&str>) -> Option<u64> {
    let content_range = content_range?;
    let (_, total) = content_range.split_once('/')?;
    total.parse::<u64>().ok()
}

fn download_response_mode(
    requested_resume_from: u64,
    status: StatusCode,
    content_length: Option<u64>,
    content_range: Option<&str>,
) -> Result<DownloadResponseMode, String> {
    if requested_resume_from == 0 {
        if !status.is_success() {
            return Err(format!("Download failed with status: {status}"));
        }
        return Ok(DownloadResponseMode::Fresh {
            total_size: content_length.unwrap_or(0),
        });
    }

    match status {
        StatusCode::PARTIAL_CONTENT => Ok(DownloadResponseMode::Resume {
            total_size: parse_content_range_total(content_range)
                .or_else(|| content_length.map(|length| requested_resume_from + length))
                .unwrap_or(requested_resume_from),
        }),
        StatusCode::OK => Ok(DownloadResponseMode::Fresh {
            total_size: content_length.unwrap_or(0),
        }),
        _ => Err(format!("Download failed with status: {status}")),
    }
}

fn projected_snapshot_readiness_state(
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
) -> ManagedRuntimeReadinessState {
    let Some(runtime) = persisted_runtime else {
        return readiness_state_for_capability(capability);
    };

    if let Some(selected_version) = runtime.selection.selected_version.as_deref() {
        if let Some(version) = runtime
            .versions
            .iter()
            .find(|version| version.version == selected_version)
        {
            return version.readiness_state;
        }

        return match capability.install_state {
            ManagedBinaryInstallState::Unsupported => ManagedRuntimeReadinessState::Unsupported,
            _ => ManagedRuntimeReadinessState::Missing,
        };
    }

    runtime
        .active_job
        .as_ref()
        .map(|job| job_readiness_state(job.state))
        .unwrap_or_else(|| readiness_state_for_capability(capability))
}

fn projected_snapshot_available(
    capability: &ManagedBinaryCapability,
    readiness_state: ManagedRuntimeReadinessState,
) -> bool {
    capability.available && readiness_state == ManagedRuntimeReadinessState::Ready
}

fn projected_snapshot_unavailable_reason(
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
) -> Option<String> {
    if let Some(reason) = capability
        .unavailable_reason
        .as_ref()
        .filter(|reason| !reason.trim().is_empty())
    {
        return Some(reason.clone());
    }

    let runtime = persisted_runtime?;
    let selected_version = runtime.selection.selected_version.as_deref()?;
    runtime
        .versions
        .iter()
        .find(|version| version.version == selected_version)
        .and_then(|version| version.last_error.clone())
        .filter(|reason| !reason.trim().is_empty())
}

fn job_readiness_state(state: ManagedRuntimeJobState) -> ManagedRuntimeReadinessState {
    match state {
        ManagedRuntimeJobState::Queued | ManagedRuntimeJobState::Downloading => {
            ManagedRuntimeReadinessState::Downloading
        }
        ManagedRuntimeJobState::Extracting => ManagedRuntimeReadinessState::Extracting,
        ManagedRuntimeJobState::Validating => ManagedRuntimeReadinessState::Validating,
        ManagedRuntimeJobState::Ready => ManagedRuntimeReadinessState::Ready,
        ManagedRuntimeJobState::Failed
        | ManagedRuntimeJobState::Paused
        | ManagedRuntimeJobState::Cancelled => ManagedRuntimeReadinessState::Failed,
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
    app_data_dir: &Path,
    capability: &ManagedBinaryCapability,
) -> ManagedRuntimeVersionStatus {
    let definition = definition(capability.id);
    let install_root = managed_install_dir(app_data_dir, capability.id);
    ManagedRuntimeVersionStatus {
        version: None,
        display_label: match capability.install_state {
            ManagedBinaryInstallState::SystemProvided => "System provided".to_string(),
            ManagedBinaryInstallState::Installed => "Managed install".to_string(),
            ManagedBinaryInstallState::Missing => "No managed install".to_string(),
            ManagedBinaryInstallState::Unsupported => "Unsupported platform".to_string(),
        },
        runtime_key: capability.id.key().to_string(),
        platform_key: definition.platform_key().to_string(),
        install_root: install_root
            .exists()
            .then(|| install_root.display().to_string()),
        executable_name: definition.executable_name().to_string(),
        executable_ready: capability.available,
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
    persist_active_job_with_artifact(app_data_dir, id, job, None)
}

fn persist_active_job_with_artifact(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    job: ManagedRuntimeJobStatus,
    job_artifact: Option<ManagedRuntimePersistedJobArtifact>,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = Some(job);
    runtime.active_job_artifact = job_artifact;
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
    runtime.active_job_artifact = None;
    upsert_persisted_version(
        runtime,
        ManagedRuntimePersistedVersion {
            version: version.to_string(),
            runtime_key: Some(id.key().to_string()),
            platform_key: Some(definition(id).platform_key().to_string()),
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

fn persist_paused_job(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    current: u64,
    total: u64,
    job_artifact: ManagedRuntimePersistedJobArtifact,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Paused,
        status: "Paused".to_string(),
        current,
        total,
        resumable: true,
        cancellable: false,
        error: None,
    });
    runtime.active_job_artifact = Some(job_artifact);
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            event: ManagedRuntimeHistoryEventKind::Paused,
            version: Some(version.to_string()),
            at_ms: current_unix_timestamp_ms(),
            detail: Some("Managed runtime install paused with retained download artifact".to_string()),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

fn persist_cancelled_job(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    current: u64,
    total: u64,
    job_artifact: Option<ManagedRuntimePersistedJobArtifact>,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Cancelled,
        status: "Cancelled".to_string(),
        current,
        total,
        resumable: job_artifact.is_some(),
        cancellable: false,
        error: None,
    });
    runtime.active_job_artifact = job_artifact;
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            event: ManagedRuntimeHistoryEventKind::Cancelled,
            version: Some(version.to_string()),
            at_ms: current_unix_timestamp_ms(),
            detail: Some(if runtime.active_job_artifact.is_some() {
                "Managed runtime install cancelled with retained download artifact".to_string()
            } else {
                "Managed runtime install cancelled".to_string()
            }),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

fn discard_retained_job_artifact(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    runtime: &ManagedRuntimePersistedRuntime,
) -> Result<(), String> {
    let Some(artifact) = runtime.active_job_artifact.as_ref() else {
        return Err(format!(
            "{} does not have a retained managed runtime artifact",
            id.display_name()
        ));
    };
    let artifact_path = PathBuf::from(&artifact.archive_path);
    if artifact_path.exists() {
        let _ = fs::remove_file(&artifact_path);
    }
    let version = artifact.version.clone();
    let current = artifact.downloaded_bytes;
    let total = artifact.total_bytes;
    persist_cancelled_job(app_data_dir, id, &version, current, total, None)
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
    runtime.active_job_artifact = None;
    upsert_persisted_version(
        runtime,
        ManagedRuntimePersistedVersion {
            version: version.to_string(),
            runtime_key: Some(id.key().to_string()),
            platform_key: Some(definition(id).platform_key().to_string()),
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
    runtime.active_job_artifact = None;
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

fn finish_requested_cancellation(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    current: u64,
    total: u64,
    job_artifact: Option<ManagedRuntimePersistedJobArtifact>,
) -> Result<bool, String> {
    if !take_cancellation_request(id) {
        return Ok(false);
    }

    persist_cancelled_job(app_data_dir, id, version, current, total, job_artifact)?;
    Ok(true)
}

fn finish_requested_pause(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    current: u64,
    total: u64,
    job_artifact: ManagedRuntimePersistedJobArtifact,
) -> Result<bool, String> {
    if !take_pause_request(id) {
        return Ok(false);
    }

    persist_paused_job(app_data_dir, id, version, current, total, job_artifact)?;
    Ok(true)
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
        let Some(persisted_version) = runtime
            .versions
            .iter()
            .find(|entry| entry.version == version)
        else {
            return Err(format!(
                "{} version '{}' is not installed",
                id.display_name(),
                version
            ));
        };

        if persisted_version.readiness_state != ManagedRuntimeReadinessState::Ready {
            return Err(format!(
                "{} version '{}' is not ready for selection",
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
        binary_capability, cancel_binary_download, definition, ensure_runtime_state_entry,
        download_response_mode, existing_download_artifact, finish_requested_cancellation,
        finish_requested_pause, pause_binary_download, persist_install_success,
        persist_remove_success, readiness_state_for_capability, resolve_runtime_install_dir,
        runtime_install_dir_for_projection, select_managed_runtime_version,
        set_default_managed_runtime_version, snapshot_from_capability, DownloadResponseMode,
        ManagedBinaryCapability, ManagedBinaryId, ManagedBinaryInstallState,
        ManagedRuntimeJobState, ManagedRuntimeJobStatus, ManagedRuntimeReadinessState,
    };
    use crate::managed_runtime::{
        load_managed_runtime_state, save_managed_runtime_state, ManagedRuntimeHistoryEventKind,
        ManagedRuntimePersistedJobArtifact, ManagedRuntimePersistedVersion,
    };
    use reqwest::StatusCode;
    use std::path::Path;

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

    fn install_fake_runtime_files(dir: &Path, id: ManagedBinaryId) {
        std::fs::create_dir_all(dir).expect("create runtime dir");
        for file_name in definition(id).validate_installation(dir) {
            std::fs::write(dir.join(&file_name), [])
                .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
        }
        assert!(
            definition(id).validate_installation(dir).is_empty(),
            "fake runtime files should satisfy install validation"
        );
    }

    #[test]
    fn download_response_mode_supports_partial_content_resume() {
        let mode = download_response_mode(
            64,
            StatusCode::PARTIAL_CONTENT,
            Some(64),
            Some("bytes 64-127/128"),
        )
        .expect("resume mode");

        assert_eq!(mode, DownloadResponseMode::Resume { total_size: 128 });
    }

    #[test]
    fn download_response_mode_restarts_when_server_ignores_range() {
        let mode = download_response_mode(64, StatusCode::OK, Some(128), None)
            .expect("fresh mode");

        assert_eq!(mode, DownloadResponseMode::Fresh { total_size: 128 });
    }

    #[test]
    fn existing_download_artifact_uses_current_file_length() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
        std::fs::write(&artifact_path, vec![1_u8; 16]).expect("write retained artifact");

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job_artifact = Some(ManagedRuntimePersistedJobArtifact {
            version: "b8248".to_string(),
            archive_name: "llama-b8248.tar.gz".to_string(),
            archive_path: artifact_path.display().to_string(),
            downloaded_bytes: 8,
            total_bytes: 32,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let artifact = existing_download_artifact(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            "llama-b8248.tar.gz",
        )
        .expect("load retained artifact")
        .expect("retained artifact");

        assert_eq!(artifact.temp_path, artifact_path);
        assert_eq!(artifact.downloaded_bytes, 16);
        assert_eq!(artifact.total_bytes, 32);
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
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let snapshot = snapshot_from_capability(
            temp_dir.path(),
            &capability(ManagedBinaryInstallState::Installed),
            None,
        );

        assert_eq!(snapshot.versions.len(), 1);
        assert_eq!(snapshot.selection.selected_version, None);
        assert_eq!(
            snapshot.readiness_state,
            ManagedRuntimeReadinessState::Ready
        );
        assert_eq!(
            snapshot.versions[0].runtime_key,
            ManagedBinaryId::LlamaCpp.key()
        );
        assert!(!snapshot.versions[0].platform_key.is_empty());
        assert!(!snapshot.versions[0].executable_name.is_empty());
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
        assert_eq!(
            runtime.versions[0].runtime_key.as_deref(),
            Some(ManagedBinaryId::LlamaCpp.key())
        );
        assert!(runtime.versions[0].platform_key.is_some());
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
    fn select_managed_runtime_version_rejects_non_ready_version() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.versions.push(ManagedRuntimePersistedVersion {
            version: "b8248".to_string(),
            runtime_key: Some(ManagedBinaryId::LlamaCpp.key().to_string()),
            platform_key: Some("linux-x86_64".to_string()),
            readiness_state: ManagedRuntimeReadinessState::Failed,
            install_root: None,
            last_ready_at_ms: None,
            last_error: Some("validation failed".to_string()),
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let error = select_managed_runtime_version(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            Some("b8248"),
        )
        .expect_err("non-ready version should fail");

        assert!(error.contains("is not ready for selection"));
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
            runtime_key: Some(ManagedBinaryId::LlamaCpp.key().to_string()),
            platform_key: Some("linux-x86_64".to_string()),
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

    #[test]
    fn managed_runtime_snapshot_uses_selected_version_failed_readiness() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");
        install_fake_runtime_files(&install_dir, ManagedBinaryId::LlamaCpp);

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.selection.selected_version = Some("b8248".to_string());
        runtime.selection.default_version = Some("b8248".to_string());
        runtime.versions.push(ManagedRuntimePersistedVersion {
            version: "b8248".to_string(),
            runtime_key: Some(ManagedBinaryId::LlamaCpp.key().to_string()),
            platform_key: Some("linux-x86_64".to_string()),
            readiness_state: ManagedRuntimeReadinessState::Failed,
            install_root: Some(install_dir.display().to_string()),
            last_ready_at_ms: None,
            last_error: Some("validation failed".to_string()),
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let snapshot = crate::managed_runtime::managed_runtime_snapshot(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
        )
        .expect("managed runtime snapshot");

        assert_eq!(
            snapshot.readiness_state,
            ManagedRuntimeReadinessState::Failed
        );
        assert!(!snapshot.available);
        assert_eq!(
            snapshot.unavailable_reason.as_deref(),
            Some("validation failed")
        );
    }

    #[test]
    fn managed_runtime_snapshot_uses_reconciled_interrupted_job_readiness() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp");
        install_fake_runtime_files(&install_dir, ManagedBinaryId::LlamaCpp);

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Downloading,
            status: "Downloading b8248".to_string(),
            current: 5,
            total: 10,
            resumable: true,
            cancellable: true,
            error: None,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let snapshot = crate::managed_runtime::managed_runtime_snapshot(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
        )
        .expect("managed runtime snapshot");

        assert_eq!(
            snapshot.readiness_state,
            ManagedRuntimeReadinessState::Failed
        );
        assert!(!snapshot.available);
        let active_job = snapshot.active_job.expect("reconciled active job");
        assert_eq!(active_job.state, ManagedRuntimeJobState::Failed);
        assert_eq!(active_job.status, "Interrupted before completion");
    }

    #[test]
    fn cancel_binary_download_rejects_non_cancellable_jobs() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Extracting,
            status: "Extracting".to_string(),
            current: 10,
            total: 10,
            resumable: false,
            cancellable: false,
            error: None,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let error = cancel_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect_err("non-cancellable job should fail");

        assert!(error.contains("does not have a cancellable managed runtime job"));
    }

    #[test]
    fn pause_binary_download_rejects_non_cancellable_jobs() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Extracting,
            status: "Extracting".to_string(),
            current: 10,
            total: 10,
            resumable: false,
            cancellable: false,
            error: None,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let error = pause_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect_err("non-cancellable job should fail");

        assert!(error.contains("does not have a pausable managed runtime job"));
    }

    #[test]
    fn finish_requested_cancellation_persists_cancelled_job_and_history() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Downloading,
            status: "Downloading".to_string(),
            current: 32,
            total: 64,
            resumable: false,
            cancellable: true,
            error: None,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        cancel_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect("request cancellation");
        let cancelled = finish_requested_cancellation(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            32,
            64,
            None,
        )
        .expect("finish cancellation");

        assert!(cancelled);
        let state = load_managed_runtime_state(temp_dir.path()).expect("reload runtime state");
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");
        let active_job = runtime.active_job.as_ref().expect("cancelled job");
        assert_eq!(active_job.state, ManagedRuntimeJobState::Cancelled);
        assert_eq!(active_job.status, "Cancelled");
        assert_eq!(active_job.current, 32);
        assert_eq!(active_job.total, 64);
        assert!(runtime.active_job_artifact.is_none());
        assert_eq!(
            runtime
                .install_history
                .last()
                .map(|entry| entry.event.clone()),
            Some(ManagedRuntimeHistoryEventKind::Cancelled)
        );
    }

    #[test]
    fn finish_requested_pause_persists_paused_job_and_history() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
        std::fs::write(&artifact_path, vec![1_u8; 32]).expect("write retained artifact");

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Downloading,
            status: "Downloading".to_string(),
            current: 32,
            total: 64,
            resumable: true,
            cancellable: true,
            error: None,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        pause_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect("request pause");
        let paused = finish_requested_pause(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            "b8248",
            32,
            64,
            ManagedRuntimePersistedJobArtifact {
                version: "b8248".to_string(),
                archive_name: "llama-b8248.tar.gz".to_string(),
                archive_path: artifact_path.display().to_string(),
                downloaded_bytes: 32,
                total_bytes: 64,
            },
        )
        .expect("finish pause");

        assert!(paused);
        let state = load_managed_runtime_state(temp_dir.path()).expect("reload runtime state");
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");
        let active_job = runtime.active_job.as_ref().expect("paused job");
        assert_eq!(active_job.state, ManagedRuntimeJobState::Paused);
        assert_eq!(active_job.status, "Paused");
        assert_eq!(active_job.current, 32);
        assert_eq!(active_job.total, 64);
        assert!(active_job.resumable);
        assert!(runtime.active_job_artifact.is_some());
        assert_eq!(
            runtime
                .install_history
                .last()
                .map(|entry| entry.event.clone()),
            Some(ManagedRuntimeHistoryEventKind::Paused)
        );
    }

    #[test]
    fn cancel_binary_download_discards_paused_artifact() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
        std::fs::write(&artifact_path, vec![1_u8; 32]).expect("write retained artifact");

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Paused,
            status: "Paused".to_string(),
            current: 32,
            total: 64,
            resumable: true,
            cancellable: false,
            error: None,
        });
        runtime.active_job_artifact = Some(ManagedRuntimePersistedJobArtifact {
            version: "b8248".to_string(),
            archive_name: "llama-b8248.tar.gz".to_string(),
            archive_path: artifact_path.display().to_string(),
            downloaded_bytes: 32,
            total_bytes: 64,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        cancel_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect("discard paused artifact");

        let state = load_managed_runtime_state(temp_dir.path()).expect("reload runtime state");
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama runtime state");
        let active_job = runtime.active_job.as_ref().expect("cancelled job");
        assert_eq!(active_job.state, ManagedRuntimeJobState::Cancelled);
        assert!(runtime.active_job_artifact.is_none());
        assert!(!artifact_path.exists());
        assert_eq!(
            runtime
                .install_history
                .last()
                .map(|entry| entry.event.clone()),
            Some(ManagedRuntimeHistoryEventKind::Cancelled)
        );
    }

    #[test]
    fn managed_runtime_snapshot_projects_retained_job_artifact() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
        std::fs::write(&artifact_path, []).expect("write retained artifact");

        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
        runtime.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Cancelled,
            status: "Cancelled".to_string(),
            current: 64,
            total: 128,
            resumable: true,
            cancellable: false,
            error: None,
        });
        runtime.active_job_artifact = Some(ManagedRuntimePersistedJobArtifact {
            version: "b8248".to_string(),
            archive_name: "llama-b8248.tar.gz".to_string(),
            archive_path: artifact_path.display().to_string(),
            downloaded_bytes: 64,
            total_bytes: 128,
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let snapshot =
            crate::managed_runtime::managed_runtime_snapshot(temp_dir.path(), ManagedBinaryId::LlamaCpp)
                .expect("managed runtime snapshot");

        let artifact = snapshot.job_artifact.expect("job artifact");
        assert_eq!(artifact.version, "b8248");
        assert_eq!(artifact.archive_name, "llama-b8248.tar.gz");
        assert_eq!(artifact.downloaded_bytes, 64);
        assert_eq!(artifact.total_bytes, 128);
        assert!(artifact.retained);
    }
}
