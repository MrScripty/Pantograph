use std::ffi::OsString;
use std::path::Path;

use pantograph_managed_dependencies::{
    ManagedDependencyCategory, ManagedDependencyInstallState, ManagedDependencyKey,
    ManagedDependencyReadinessState, ManagedDependencySelectionState, ManagedDependencyStatus,
    ManagedDependencyVersionStatus, ResolvedManagedDependencyCommand, RuntimeSidecarDependencyId,
};

use super::{
    list_managed_runtime_snapshots, managed_runtime_snapshot, resolve_binary_command,
    ManagedBinaryId, ManagedBinaryInstallState, ManagedRuntimeReadinessState,
    ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus, ResolvedCommand,
};

pub fn list_managed_runtime_dependency_statuses(
    app_data_dir: &Path,
) -> Result<Vec<ManagedDependencyStatus>, String> {
    list_managed_runtime_snapshots(app_data_dir).map(|snapshots| {
        snapshots
            .into_iter()
            .filter_map(managed_dependency_status_from_runtime_snapshot)
            .collect()
    })
}

pub fn managed_runtime_dependency_status(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<ManagedDependencyStatus, String> {
    let snapshot = managed_runtime_snapshot(app_data_dir, id)?;
    managed_dependency_status_from_runtime_snapshot(snapshot).ok_or_else(|| {
        format!(
            "{} is not a neutral managed runtime sidecar",
            id.display_name()
        )
    })
}

pub fn resolve_runtime_sidecar_dependency_command(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    args: &[&str],
) -> Result<ResolvedManagedDependencyCommand, String> {
    let key = managed_dependency_key(id).ok_or_else(|| {
        format!(
            "{} is not a neutral managed runtime sidecar",
            id.display_name()
        )
    })?;
    let command = resolve_binary_command(app_data_dir, id, args)?;
    Ok(resolved_managed_dependency_command(key, command))
}

fn managed_dependency_status_from_runtime_snapshot(
    snapshot: ManagedRuntimeSnapshot,
) -> Option<ManagedDependencyStatus> {
    let key = managed_dependency_key(snapshot.id)?;
    Some(ManagedDependencyStatus {
        key,
        display_name: snapshot.display_name,
        category: ManagedDependencyCategory::RuntimeSidecar,
        install_state: managed_dependency_install_state(snapshot.install_state),
        readiness_state: managed_dependency_readiness_state(snapshot.readiness_state),
        available: snapshot.available,
        missing_files: snapshot.missing_files,
        selection: ManagedDependencySelectionState {
            selected_version: snapshot.selection.selected_version,
            active_version: snapshot.selection.active_version,
            default_version: snapshot.selection.default_version,
        },
        versions: snapshot
            .versions
            .into_iter()
            .map(managed_dependency_version_status)
            .collect(),
        unavailable_reason: snapshot.unavailable_reason,
    })
}

fn managed_dependency_version_status(
    status: ManagedRuntimeVersionStatus,
) -> ManagedDependencyVersionStatus {
    let missing_files = if status.executable_ready {
        Vec::new()
    } else {
        vec![status.executable_name.clone()]
    };

    ManagedDependencyVersionStatus {
        version: status.version,
        platform_key: status.platform_key,
        install_root: status.install_root,
        expected_files: vec![status.executable_name],
        missing_files,
        install_state: managed_dependency_install_state(status.install_state),
        readiness_state: managed_dependency_readiness_state(status.readiness_state),
        selected: status.selected,
        active: status.active,
    }
}

fn managed_dependency_key(id: ManagedBinaryId) -> Option<ManagedDependencyKey> {
    match id {
        ManagedBinaryId::LlamaCpp => Some(ManagedDependencyKey::RuntimeSidecar(
            RuntimeSidecarDependencyId::LlamaCpp,
        )),
    }
}

fn managed_dependency_install_state(
    state: ManagedBinaryInstallState,
) -> ManagedDependencyInstallState {
    match state {
        ManagedBinaryInstallState::Installed => ManagedDependencyInstallState::Installed,
        ManagedBinaryInstallState::SystemProvided => ManagedDependencyInstallState::SystemProvided,
        ManagedBinaryInstallState::Missing => ManagedDependencyInstallState::Missing,
        ManagedBinaryInstallState::Unsupported => ManagedDependencyInstallState::Unsupported,
    }
}

fn managed_dependency_readiness_state(
    state: ManagedRuntimeReadinessState,
) -> ManagedDependencyReadinessState {
    match state {
        ManagedRuntimeReadinessState::Unknown => ManagedDependencyReadinessState::Unknown,
        ManagedRuntimeReadinessState::Missing => ManagedDependencyReadinessState::Missing,
        ManagedRuntimeReadinessState::Downloading => ManagedDependencyReadinessState::Downloading,
        ManagedRuntimeReadinessState::Extracting => ManagedDependencyReadinessState::Extracting,
        ManagedRuntimeReadinessState::Validating => ManagedDependencyReadinessState::Validating,
        ManagedRuntimeReadinessState::Ready => ManagedDependencyReadinessState::Ready,
        ManagedRuntimeReadinessState::Failed => ManagedDependencyReadinessState::Failed,
        ManagedRuntimeReadinessState::Unsupported => ManagedDependencyReadinessState::Unsupported,
    }
}

fn resolved_managed_dependency_command(
    key: ManagedDependencyKey,
    command: ResolvedCommand,
) -> ResolvedManagedDependencyCommand {
    ResolvedManagedDependencyCommand {
        key,
        executable_path: command.executable_path.display().to_string(),
        working_directory: command.working_directory.display().to_string(),
        args: os_strings_to_strings(command.args),
        env_overrides: command
            .env_overrides
            .into_iter()
            .map(|(key, value)| (os_string_to_string(key), os_string_to_string(value)))
            .collect(),
        pid_file: command.pid_file.map(|path| path.display().to_string()),
    }
}

fn os_strings_to_strings(values: Vec<OsString>) -> Vec<String> {
    values.into_iter().map(os_string_to_string).collect()
}

fn os_string_to_string(value: OsString) -> String {
    value.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::super::definitions::definition;
    use super::*;
    use crate::managed_runtime::{
        save_managed_runtime_state, ManagedRuntimePersistedRuntime, ManagedRuntimePersistedState,
        ManagedRuntimePersistedVersion, ManagedRuntimeSelectionState,
    };

    #[test]
    fn runtime_dependency_status_projects_missing_llama_cpp_snapshot() {
        let temp_dir = tempfile::tempdir().expect("temp dir");

        let status = managed_runtime_dependency_status(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect("runtime dependency status");

        assert_eq!(
            status.key,
            ManagedDependencyKey::RuntimeSidecar(RuntimeSidecarDependencyId::LlamaCpp)
        );
        assert_eq!(status.category, ManagedDependencyCategory::RuntimeSidecar);
        assert_eq!(status.install_state, ManagedDependencyInstallState::Missing);
        assert_eq!(
            status.readiness_state,
            ManagedDependencyReadinessState::Missing
        );
        assert!(!status.available);
    }

    #[test]
    fn runtime_sidecar_command_projection_preserves_resolved_command_facts() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");
        install_fake_runtime_files(&install_dir, ManagedBinaryId::LlamaCpp);
        save_ready_runtime_state(temp_dir.path(), &install_dir);

        let command = resolve_runtime_sidecar_dependency_command(
            temp_dir.path(),
            ManagedBinaryId::LlamaCpp,
            &["--port", "8080", "--pid-file", "server.pid"],
        )
        .expect("runtime sidecar command");

        assert_eq!(
            command.key,
            ManagedDependencyKey::RuntimeSidecar(RuntimeSidecarDependencyId::LlamaCpp)
        );
        assert!(command.executable_path.contains("llama-server"));
        assert_eq!(command.working_directory, install_dir.display().to_string());
        assert_eq!(command.args, vec!["--port".to_string(), "8080".to_string()]);
        assert_eq!(command.pid_file.as_deref(), Some("server.pid"));
        assert!(!command.env_overrides.is_empty());
    }

    fn install_fake_runtime_files(dir: &Path, id: ManagedBinaryId) {
        std::fs::create_dir_all(dir).expect("create runtime dir");
        for file_name in definition(id).validate_installation(dir) {
            std::fs::write(dir.join(&file_name), [])
                .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
        }
    }

    fn save_ready_runtime_state(app_data_dir: &Path, install_dir: &Path) {
        let state = ManagedRuntimePersistedState {
            schema_version: 1,
            runtimes: vec![ManagedRuntimePersistedRuntime {
                id: ManagedBinaryId::LlamaCpp,
                catalog_versions: Vec::new(),
                catalog_refreshed_at_ms: None,
                versions: vec![ManagedRuntimePersistedVersion {
                    version: "b8248".to_string(),
                    runtime_key: Some(ManagedBinaryId::LlamaCpp.key().to_string()),
                    platform_key: Some(
                        definition(ManagedBinaryId::LlamaCpp)
                            .platform_key()
                            .to_string(),
                    ),
                    readiness_state: ManagedRuntimeReadinessState::Ready,
                    install_root: Some(install_dir.display().to_string()),
                    last_ready_at_ms: Some(1),
                    last_error: None,
                }],
                selection: ManagedRuntimeSelectionState {
                    selected_version: Some("b8248".to_string()),
                    active_version: Some("b8248".to_string()),
                    default_version: None,
                },
                active_job: None,
                active_job_artifact: None,
                install_history: Vec::new(),
            }],
        };
        save_managed_runtime_state(app_data_dir, &state).expect("save runtime state");
    }
}
