use std::path::Path;

use inference::{
    cancel_binary_download, download_binary, list_managed_runtime_snapshots,
    load_managed_runtime_state, pause_binary_download, refresh_managed_runtime_catalogs,
    remove_binary, select_managed_runtime_version, set_default_managed_runtime_version,
    DownloadProgress, ManagedBinaryId, ManagedRuntimeInstallHistoryEntry,
    ManagedRuntimeJobArtifactStatus, ManagedRuntimeJobStatus, ManagedRuntimeSelectionState,
    ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeManagerRuntimeView {
    pub id: ManagedBinaryId,
    pub display_name: String,
    pub install_state: inference::ManagedBinaryInstallState,
    pub readiness_state: inference::ManagedRuntimeReadinessState,
    pub available: bool,
    pub can_install: bool,
    pub can_remove: bool,
    pub missing_files: Vec<String>,
    pub unavailable_reason: Option<String>,
    #[serde(default)]
    pub versions: Vec<ManagedRuntimeVersionStatus>,
    #[serde(default)]
    pub selection: ManagedRuntimeSelectionState,
    pub active_job: Option<ManagedRuntimeJobStatus>,
    pub job_artifact: Option<ManagedRuntimeJobArtifactStatus>,
    #[serde(default)]
    pub install_history: Vec<ManagedRuntimeInstallHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeManagerProgress {
    pub runtime_id: ManagedBinaryId,
    pub status: String,
    pub current: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
    pub runtime: ManagedRuntimeManagerRuntimeView,
}

pub fn list_managed_runtime_manager_runtimes(
    app_data_dir: &Path,
) -> Result<Vec<ManagedRuntimeManagerRuntimeView>, String> {
    let snapshots = list_managed_runtime_snapshots(app_data_dir)?;
    let state = load_managed_runtime_state(app_data_dir)?;
    Ok(snapshots
        .iter()
        .map(|snapshot| runtime_view_from_snapshot(snapshot, &state))
        .collect())
}

pub fn inspect_managed_runtime_manager_runtime(
    app_data_dir: &Path,
    runtime_id: ManagedBinaryId,
) -> Result<ManagedRuntimeManagerRuntimeView, String> {
    list_managed_runtime_manager_runtimes(app_data_dir)?
        .into_iter()
        .find(|runtime| runtime.id == runtime_id)
        .ok_or_else(|| format!("managed runtime '{}' was not found", runtime_id.key()))
}

pub async fn install_managed_runtime_manager_runtime<F>(
    app_data_dir: &Path,
    runtime_id: ManagedBinaryId,
    version: Option<&str>,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(ManagedRuntimeManagerProgress),
{
    let mut last_runtime = inspect_managed_runtime_manager_runtime(app_data_dir, runtime_id)?;
    download_binary(app_data_dir, runtime_id, version, |progress| {
        let runtime = inspect_managed_runtime_manager_runtime(app_data_dir, runtime_id)
            .unwrap_or_else(|_| last_runtime.clone());
        last_runtime = runtime.clone();
        on_progress(project_progress(runtime_id, progress, runtime));
    })
    .await
}

pub async fn refresh_managed_runtime_manager_catalog_views(
    app_data_dir: &Path,
) -> Result<Vec<ManagedRuntimeManagerRuntimeView>, String> {
    let snapshots = refresh_managed_runtime_catalogs(app_data_dir).await?;
    let state = load_managed_runtime_state(app_data_dir)?;
    Ok(snapshots
        .iter()
        .map(|snapshot| runtime_view_from_snapshot(snapshot, &state))
        .collect())
}

pub async fn remove_managed_runtime_manager_runtime(
    app_data_dir: &Path,
    runtime_id: ManagedBinaryId,
) -> Result<(), String> {
    remove_binary(app_data_dir, runtime_id).await
}

pub fn cancel_managed_runtime_manager_job(
    app_data_dir: &Path,
    runtime_id: ManagedBinaryId,
) -> Result<(), String> {
    cancel_binary_download(app_data_dir, runtime_id)
}

pub fn pause_managed_runtime_manager_job(
    app_data_dir: &Path,
    runtime_id: ManagedBinaryId,
) -> Result<(), String> {
    pause_binary_download(app_data_dir, runtime_id)
}

pub fn select_managed_runtime_manager_version(
    app_data_dir: &Path,
    runtime_id: ManagedBinaryId,
    version: Option<&str>,
) -> Result<ManagedRuntimeManagerRuntimeView, String> {
    select_managed_runtime_version(app_data_dir, runtime_id, version)?;
    inspect_managed_runtime_manager_runtime(app_data_dir, runtime_id)
}

pub fn set_default_managed_runtime_manager_version_view(
    app_data_dir: &Path,
    runtime_id: ManagedBinaryId,
    version: Option<&str>,
) -> Result<ManagedRuntimeManagerRuntimeView, String> {
    set_default_managed_runtime_version(app_data_dir, runtime_id, version)?;
    inspect_managed_runtime_manager_runtime(app_data_dir, runtime_id)
}

fn runtime_view_from_snapshot(
    snapshot: &ManagedRuntimeSnapshot,
    state: &inference::ManagedRuntimePersistedState,
) -> ManagedRuntimeManagerRuntimeView {
    let install_history = state
        .runtimes
        .iter()
        .find(|runtime| runtime.id == snapshot.id)
        .map(|runtime| runtime.install_history.clone())
        .unwrap_or_default();

    ManagedRuntimeManagerRuntimeView {
        id: snapshot.id,
        display_name: snapshot.display_name.clone(),
        install_state: snapshot.install_state,
        readiness_state: snapshot.readiness_state,
        available: snapshot.available,
        can_install: snapshot.can_install,
        can_remove: snapshot.can_remove,
        missing_files: snapshot.missing_files.clone(),
        unavailable_reason: snapshot.unavailable_reason.clone(),
        versions: snapshot.versions.clone(),
        selection: snapshot.selection.clone(),
        active_job: snapshot.active_job.clone(),
        job_artifact: snapshot.job_artifact.clone(),
        install_history,
    }
}

fn project_progress(
    runtime_id: ManagedBinaryId,
    progress: DownloadProgress,
    runtime: ManagedRuntimeManagerRuntimeView,
) -> ManagedRuntimeManagerProgress {
    ManagedRuntimeManagerProgress {
        runtime_id,
        status: progress.status,
        current: progress.current,
        total: progress.total,
        done: progress.done,
        error: progress.error,
        runtime,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        inspect_managed_runtime_manager_runtime, list_managed_runtime_manager_runtimes,
        project_progress, select_managed_runtime_manager_version, ManagedRuntimeManagerProgress,
        ManagedRuntimeManagerRuntimeView,
    };
    use inference::{
        load_managed_runtime_state, save_managed_runtime_state, DownloadProgress, ManagedBinaryId,
        ManagedRuntimeHistoryEventKind, ManagedRuntimeInstallHistoryEntry,
        ManagedRuntimePersistedRuntime, ManagedRuntimePersistedVersion,
        ManagedRuntimeReadinessState, ManagedRuntimeSelectionState,
    };

    #[test]
    fn manager_list_projects_install_history_and_selection() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        state.runtimes.push(ManagedRuntimePersistedRuntime {
            id: ManagedBinaryId::LlamaCpp,
            catalog_versions: Vec::new(),
            catalog_refreshed_at_ms: None,
            versions: vec![ManagedRuntimePersistedVersion {
                version: "b8248".to_string(),
                runtime_key: Some("llama_cpp".to_string()),
                platform_key: Some("linux-x86_64".to_string()),
                readiness_state: ManagedRuntimeReadinessState::Ready,
                install_root: Some("/tmp/llama-cpp-b8248".to_string()),
                last_ready_at_ms: Some(42),
                last_error: None,
            }],
            selection: ManagedRuntimeSelectionState {
                selected_version: Some("b8248".to_string()),
                active_version: None,
                default_version: Some("b8248".to_string()),
            },
            active_job: None,
            active_job_artifact: None,
            install_history: vec![ManagedRuntimeInstallHistoryEntry {
                event: ManagedRuntimeHistoryEventKind::Installed,
                version: Some("b8248".to_string()),
                at_ms: 42,
                detail: Some("Installed runtime".to_string()),
            }],
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let runtimes =
            list_managed_runtime_manager_runtimes(temp_dir.path()).expect("list managed runtimes");
        let runtime = runtimes
            .into_iter()
            .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
            .expect("llama.cpp runtime");

        assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
        assert_eq!(runtime.selection.default_version.as_deref(), Some("b8248"));
        assert_eq!(runtime.install_history.len(), 1);
        assert_eq!(
            runtime.install_history[0].event,
            ManagedRuntimeHistoryEventKind::Installed
        );
    }

    #[test]
    fn inspect_returns_requested_runtime_view() {
        let temp_dir = tempfile::tempdir().expect("temp dir");

        let runtime =
            inspect_managed_runtime_manager_runtime(temp_dir.path(), ManagedBinaryId::LlamaCpp)
                .expect("inspect managed runtime");

        assert_eq!(runtime.id, ManagedBinaryId::LlamaCpp);
        assert_eq!(runtime.display_name, "llama.cpp");
    }

    #[test]
    fn selection_update_returns_refreshed_runtime_view() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        state.runtimes.push(ManagedRuntimePersistedRuntime {
            id: ManagedBinaryId::LlamaCpp,
            catalog_versions: Vec::new(),
            catalog_refreshed_at_ms: None,
            versions: vec![ManagedRuntimePersistedVersion {
                version: "b8248".to_string(),
                runtime_key: Some("llama_cpp".to_string()),
                platform_key: Some("linux-x86_64".to_string()),
                readiness_state: ManagedRuntimeReadinessState::Ready,
                install_root: Some("/tmp/llama-cpp-b8248".to_string()),
                last_ready_at_ms: Some(42),
                last_error: None,
            }],
            selection: ManagedRuntimeSelectionState::default(),
            active_job: None,
            active_job_artifact: None,
            install_history: Vec::new(),
        });
        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

        let runtime = select_managed_runtime_manager_version(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            Some("b8248"),
        )
        .expect("select managed runtime version");

        assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
        assert!(runtime.install_history.iter().any(|entry| {
            entry.event == ManagedRuntimeHistoryEventKind::SelectionUpdated
                && entry.version.as_deref() == Some("b8248")
        }));
    }

    #[test]
    fn progress_projection_preserves_download_fields_and_runtime_id() {
        let runtime = ManagedRuntimeManagerRuntimeView {
            id: ManagedBinaryId::LlamaCpp,
            display_name: "llama.cpp".to_string(),
            install_state: inference::ManagedBinaryInstallState::Missing,
            readiness_state: ManagedRuntimeReadinessState::Downloading,
            available: true,
            can_install: true,
            can_remove: false,
            missing_files: Vec::new(),
            unavailable_reason: None,
            versions: Vec::new(),
            selection: ManagedRuntimeSelectionState::default(),
            active_job: None,
            job_artifact: None,
            install_history: Vec::new(),
        };
        let progress = project_progress(
            ManagedBinaryId::LlamaCpp,
            DownloadProgress {
                status: "Downloading".to_string(),
                current: 64,
                total: 128,
                done: false,
                error: None,
            },
            runtime.clone(),
        );

        assert_eq!(
            progress,
            ManagedRuntimeManagerProgress {
                runtime_id: ManagedBinaryId::LlamaCpp,
                status: "Downloading".to_string(),
                current: 64,
                total: 128,
                done: false,
                error: None,
                runtime,
            }
        );
    }
}
