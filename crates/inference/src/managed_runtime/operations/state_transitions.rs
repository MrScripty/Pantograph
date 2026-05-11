use super::super::contracts::{
    ManagedBinaryId, ManagedRuntimeJobState, ManagedRuntimeJobStatus, ManagedRuntimeReadinessState,
    ManagedRuntimeSelectionState,
};
use super::super::definitions::definition;
use super::super::paths::managed_install_dir;
use super::super::state::{
    ensure_runtime_state_entry, load_managed_runtime_state, runtime_state_entry,
    runtime_state_entry_mut, save_managed_runtime_state, ManagedRuntimeHistoryEventKind,
    ManagedRuntimeInstallHistoryEntry, ManagedRuntimePersistedJobArtifact,
    ManagedRuntimePersistedRuntime, ManagedRuntimePersistedVersion,
};
use super::{take_cancellation_request, take_pause_request};
use crate::RuntimeVariantId;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn persist_active_job(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    job: ManagedRuntimeJobStatus,
) -> Result<(), String> {
    persist_active_job_with_artifact(app_data_dir, id, job, None)
}

pub(super) fn persist_active_job_with_artifact(
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

pub(super) fn persist_failed_job(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    runtime_variant_id: RuntimeVariantId,
    status: String,
    error: String,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        runtime_variant_id: runtime_variant_id.clone(),
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
            runtime_variant_id: Some(runtime_variant_id.clone()),
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
            runtime_variant_id,
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
    let runtime_variant_id = job_artifact.runtime_variant_id.clone();
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        runtime_variant_id: runtime_variant_id.clone(),
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
            runtime_variant_id,
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
    runtime_variant_id: RuntimeVariantId,
    current: u64,
    total: u64,
    job_artifact: Option<ManagedRuntimePersistedJobArtifact>,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        runtime_variant_id: runtime_variant_id.clone(),
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
            runtime_variant_id,
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

pub(super) fn discard_retained_job_artifact(
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
    let runtime_variant_id = artifact.runtime_variant_id.clone();
    let current = artifact.downloaded_bytes;
    let total = artifact.total_bytes;
    persist_cancelled_job(
        app_data_dir,
        id,
        &version,
        runtime_variant_id,
        current,
        total,
        None,
    )
}

pub(super) fn persist_install_success(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    install_dir: &Path,
    runtime_key: &str,
    runtime_variant_id: RuntimeVariantId,
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
            runtime_variant_id: Some(runtime_variant_id.clone()),
            platform_key: Some(platform_key.to_string()),
            readiness_state: ManagedRuntimeReadinessState::Ready,
            install_root: Some(install_dir.display().to_string()),
            last_ready_at_ms: Some(current_unix_timestamp_ms()),
            last_error: None,
        },
    );
    if runtime.selection.selected_version.is_none() {
        runtime.selection.selected_version = Some(version.to_string());
        runtime.selection.selected_runtime_variant_id = Some(runtime_variant_id.clone());
    }
    if runtime.selection.default_version.is_none() {
        runtime.selection.default_version = Some(version.to_string());
        runtime.selection.default_runtime_variant_id = Some(runtime_variant_id.clone());
    }
    runtime.selection.active_version = Some(version.to_string());
    runtime.selection.active_runtime_variant_id = Some(runtime_variant_id.clone());
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            runtime_variant_id,
            event: ManagedRuntimeHistoryEventKind::Installed,
            version: Some(version.to_string()),
            at_ms: current_unix_timestamp_ms(),
            detail: Some(format!("Installed into {}", install_dir.display())),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

pub(super) fn persist_remove_success(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let Some(runtime) = runtime_state_entry_mut(&mut state, id) else {
        return Ok(());
    };
    let runtime_variant_id = default_runtime_variant_id(id);

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
        runtime.selection.selected_runtime_variant_id = None;
    }
    if runtime
        .selection
        .default_version
        .as_ref()
        .is_some_and(|default| removed_versions.contains(default))
    {
        runtime.selection.default_version = None;
        runtime.selection.default_runtime_variant_id = None;
    }
    runtime.selection.active_version = None;
    runtime.selection.active_runtime_variant_id = None;
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            runtime_variant_id,
            event: ManagedRuntimeHistoryEventKind::Removed,
            version: removed_versions.first().cloned(),
            at_ms: current_unix_timestamp_ms(),
            detail: Some("Managed install removed".to_string()),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

pub(super) fn persist_remove_version_success(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let Some(runtime) = runtime_state_entry_mut(&mut state, id) else {
        return Ok(());
    };
    let runtime_variant_id = default_runtime_variant_id(id);

    runtime
        .versions
        .retain(|installed_version| installed_version.version != version);
    runtime.active_job = None;
    runtime.active_job_artifact = None;
    clear_selection_version(
        &mut runtime.selection.selected_version,
        &mut runtime.selection.selected_runtime_variant_id,
        version,
    );
    clear_selection_version(
        &mut runtime.selection.default_version,
        &mut runtime.selection.default_runtime_variant_id,
        version,
    );
    clear_selection_version(
        &mut runtime.selection.active_version,
        &mut runtime.selection.active_runtime_variant_id,
        version,
    );
    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            runtime_variant_id,
            event: ManagedRuntimeHistoryEventKind::Removed,
            version: Some(version.to_string()),
            at_ms: current_unix_timestamp_ms(),
            detail: Some("Managed runtime version removed".to_string()),
        });
    save_managed_runtime_state(app_data_dir, &state)
}

