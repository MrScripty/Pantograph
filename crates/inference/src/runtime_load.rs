//! Runtime-load phase contracts for backend-owned readiness decisions.
//!
//! This module is intentionally pure contract/projection code. Process
//! spawning, HTTP probing, scheduler emission, and runtime registry policy stay
//! in their existing owners while they converge on these DTOs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    DeviceConfig, ManagedBinaryId, ManagedRuntimeJobState, ManagedRuntimeReadinessState,
    ManagedRuntimeSnapshot, ResolvedCommand,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLoadPhase {
    DependencyResolved,
    ProcessSpawning,
    ProcessSpawned,
    HttpReady,
    RequestedModelActive,
    LoadCompleted,
    LoadFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeLoadFacts {
    pub runtime_id: ManagedBinaryId,
    pub display_name: String,
    pub readiness_state: ManagedRuntimeReadinessState,
    pub selected_version: Option<String>,
    pub active_version: Option<String>,
    pub default_version: Option<String>,
    pub install_root: Option<PathBuf>,
    pub missing_files: Vec<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeLoadCommandFacts {
    pub executable_path: PathBuf,
    pub working_directory: PathBuf,
    pub args: Vec<String>,
    pub env_overrides: Vec<(String, String)>,
    pub pid_file: Option<PathBuf>,
}

impl RuntimeLoadCommandFacts {
    pub fn from_resolved_command(command: &ResolvedCommand) -> Self {
        Self {
            executable_path: command.executable_path.clone(),
            working_directory: command.working_directory.clone(),
            args: command
                .args
                .iter()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect(),
            env_overrides: command
                .env_overrides
                .iter()
                .map(|(key, value)| {
                    (
                        key.to_string_lossy().to_string(),
                        value.to_string_lossy().to_string(),
                    )
                })
                .collect(),
            pid_file: command.pid_file.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LlamaCppActiveRuntimeDescriptor {
    pub mode: LlamaCppRuntimeMode,
    pub port: u16,
    pub model_path: PathBuf,
    pub mmproj_path: Option<PathBuf>,
    pub device: DeviceConfig,
    pub context_size: Option<u32>,
    pub cpu_threads: Option<u32>,
    pub batch_size: Option<u32>,
    pub ubatch_size: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlamaCppRuntimeMode {
    Inference,
    Embedding,
    Reranking,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeLoadPhaseRecord {
    pub phase: RuntimeLoadPhase,
    pub runtime: ManagedRuntimeLoadFacts,
    pub command: Option<RuntimeLoadCommandFacts>,
    pub active_runtime: Option<LlamaCppActiveRuntimeDescriptor>,
}

impl RuntimeLoadPhaseRecord {
    pub fn dependency_resolved(
        runtime: ManagedRuntimeLoadFacts,
        command: RuntimeLoadCommandFacts,
    ) -> Self {
        Self {
            phase: RuntimeLoadPhase::DependencyResolved,
            runtime,
            command: Some(command),
            active_runtime: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLoadReadinessError {
    ActiveMutatingJob {
        runtime_id: ManagedBinaryId,
        job_state: ManagedRuntimeJobState,
        status: String,
    },
    RuntimeNotReady {
        runtime_id: ManagedBinaryId,
        readiness_state: ManagedRuntimeReadinessState,
        missing_files: Vec<String>,
        unavailable_reason: Option<String>,
    },
}

pub fn managed_runtime_load_facts_from_snapshot(
    snapshot: &ManagedRuntimeSnapshot,
) -> Result<ManagedRuntimeLoadFacts, RuntimeLoadReadinessError> {
    if let Some(active_job) = snapshot.active_job.as_ref() {
        if is_mutating_job_state(active_job.state) {
            return Err(RuntimeLoadReadinessError::ActiveMutatingJob {
                runtime_id: snapshot.id,
                job_state: active_job.state,
                status: active_job.status.clone(),
            });
        }
    }

    if !snapshot.available || snapshot.readiness_state != ManagedRuntimeReadinessState::Ready {
        return Err(RuntimeLoadReadinessError::RuntimeNotReady {
            runtime_id: snapshot.id,
            readiness_state: snapshot.readiness_state,
            missing_files: snapshot.missing_files.clone(),
            unavailable_reason: snapshot.unavailable_reason.clone(),
        });
    }

    Ok(ManagedRuntimeLoadFacts {
        runtime_id: snapshot.id,
        display_name: snapshot.display_name.clone(),
        readiness_state: snapshot.readiness_state,
        selected_version: snapshot.selection.selected_version.clone(),
        active_version: snapshot.selection.active_version.clone(),
        default_version: snapshot.selection.default_version.clone(),
        install_root: selected_install_root(snapshot).map(PathBuf::from),
        missing_files: snapshot.missing_files.clone(),
        unavailable_reason: snapshot.unavailable_reason.clone(),
    })
}

fn selected_install_root(snapshot: &ManagedRuntimeSnapshot) -> Option<String> {
    snapshot
        .versions
        .iter()
        .find(|version| version.selected || version.active)
        .and_then(|version| version.install_root.clone())
        .or_else(|| {
            snapshot
                .versions
                .iter()
                .find_map(|version| version.install_root.clone())
        })
}

fn is_mutating_job_state(state: ManagedRuntimeJobState) -> bool {
    matches!(
        state,
        ManagedRuntimeJobState::Queued
            | ManagedRuntimeJobState::Downloading
            | ManagedRuntimeJobState::Extracting
            | ManagedRuntimeJobState::Validating
            | ManagedRuntimeJobState::Paused
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::{
        ManagedBinaryInstallState, ManagedRuntimeJobStatus, ManagedRuntimeSelectionState,
        ManagedRuntimeVersionStatus,
    };

    #[test]
    fn runtime_load_facts_reject_missing_runtime() {
        let snapshot = snapshot_with_readiness(ManagedRuntimeReadinessState::Missing, false);

        let error = managed_runtime_load_facts_from_snapshot(&snapshot)
            .expect_err("missing runtime must not produce load facts");

        assert_eq!(
            error,
            RuntimeLoadReadinessError::RuntimeNotReady {
                runtime_id: ManagedBinaryId::LlamaCpp,
                readiness_state: ManagedRuntimeReadinessState::Missing,
                missing_files: vec!["llama-server".to_string()],
                unavailable_reason: Some("llama.cpp executable is missing".to_string()),
            }
        );
    }

    #[test]
    fn runtime_load_facts_reject_partial_install() {
        let mut snapshot = snapshot_with_readiness(ManagedRuntimeReadinessState::Failed, false);
        snapshot.missing_files = vec!["libllama.so".to_string()];
        snapshot.unavailable_reason = Some("selected install is incomplete".to_string());

        let error = managed_runtime_load_facts_from_snapshot(&snapshot)
            .expect_err("partial runtime must not produce load facts");

        assert_eq!(
            error,
            RuntimeLoadReadinessError::RuntimeNotReady {
                runtime_id: ManagedBinaryId::LlamaCpp,
                readiness_state: ManagedRuntimeReadinessState::Failed,
                missing_files: vec!["libllama.so".to_string()],
                unavailable_reason: Some("selected install is incomplete".to_string()),
            }
        );
    }

    #[test]
    fn runtime_load_facts_reject_active_mutating_job() {
        let mut snapshot = snapshot_with_readiness(ManagedRuntimeReadinessState::Ready, true);
        snapshot.active_job = Some(ManagedRuntimeJobStatus {
            state: ManagedRuntimeJobState::Downloading,
            status: "downloading selected llama.cpp archive".to_string(),
            current: 1,
            total: 2,
            resumable: true,
            cancellable: true,
            error: None,
        });

        let error = managed_runtime_load_facts_from_snapshot(&snapshot)
            .expect_err("active mutating job must block runtime load facts");

        assert_eq!(
            error,
            RuntimeLoadReadinessError::ActiveMutatingJob {
                runtime_id: ManagedBinaryId::LlamaCpp,
                job_state: ManagedRuntimeJobState::Downloading,
                status: "downloading selected llama.cpp archive".to_string(),
            }
        );
    }

    #[test]
    fn dependency_resolved_phase_projects_managed_command_facts() {
        let snapshot = snapshot_with_readiness(ManagedRuntimeReadinessState::Ready, true);
        let runtime = managed_runtime_load_facts_from_snapshot(&snapshot)
            .expect("ready runtime should produce load facts");
        let command = ResolvedCommand {
            executable_path: PathBuf::from("/opt/pantograph/llama-server"),
            working_directory: PathBuf::from("/opt/pantograph"),
            args: vec![OsString::from("--port"), OsString::from("8080")],
            env_overrides: vec![(
                OsString::from("LD_LIBRARY_PATH"),
                OsString::from("/opt/lib"),
            )],
            pid_file: Some(PathBuf::from("/tmp/llama.pid")),
        };

        let phase = RuntimeLoadPhaseRecord::dependency_resolved(
            runtime,
            RuntimeLoadCommandFacts::from_resolved_command(&command),
        );

        assert_eq!(phase.phase, RuntimeLoadPhase::DependencyResolved);
        let command = phase.command.expect("command facts");
        assert_eq!(
            command.executable_path,
            PathBuf::from("/opt/pantograph/llama-server")
        );
        assert_eq!(command.working_directory, PathBuf::from("/opt/pantograph"));
        assert_eq!(command.args, vec!["--port".to_string(), "8080".to_string()]);
        assert_eq!(
            command.env_overrides,
            vec![("LD_LIBRARY_PATH".to_string(), "/opt/lib".to_string())]
        );
        assert_eq!(command.pid_file, Some(PathBuf::from("/tmp/llama.pid")));
    }

    fn snapshot_with_readiness(
        readiness_state: ManagedRuntimeReadinessState,
        available: bool,
    ) -> ManagedRuntimeSnapshot {
        ManagedRuntimeSnapshot {
            id: ManagedBinaryId::LlamaCpp,
            display_name: "llama.cpp".to_string(),
            install_state: if available {
                ManagedBinaryInstallState::Installed
            } else {
                ManagedBinaryInstallState::Missing
            },
            readiness_state,
            available,
            can_install: !available,
            can_remove: available,
            missing_files: if available {
                Vec::new()
            } else {
                vec!["llama-server".to_string()]
            },
            unavailable_reason: (!available).then(|| "llama.cpp executable is missing".to_string()),
            versions: vec![ManagedRuntimeVersionStatus {
                version: Some("b5920".to_string()),
                display_label: "b5920".to_string(),
                runtime_key: "llama_cpp".to_string(),
                platform_key: "linux-x86_64".to_string(),
                install_root: Some("/opt/pantograph/llama-cpp/b5920".to_string()),
                executable_name: "llama-server".to_string(),
                executable_ready: available,
                install_state: if available {
                    ManagedBinaryInstallState::Installed
                } else {
                    ManagedBinaryInstallState::Missing
                },
                readiness_state,
                catalog_available: true,
                installable: true,
                selected: true,
                active: available,
            }],
            selection: ManagedRuntimeSelectionState {
                selected_version: Some("b5920".to_string()),
                active_version: available.then(|| "b5920".to_string()),
                default_version: Some("b5920".to_string()),
            },
            active_job: None,
            job_artifact: None,
        }
    }
}
