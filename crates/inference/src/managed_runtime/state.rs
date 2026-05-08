use super::contracts::{
    ManagedBinaryId, ManagedRuntimeCatalogVersion, ManagedRuntimeJobState, ManagedRuntimeJobStatus,
    ManagedRuntimeReadinessState, ManagedRuntimeSelectionState,
};
use super::paths::managed_runtime_dir;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const MANAGED_RUNTIME_STATE_SCHEMA_VERSION: u32 = 1;
const MANAGED_RUNTIME_STATE_FILE: &str = "state.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRuntimeHistoryEventKind {
    Installed,
    Removed,
    SelectionUpdated,
    RecoveryReconciled,
    ValidationFailed,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeInstallHistoryEntry {
    pub event: ManagedRuntimeHistoryEventKind,
    pub version: Option<String>,
    pub at_ms: u64,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimePersistedVersion {
    pub version: String,
    #[serde(default)]
    pub runtime_key: Option<String>,
    #[serde(default)]
    pub platform_key: Option<String>,
    pub readiness_state: ManagedRuntimeReadinessState,
    pub install_root: Option<String>,
    pub last_ready_at_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimePersistedJobArtifact {
    pub version: String,
    pub archive_name: String,
    pub archive_path: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimePersistedRuntime {
    pub id: ManagedBinaryId,
    #[serde(default)]
    pub catalog_versions: Vec<ManagedRuntimeCatalogVersion>,
    pub catalog_refreshed_at_ms: Option<u64>,
    #[serde(default)]
    pub versions: Vec<ManagedRuntimePersistedVersion>,
    #[serde(default)]
    pub selection: ManagedRuntimeSelectionState,
    pub active_job: Option<ManagedRuntimeJobStatus>,
    pub active_job_artifact: Option<ManagedRuntimePersistedJobArtifact>,
    #[serde(default)]
    pub install_history: Vec<ManagedRuntimeInstallHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimePersistedState {
    pub schema_version: u32,
    #[serde(default)]
    pub runtimes: Vec<ManagedRuntimePersistedRuntime>,
}

impl Default for ManagedRuntimePersistedState {
    fn default() -> Self {
        Self {
            schema_version: MANAGED_RUNTIME_STATE_SCHEMA_VERSION,
            runtimes: Vec::new(),
        }
    }
}

pub fn load_managed_runtime_state(
    app_data_dir: &Path,
) -> Result<ManagedRuntimePersistedState, String> {
    let path = state_path(app_data_dir);
    if !path.exists() {
        return Ok(ManagedRuntimePersistedState::default());
    }

    let contents = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read managed runtime state {:?}: {}", path, e))?;
    let filtered_contents = drop_unsupported_runtime_entries(&contents)
        .map_err(|e| format!("Failed to parse managed runtime state {:?}: {}", path, e))?;
    let mut state: ManagedRuntimePersistedState = serde_json::from_value(filtered_contents)
        .map_err(|e| format!("Failed to parse managed runtime state {:?}: {}", path, e))?;

    if state.schema_version == 0 {
        state.schema_version = MANAGED_RUNTIME_STATE_SCHEMA_VERSION;
    }

    Ok(state)
}

fn drop_unsupported_runtime_entries(contents: &str) -> Result<Value, serde_json::Error> {
    let mut value = serde_json::from_str::<Value>(contents)?;
    if let Some(runtimes) = value.get_mut("runtimes").and_then(Value::as_array_mut) {
        runtimes.retain(|runtime| {
            runtime
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| ManagedBinaryId::all().iter().any(|known| known.key() == id))
        });
    }

    Ok(value)
}

pub fn reconcile_interrupted_managed_runtime_jobs(app_data_dir: &Path) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    if reconcile_interrupted_jobs(&mut state) {
        save_managed_runtime_state(app_data_dir, &state)?;
    }
    Ok(())
}

pub fn save_managed_runtime_state(
    app_data_dir: &Path,
    state: &ManagedRuntimePersistedState,
) -> Result<(), String> {
    let runtime_dir = managed_runtime_dir(app_data_dir);
    fs::create_dir_all(&runtime_dir).map_err(|e| {
        format!(
            "Failed to create managed runtime directory {:?}: {}",
            runtime_dir, e
        )
    })?;

    let path = state_path(app_data_dir);
    let temp_path = temp_state_path(&path);
    let contents = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize managed runtime state: {}", e))?;

    fs::write(&temp_path, contents).map_err(|e| {
        format!(
            "Failed to write managed runtime temp state {:?}: {}",
            temp_path, e
        )
    })?;
    fs::rename(&temp_path, &path)
        .map_err(|e| format!("Failed to finalize managed runtime state {:?}: {}", path, e))?;

    Ok(())
}

pub(crate) fn runtime_state_entry_mut(
    state: &mut ManagedRuntimePersistedState,
    id: ManagedBinaryId,
) -> Option<&mut ManagedRuntimePersistedRuntime> {
    state.runtimes.iter_mut().find(|runtime| runtime.id == id)
}

pub(crate) fn ensure_runtime_state_entry(
    state: &mut ManagedRuntimePersistedState,
    id: ManagedBinaryId,
) -> &mut ManagedRuntimePersistedRuntime {
    if let Some(index) = state.runtimes.iter().position(|runtime| runtime.id == id) {
        return &mut state.runtimes[index];
    }

    state.runtimes.push(ManagedRuntimePersistedRuntime {
        id,
        catalog_versions: Vec::new(),
        catalog_refreshed_at_ms: None,
        versions: Vec::new(),
        selection: ManagedRuntimeSelectionState::default(),
        active_job: None,
        active_job_artifact: None,
        install_history: Vec::new(),
    });
    state
        .runtimes
        .last_mut()
        .expect("managed runtime state entry should exist after push")
}

pub(crate) fn runtime_state_entry(
    state: &ManagedRuntimePersistedState,
    id: ManagedBinaryId,
) -> Option<&ManagedRuntimePersistedRuntime> {
    state.runtimes.iter().find(|runtime| runtime.id == id)
}

fn state_path(app_data_dir: &Path) -> PathBuf {
    managed_runtime_dir(app_data_dir).join(MANAGED_RUNTIME_STATE_FILE)
}

fn temp_state_path(path: &Path) -> PathBuf {
    let mut temp_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    temp_name.push(format!(".tmp-{}", uuid::Uuid::new_v4()));
    path.with_file_name(temp_name)
}

fn reconcile_interrupted_jobs(state: &mut ManagedRuntimePersistedState) -> bool {
    let mut reconciled = false;
    for runtime in &mut state.runtimes {
        let Some(job) = runtime.active_job.as_mut() else {
            continue;
        };

        if !job_state_requires_recovery(job.state) {
            continue;
        }

        let detail = format!(
            "Interrupted {} job was reconciled during startup",
            job.status
        );
        job.state = ManagedRuntimeJobState::Failed;
        job.status = "Interrupted before completion".to_string();
        if job.error.is_none() {
            job.error = Some(detail.clone());
        }
        if runtime.active_job_artifact.is_some() {
            job.resumable = true;
        }

        runtime
            .install_history
            .push(ManagedRuntimeInstallHistoryEntry {
                event: ManagedRuntimeHistoryEventKind::RecoveryReconciled,
                version: None,
                at_ms: current_unix_timestamp_ms(),
                detail: Some(detail),
            });
        reconciled = true;
    }
    reconciled
}

fn job_state_requires_recovery(state: ManagedRuntimeJobState) -> bool {
    matches!(
        state,
        ManagedRuntimeJobState::Queued
            | ManagedRuntimeJobState::Downloading
            | ManagedRuntimeJobState::Extracting
            | ManagedRuntimeJobState::Validating
    )
}

fn current_unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::{
        load_managed_runtime_state, reconcile_interrupted_managed_runtime_jobs,
        runtime_state_entry, save_managed_runtime_state, state_path, ManagedBinaryId,
        ManagedRuntimeHistoryEventKind, ManagedRuntimeJobState, ManagedRuntimeJobStatus,
        ManagedRuntimePersistedJobArtifact, ManagedRuntimePersistedRuntime,
        ManagedRuntimePersistedState, ManagedRuntimeSelectionState,
    };
    use std::fs;

    #[test]
    fn load_returns_default_when_state_file_is_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let state = load_managed_runtime_state(temp_dir.path()).expect("load default state");

        assert_eq!(state, ManagedRuntimePersistedState::default());
    }

    #[test]
    fn save_and_load_round_trip_preserves_runtime_state() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let state = ManagedRuntimePersistedState {
            schema_version: 1,
            runtimes: vec![ManagedRuntimePersistedRuntime {
                id: ManagedBinaryId::LlamaCpp,
                catalog_versions: Vec::new(),
                catalog_refreshed_at_ms: None,
                versions: Vec::new(),
                selection: ManagedRuntimeSelectionState {
                    selected_version: Some("b8248".to_string()),
                    active_version: None,
                    default_version: None,
                },
                active_job: None,
                active_job_artifact: None,
                install_history: Vec::new(),
            }],
        };

        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");
        let loaded = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");

        let runtime =
            runtime_state_entry(&loaded, ManagedBinaryId::LlamaCpp).expect("llama runtime entry");
        assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
    }

    #[test]
    fn load_drops_retired_runtime_entries_from_persisted_state() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = state_path(temp_dir.path());
        fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
        fs::write(
            &path,
            serde_json::json!({
                "schema_version": 1,
                "runtimes": [
                    {
                        "id": "ollama",
                        "catalog_refreshed_at_ms": null,
                        "versions": [],
                        "selection": {},
                        "active_job": null,
                        "active_job_artifact": null
                    },
                    {
                        "id": "llama_cpp",
                        "catalog_refreshed_at_ms": null,
                        "versions": [],
                        "selection": {
                            "selected_version": "b8248"
                        },
                        "active_job": null,
                        "active_job_artifact": null
                    }
                ]
            })
            .to_string(),
        )
        .expect("write old state");

        let loaded = load_managed_runtime_state(temp_dir.path()).expect("load filtered state");

        assert_eq!(loaded.runtimes.len(), 1);
        let runtime =
            runtime_state_entry(&loaded, ManagedBinaryId::LlamaCpp).expect("llama runtime entry");
        assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
    }

    #[test]
    fn load_preserves_in_progress_jobs_without_reconciling() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let state = ManagedRuntimePersistedState {
            schema_version: 1,
            runtimes: vec![ManagedRuntimePersistedRuntime {
                id: ManagedBinaryId::LlamaCpp,
                catalog_versions: Vec::new(),
                catalog_refreshed_at_ms: None,
                versions: Vec::new(),
                selection: ManagedRuntimeSelectionState::default(),
                active_job: Some(ManagedRuntimeJobStatus {
                    state: ManagedRuntimeJobState::Downloading,
                    status: "Downloading".to_string(),
                    current: 5,
                    total: 10,
                    resumable: true,
                    cancellable: true,
                    error: None,
                }),
                active_job_artifact: Some(ManagedRuntimePersistedJobArtifact {
                    version: "b8248".to_string(),
                    archive_name: "llama-b8248.tar.gz".to_string(),
                    archive_path: temp_dir
                        .path()
                        .join("partial-llama.tar.gz")
                        .display()
                        .to_string(),
                    downloaded_bytes: 5,
                    total_bytes: 10,
                }),
                install_history: Vec::new(),
            }],
        };

        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");
        let loaded = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
        let runtime =
            runtime_state_entry(&loaded, ManagedBinaryId::LlamaCpp).expect("llama runtime entry");
        let job = runtime.active_job.as_ref().expect("active job");

        assert_eq!(job.state, ManagedRuntimeJobState::Downloading);
        assert!(runtime.install_history.is_empty());
    }

    #[test]
    fn startup_reconcile_marks_interrupted_jobs_failed_once() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let state = ManagedRuntimePersistedState {
            schema_version: 1,
            runtimes: vec![ManagedRuntimePersistedRuntime {
                id: ManagedBinaryId::LlamaCpp,
                catalog_versions: Vec::new(),
                catalog_refreshed_at_ms: None,
                versions: Vec::new(),
                selection: ManagedRuntimeSelectionState::default(),
                active_job: Some(ManagedRuntimeJobStatus {
                    state: ManagedRuntimeJobState::Downloading,
                    status: "Downloading".to_string(),
                    current: 5,
                    total: 10,
                    resumable: true,
                    cancellable: true,
                    error: None,
                }),
                active_job_artifact: Some(ManagedRuntimePersistedJobArtifact {
                    version: "b8248".to_string(),
                    archive_name: "llama-b8248.tar.gz".to_string(),
                    archive_path: temp_dir
                        .path()
                        .join("partial-llama.tar.gz")
                        .display()
                        .to_string(),
                    downloaded_bytes: 5,
                    total_bytes: 10,
                }),
                install_history: Vec::new(),
            }],
        };

        save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");
        reconcile_interrupted_managed_runtime_jobs(temp_dir.path())
            .expect("reconcile runtime state");
        reconcile_interrupted_managed_runtime_jobs(temp_dir.path())
            .expect("second reconcile is idempotent");
        let loaded = load_managed_runtime_state(temp_dir.path()).expect("load reconciled state");
        let runtime =
            runtime_state_entry(&loaded, ManagedBinaryId::LlamaCpp).expect("llama runtime entry");
        let job = runtime.active_job.as_ref().expect("reconciled job");

        assert_eq!(job.state, ManagedRuntimeJobState::Failed);
        assert!(job.resumable);
        assert!(runtime.active_job_artifact.is_some());
        assert_eq!(
            runtime
                .install_history
                .iter()
                .filter(|entry| entry.event == ManagedRuntimeHistoryEventKind::RecoveryReconciled)
                .count(),
            1
        );
    }
}