pub(super) fn finish_requested_cancellation(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
    runtime_variant_id: RuntimeVariantId,
    current: u64,
    total: u64,
    job_artifact: Option<ManagedRuntimePersistedJobArtifact>,
) -> Result<bool, String> {
    if !take_cancellation_request(id) {
        return Ok(false);
    }

    persist_cancelled_job(
        app_data_dir,
        id,
        version,
        runtime_variant_id,
        current,
        total,
        job_artifact,
    )?;
    Ok(true)
}

pub(super) fn finish_requested_pause(
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
    if let Some(existing) = runtime.versions.iter_mut().find(|existing| {
        existing.version == version.version
            && existing.runtime_variant_id == version.runtime_variant_id
    }) {
        *existing = version;
        return;
    }

    runtime.versions.push(version);
}

pub(super) fn current_unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) enum SelectionTarget {
    Selected,
    Default,
}

pub(super) fn update_runtime_selection(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: Option<&str>,
    runtime_variant_id: Option<&RuntimeVariantId>,
    target: SelectionTarget,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);

    let runtime_variant_id = if let Some(version) = version {
        let candidates = runtime
            .versions
            .iter()
            .filter(|entry| entry.version == version);
        let mut candidates: Vec<_> = if let Some(runtime_variant_id) = runtime_variant_id {
            candidates
                .filter(|entry| entry.runtime_variant_id.as_ref() == Some(runtime_variant_id))
                .collect()
        } else {
            candidates.collect()
        };

        if candidates.is_empty() {
            return Err(format!(
                "{} version '{}'{} is not installed",
                id.display_name(),
                version,
                runtime_variant_id
                    .map(|variant| format!(" variant '{}'", variant))
                    .unwrap_or_default()
            ));
        };
        if runtime_variant_id.is_none() && candidates.len() > 1 {
            return Err(format!(
                "{} version '{}' has multiple runtime variants; provide runtime variant id",
                id.display_name(),
                version
            ));
        }

        let persisted_version = candidates.remove(0);

        if persisted_version.readiness_state != ManagedRuntimeReadinessState::Ready {
            return Err(format!(
                "{} version '{}' variant '{}' is not ready for selection",
                id.display_name(),
                version,
                persisted_version
                    .runtime_variant_id
                    .as_ref()
                    .map(RuntimeVariantId::as_str)
                    .unwrap_or("<missing>")
            ));
        }

        persisted_version
            .runtime_variant_id
            .clone()
            .ok_or_else(|| {
                format!(
                    "{} version '{}' does not have a canonical runtime variant id",
                    id.display_name(),
                    version
                )
            })?
    } else {
        if let Some(runtime_variant_id) = runtime_variant_id {
            return Err(format!(
                "{} runtime variant '{}' cannot be selected without a version",
                id.display_name(),
                runtime_variant_id
            ));
        }
        default_runtime_variant_id(id)
    };

    let version = version.map(str::to_string);
    match target {
        SelectionTarget::Selected => {
            runtime.selection.selected_version = version.clone();
            runtime.selection.selected_runtime_variant_id =
                version.as_ref().map(|_| runtime_variant_id.clone());
        }
        SelectionTarget::Default => {
            runtime.selection.default_version = version.clone();
            runtime.selection.default_runtime_variant_id =
                version.as_ref().map(|_| runtime_variant_id.clone());
        }
    }

    runtime
        .install_history
        .push(ManagedRuntimeInstallHistoryEntry {
            runtime_variant_id,
            event: ManagedRuntimeHistoryEventKind::SelectionUpdated,
            version,
            at_ms: current_unix_timestamp_ms(),
            detail: Some(selection_target_label(target).to_string()),
        });

    save_managed_runtime_state(app_data_dir, &state)
}

