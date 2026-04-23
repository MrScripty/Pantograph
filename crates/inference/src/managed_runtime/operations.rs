use super::archive::extract_archive;
use super::catalog::fetch_managed_runtime_catalog;
use super::contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeCatalogVersion, ManagedRuntimeJobArtifactStatus,
    ManagedRuntimeJobState, ManagedRuntimeJobStatus, ManagedRuntimeReadinessState,
    ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus, ResolvedCommand,
};
use super::definitions::definition;
use super::paths::{
    extract_pid_file, managed_install_dir, managed_runtime_dir, managed_version_install_dir,
};
use super::state::{
    ManagedRuntimeHistoryEventKind, ManagedRuntimeInstallHistoryEntry,
    ManagedRuntimePersistedJobArtifact, ManagedRuntimePersistedRuntime,
    ManagedRuntimePersistedVersion, ensure_runtime_state_entry, load_managed_runtime_state,
    runtime_state_entry, runtime_state_entry_mut, save_managed_runtime_state,
};
use futures_util::TryStreamExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::StatusCode;
use reqwest::header::{CONTENT_RANGE, RANGE};
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

static TRANSITION_LOCKS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<tokio::sync::Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CANCELLATION_REQUESTS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PAUSE_REQUESTS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
const CATALOG_REFRESH_TTL_MS: u64 = 60 * 60 * 1000;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedRuntimeDownloadSource {
    version: String,
    runtime_key: String,
    platform_key: String,
    archive_name: String,
    download_url: String,
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

    let release_asset = definition.release_asset(definition.default_release_version());
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

pub async fn refresh_managed_runtime_catalog(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<ManagedRuntimeSnapshot, String> {
    let catalog = fetch_managed_runtime_catalog(id).await?;
    persist_catalog_versions(app_data_dir, id, catalog)?;
    managed_runtime_snapshot(app_data_dir, id)
}

pub async fn refresh_managed_runtime_catalogs(
    app_data_dir: &Path,
) -> Result<Vec<ManagedRuntimeSnapshot>, String> {
    for id in ManagedBinaryId::all().iter().copied() {
        if let Ok(catalog) = fetch_managed_runtime_catalog(id).await {
            let _ = persist_catalog_versions(app_data_dir, id, catalog);
        }
    }

    list_managed_runtime_snapshots(app_data_dir)
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
    requested_version: Option<&str>,
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
    let download_source = resolve_download_source(app_data_dir, id, requested_version).await?;
    let runtime_version = download_source.version.clone();
    if definition.system_command().is_some() {
        return Err(format!(
            "{} is already available from the system PATH",
            definition.display_name()
        ));
    }
    let runtime_root = managed_runtime_dir(app_data_dir);
    let install_dir = managed_version_install_dir(app_data_dir, id, &runtime_version);
    let release_asset = definition.release_asset(&runtime_version)?;
    let download_url = download_source.download_url.clone();

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
                download_source.archive_name
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
    let initial_artifact =
        retained_artifact
            .as_ref()
            .map(|artifact| ManagedRuntimePersistedJobArtifact {
                version: runtime_version.clone(),
                archive_name: download_source.archive_name.clone(),
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
            DownloadResponseMode::Fresh {
                total_size: response_total,
            } => {
                total_size = response_total;
                downloaded = 0;
                fs::File::create(&temp_path)
                    .map_err(|e| format!("Failed to create temp file: {}", e))?
            }
            DownloadResponseMode::Resume {
                total_size: response_total,
            } => {
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
                archive_name: download_source.archive_name.clone(),
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
        archive_name: download_source.archive_name.clone(),
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

    persist_install_success(
        app_data_dir,
        id,
        &runtime_version,
        &install_dir,
        &download_source.runtime_key,
        &download_source.platform_key,
    )?;
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
    let job_artifact = persisted_runtime.and_then(projected_job_artifact_status);
    let versions = projected_version_statuses(
        app_data_dir,
        capability,
        persisted_runtime,
        &selection,
        definition,
    );
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

fn persist_catalog_versions(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    catalog_versions: Vec<ManagedRuntimeCatalogVersion>,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.catalog_versions = catalog_versions;
    runtime.catalog_refreshed_at_ms = Some(current_unix_timestamp_ms());
    save_managed_runtime_state(app_data_dir, &state)
}

async fn resolve_download_source(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    requested_version: Option<&str>,
) -> Result<ManagedRuntimeDownloadSource, String> {
    let definition = definition(id);
    let state = load_managed_runtime_state(app_data_dir)?;
    let runtime = runtime_state_entry(&state, id);

    let preferred_version = requested_version
        .map(str::to_string)
        .or_else(|| {
            runtime.and_then(|runtime| {
                runtime
                    .active_job
                    .as_ref()
                    .filter(|job| job.state == ManagedRuntimeJobState::Paused)
                    .and(runtime.active_job_artifact.as_ref())
                    .map(|artifact| artifact.version.clone())
            })
        })
        .or_else(|| runtime.and_then(|runtime| runtime.selection.selected_version.clone()));

    if let Some(version) = preferred_version.as_deref() {
        if let Some(download_source) = runtime
            .and_then(|runtime| {
                runtime
                    .catalog_versions
                    .iter()
                    .find(|entry| entry.version == version)
            })
            .map(download_source_from_catalog)
        {
            return Ok(download_source);
        }
    }

    let catalog_is_fresh = runtime
        .and_then(|runtime| runtime.catalog_refreshed_at_ms)
        .map(|timestamp| {
            current_unix_timestamp_ms().saturating_sub(timestamp) <= CATALOG_REFRESH_TTL_MS
        })
        .unwrap_or(false);

    if catalog_is_fresh {
        if let Some(download_source) = runtime
            .and_then(|runtime| runtime.catalog_versions.first())
            .map(download_source_from_catalog)
        {
            return Ok(download_source);
        }
    }

    let catalog = fetch_managed_runtime_catalog(id).await?;
    persist_catalog_versions(app_data_dir, id, catalog.clone())?;
    if let Some(version) = preferred_version.as_deref() {
        if let Some(download_source) = catalog
            .iter()
            .find(|entry| entry.version == version)
            .map(download_source_from_catalog)
        {
            return Ok(download_source);
        }

        return Err(format!(
            "{} version {} is not available for the current platform",
            definition.display_name(),
            version
        ));
    }

    if let Some(download_source) = catalog.first().map(download_source_from_catalog) {
        return Ok(download_source);
    }

    let version = definition.default_release_version().to_string();
    let release_asset = definition.release_asset(&version)?;
    Ok(ManagedRuntimeDownloadSource {
        version: version.clone(),
        runtime_key: id.key().to_string(),
        platform_key: definition.platform_key().to_string(),
        archive_name: release_asset.archive_name.clone(),
        download_url: definition.download_url(&version, &release_asset),
    })
}

fn download_source_from_catalog(
    catalog_version: &ManagedRuntimeCatalogVersion,
) -> ManagedRuntimeDownloadSource {
    ManagedRuntimeDownloadSource {
        version: catalog_version.version.clone(),
        runtime_key: catalog_version.runtime_key.clone(),
        platform_key: catalog_version.platform_key.clone(),
        archive_name: catalog_version.archive_name.clone(),
        download_url: catalog_version.download_url.clone(),
    }
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

fn projected_version_statuses(
    app_data_dir: &Path,
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
    selection: &super::contracts::ManagedRuntimeSelectionState,
    definition: &'static dyn super::definitions::ManagedBinaryDefinition,
) -> Vec<ManagedRuntimeVersionStatus> {
    let Some(runtime) = persisted_runtime else {
        return vec![version_status_for_capability(app_data_dir, capability)];
    };

    let mut projected_versions = Vec::new();

    for catalog_version in &runtime.catalog_versions {
        projected_versions.push(projected_catalog_version_status(
            capability,
            runtime,
            selection,
            definition,
            catalog_version,
        ));
    }

    for installed_version in &runtime.versions {
        if runtime
            .catalog_versions
            .iter()
            .any(|catalog_version| catalog_version.version == installed_version.version)
        {
            continue;
        }

        projected_versions.push(projected_installed_version_status(
            capability,
            selection,
            definition,
            installed_version,
        ));
    }

    if projected_versions.is_empty() {
        projected_versions.push(version_status_for_capability(app_data_dir, capability));
    }

    projected_versions
}

fn projected_catalog_version_status(
    capability: &ManagedBinaryCapability,
    runtime: &ManagedRuntimePersistedRuntime,
    selection: &super::contracts::ManagedRuntimeSelectionState,
    definition: &'static dyn super::definitions::ManagedBinaryDefinition,
    catalog_version: &ManagedRuntimeCatalogVersion,
) -> ManagedRuntimeVersionStatus {
    let installed_version = runtime
        .versions
        .iter()
        .find(|version| version.version == catalog_version.version);

    if let Some(installed_version) = installed_version {
        return projected_installed_version_status(
            capability,
            selection,
            definition,
            installed_version,
        );
    }

    let version = catalog_version.version.as_str();
    ManagedRuntimeVersionStatus {
        version: Some(catalog_version.version.clone()),
        display_label: catalog_version.display_label.clone(),
        runtime_key: catalog_version.runtime_key.clone(),
        platform_key: catalog_version.platform_key.clone(),
        install_root: None,
        executable_name: definition.executable_name().to_string(),
        executable_ready: false,
        install_state: ManagedBinaryInstallState::Missing,
        readiness_state: ManagedRuntimeReadinessState::Missing,
        catalog_available: true,
        installable: capability.can_install,
        selected: selection.selected_version.as_deref() == Some(version),
        active: selection.active_version.as_deref() == Some(version),
    }
}

fn projected_installed_version_status(
    capability: &ManagedBinaryCapability,
    selection: &super::contracts::ManagedRuntimeSelectionState,
    definition: &'static dyn super::definitions::ManagedBinaryDefinition,
    version: &ManagedRuntimePersistedVersion,
) -> ManagedRuntimeVersionStatus {
    ManagedRuntimeVersionStatus {
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
        install_state: if version.install_root.is_some() {
            ManagedBinaryInstallState::Installed
        } else {
            ManagedBinaryInstallState::Missing
        },
        readiness_state: version.readiness_state,
        catalog_available: true,
        installable: capability.can_install,
        selected: selection.selected_version.as_deref() == Some(version.version.as_str()),
        active: selection.active_version.as_deref() == Some(version.version.as_str()),
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
        catalog_available: false,
        installable: false,
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
            detail: Some(
                "Managed runtime install paused with retained download artifact".to_string(),
            ),
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
    runtime_key: &str,
    platform_key: &str,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = None;
    runtime.active_job_artifact = None;
    upsert_persisted_version(
        runtime,
        ManagedRuntimePersistedVersion {
            version: version.to_string(),
            runtime_key: Some(runtime_key.to_string()),
            platform_key: Some(platform_key.to_string()),
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
#[path = "operations_tests.rs"]
mod tests;
