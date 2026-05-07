use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    managed_runtime_snapshot, ManagedBinaryId, ManagedRuntimeReadinessState,
    ManagedRuntimeSnapshot, ResolvedCommand,
};
use pantograph_managed_dependencies::{
    ManagedDependencyKey, ResolvedManagedDependencyCommand, RuntimeSidecarDependencyId,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedBinaryKey(String);

impl ManagedBinaryKey {
    pub fn runtime(id: ManagedBinaryId) -> Self {
        Self(format!("runtime:{}", id.key()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ManagedBinaryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ManagedBinaryFacadeError {
    RuntimeStatus(String),
    RuntimeNotReady {
        key: ManagedBinaryKey,
        display_name: String,
        readiness_state: ManagedRuntimeReadinessState,
        selected_version: Option<String>,
        install_root: Option<String>,
        missing_files: Vec<String>,
        unavailable_reason: Option<String>,
    },
    RuntimeCommandResolution {
        key: ManagedBinaryKey,
        display_name: String,
        selected_version: Option<String>,
        install_root: Option<String>,
        source: String,
    },
}

impl fmt::Display for ManagedBinaryFacadeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeStatus(error) => {
                write!(formatter, "failed to read managed runtime status: {error}")
            }
            Self::RuntimeNotReady {
                display_name,
                readiness_state,
                selected_version,
                install_root,
                missing_files,
                unavailable_reason,
                ..
            } => {
                write!(
                    formatter,
                    "{display_name} is not ready for launch ({readiness_state:?}"
                )?;
                if let Some(version) = selected_version.as_deref() {
                    write!(formatter, ", selected version {version}")?;
                }
                if let Some(root) = install_root.as_deref() {
                    write!(formatter, ", install root {root}")?;
                }
                if !missing_files.is_empty() {
                    write!(formatter, ", missing {}", missing_files.join(", "))?;
                }
                if let Some(reason) = unavailable_reason.as_deref() {
                    write!(formatter, ": {reason}")?;
                }
                formatter.write_str(")")
            }
            Self::RuntimeCommandResolution {
                display_name,
                selected_version,
                install_root,
                source,
                ..
            } => {
                write!(formatter, "failed to resolve {display_name} launch command")?;
                if let Some(version) = selected_version.as_deref() {
                    write!(formatter, " for selected version {version}")?;
                }
                if let Some(root) = install_root.as_deref() {
                    write!(formatter, " at {root}")?;
                }
                write!(formatter, ": {source}")
            }
        }
    }
}

impl std::error::Error for ManagedBinaryFacadeError {}

pub fn resolve_managed_binary_command(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    args: &[&str],
) -> Result<ResolvedCommand, ManagedBinaryFacadeError> {
    let snapshot = managed_runtime_snapshot(app_data_dir, id)
        .map_err(ManagedBinaryFacadeError::RuntimeStatus)?;

    if !snapshot.available || snapshot.readiness_state != ManagedRuntimeReadinessState::Ready {
        let install_root = selected_install_root(&snapshot);
        return Err(ManagedBinaryFacadeError::RuntimeNotReady {
            key: ManagedBinaryKey::runtime(snapshot.id),
            display_name: snapshot.display_name,
            readiness_state: snapshot.readiness_state,
            selected_version: snapshot.selection.selected_version,
            install_root,
            missing_files: snapshot.missing_files,
            unavailable_reason: snapshot.unavailable_reason,
        });
    }

    let key = managed_runtime_dependency_key(id).ok_or_else(|| {
        ManagedBinaryFacadeError::RuntimeStatus(format!(
            "managed runtime '{}' does not have a neutral dependency key",
            id.key()
        ))
    })?;

    crate::resolve_managed_dependency_command(app_data_dir, key, args)
        .map(resolved_command_from_dependency_command)
        .map_err(|source| {
            let install_root = selected_install_root(&snapshot);
            ManagedBinaryFacadeError::RuntimeCommandResolution {
                key: ManagedBinaryKey::runtime(snapshot.id),
                display_name: snapshot.display_name,
                selected_version: snapshot.selection.selected_version,
                install_root,
                source,
            }
        })
}

fn managed_runtime_dependency_key(id: ManagedBinaryId) -> Option<ManagedDependencyKey> {
    match id {
        ManagedBinaryId::LlamaCpp => Some(ManagedDependencyKey::RuntimeSidecar(
            RuntimeSidecarDependencyId::LlamaCpp,
        )),
    }
}

fn resolved_command_from_dependency_command(
    command: ResolvedManagedDependencyCommand,
) -> ResolvedCommand {
    ResolvedCommand {
        executable_path: PathBuf::from(command.executable_path),
        working_directory: PathBuf::from(command.working_directory),
        args: command.args.into_iter().map(OsString::from).collect(),
        env_overrides: command
            .env_overrides
            .into_iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect(),
        pid_file: command.pid_file.map(PathBuf::from),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_command_reports_facade_not_ready_context() {
        let temp = tempfile::tempdir().expect("temp dir");

        let error = resolve_managed_binary_command(
            temp.path(),
            ManagedBinaryId::LlamaCpp,
            &["--port", "0"],
        )
        .expect_err("missing llama.cpp should fail before command resolution");

        match error {
            ManagedBinaryFacadeError::RuntimeNotReady {
                key,
                readiness_state,
                missing_files,
                ..
            } => {
                assert_eq!(key, ManagedBinaryKey::runtime(ManagedBinaryId::LlamaCpp));
                assert_eq!(readiness_state, ManagedRuntimeReadinessState::Missing);
                assert!(!missing_files.is_empty());
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