#[cfg(test)]
pub(super) fn resolve_runtime_install_dir(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<PathBuf, String> {
    resolve_runtime_install_dir_with_mode(app_data_dir, id, InstallDirResolutionMode::Strict)
}

pub(super) struct RuntimeInstallTarget {
    pub(super) install_dir: PathBuf,
    pub(super) runtime_variant_id: RuntimeVariantId,
}

pub(super) fn resolve_runtime_install_target(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<RuntimeInstallTarget, String> {
    let state = load_managed_runtime_state(app_data_dir)?;
    let runtime = runtime_state_entry(&state, id)
        .ok_or_else(|| format!("{} does not have managed runtime state", id.display_name()))?;
    let (version, runtime_variant_id) = preferred_runtime_selection(&runtime.selection)
        .ok_or_else(|| {
            format!(
                "{} does not have a selected managed runtime version and variant",
                id.display_name()
            )
        })?;
    let persisted_version = runtime
        .versions
        .iter()
        .find(|entry| {
            entry.version == version
                && entry.runtime_variant_id.as_ref() == Some(runtime_variant_id)
        })
        .ok_or_else(|| {
            format!(
                "{} selected version '{}' variant '{}' is not installed",
                id.display_name(),
                version,
                runtime_variant_id
            )
        })?;
    let fallback_install_dir = managed_install_dir(app_data_dir, id);

    Ok(RuntimeInstallTarget {
        install_dir: persisted_version
            .install_root
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or(fallback_install_dir),
        runtime_variant_id: runtime_variant_id.clone(),
    })
}

pub(super) fn runtime_install_dir_for_projection(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<PathBuf, String> {
    resolve_runtime_install_dir_with_mode(app_data_dir, id, InstallDirResolutionMode::BestEffort)
}

enum InstallDirResolutionMode {
    #[cfg(test)]
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

    let preferred_selection = preferred_runtime_selection(&runtime.selection);

    let Some((version, runtime_variant_id)) = preferred_selection else {
        return Ok(fallback_install_dir);
    };

    let Some(persisted_version) = runtime.versions.iter().find(|entry| {
        entry.version == version && entry.runtime_variant_id.as_ref() == Some(runtime_variant_id)
    }) else {
        return match mode {
            #[cfg(test)]
            InstallDirResolutionMode::Strict => Err(format!(
                "{} selected version '{}' variant '{}' is not installed",
                id.display_name(),
                version,
                runtime_variant_id
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

fn preferred_runtime_selection(
    selection: &ManagedRuntimeSelectionState,
) -> Option<(&str, &RuntimeVariantId)> {
    selection
        .selected_version
        .as_deref()
        .zip(selection.selected_runtime_variant_id.as_ref())
        .or_else(|| {
            selection
                .active_version
                .as_deref()
                .zip(selection.active_runtime_variant_id.as_ref())
        })
        .or_else(|| {
            selection
                .default_version
                .as_deref()
                .zip(selection.default_runtime_variant_id.as_ref())
        })
}

fn selection_target_label(target: SelectionTarget) -> &'static str {
    match target {
        SelectionTarget::Selected => "selected_version_updated",
        SelectionTarget::Default => "default_version_updated",
    }
}

fn default_runtime_variant_id(id: ManagedBinaryId) -> RuntimeVariantId {
    definition(id).default_runtime_variant_id()
}

fn clear_selection_version(
    selection: &mut Option<String>,
    runtime_variant_id: &mut Option<RuntimeVariantId>,
    removed_version: &str,
) {
    if selection.as_deref() == Some(removed_version) {
        *selection = None;
        *runtime_variant_id = None;
    }
}
