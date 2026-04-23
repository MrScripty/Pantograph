use super::archive::extract_archive;
use super::catalog::fetch_managed_runtime_catalog;
use super::contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeJobState, ManagedRuntimeJobStatus,
    ManagedRuntimeSnapshot, ResolvedCommand,
};
use super::definitions::definition;
use super::paths::{
    extract_pid_file, managed_install_dir, managed_runtime_dir, managed_version_install_dir,
};
use super::state::{
    load_managed_runtime_state, runtime_state_entry, ManagedRuntimePersistedJobArtifact,
};
use futures_util::TryStreamExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::header::{CONTENT_RANGE, RANGE};
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod download;
mod projection;
mod state_transitions;

use self::download::{download_response_mode, existing_download_artifact, DownloadResponseMode};
use self::download::{persist_catalog_versions, resolve_download_source};
#[cfg(test)]
use self::projection::readiness_state_for_capability;
use self::projection::snapshot_from_capability;
use self::state_transitions::{
    discard_retained_job_artifact, persist_active_job, persist_active_job_with_artifact,
    persist_failed_job, update_runtime_selection, SelectionTarget,
};
use self::state_transitions::{
    finish_requested_cancellation, finish_requested_pause, persist_install_success,
    persist_remove_success, resolve_runtime_install_dir, runtime_install_dir_for_projection,
};
#[cfg(test)]
use super::contracts::ManagedRuntimeReadinessState;
#[cfg(test)]
use super::state::ensure_runtime_state_entry;

static TRANSITION_LOCKS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<tokio::sync::Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CANCELLATION_REQUESTS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PAUSE_REQUESTS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
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

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
