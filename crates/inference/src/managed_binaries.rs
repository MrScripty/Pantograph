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
) -> Result<ResolvedCommand, Box<ManagedBinaryFacadeError>> {
    let snapshot = managed_runtime_snapshot(app_data_dir, id)
        .map_err(ManagedBinaryFacadeError::RuntimeStatus)?;

    if !snapshot.available || snapshot.readiness_state != ManagedRuntimeReadinessState::Ready {
        let install_root = selected_install_root(&snapshot);
        return Err(Box::new(ManagedBinaryFacadeError::RuntimeNotReady {
            key: ManagedBinaryKey::runtime(snapshot.id),
            display_name: snapshot.display_name,
            readiness_state: snapshot.readiness_state,
            selected_version: snapshot.selection.selected_version,
            install_root,
            missing_files: snapshot.missing_files,
            unavailable_reason: snapshot.unavailable_reason,
        }));
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
            Box::new(ManagedBinaryFacadeError::RuntimeCommandResolution {
                key: ManagedBinaryKey::runtime(snapshot.id),
                display_name: snapshot.display_name,
                selected_version: snapshot.selection.selected_version,
                install_root,
                source,
            })
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

        let error: Box<ManagedBinaryFacadeError> = resolve_managed_binary_command(
            temp.path(),
            ManagedBinaryId::LlamaCpp,
            &["--port", "0"],
        )
        .expect_err("missing llama.cpp should fail before command resolution");

        match *error {
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

    #[test]
    fn boxed_facade_errors_preserve_variants_and_exact_display() {
        let not_ready = Box::new(ManagedBinaryFacadeError::RuntimeNotReady {
            key: ManagedBinaryKey::runtime(ManagedBinaryId::LlamaCpp),
            display_name: "llama.cpp".to_string(),
            readiness_state: ManagedRuntimeReadinessState::Missing,
            selected_version: Some("v1".to_string()),
            install_root: Some("/runtime".to_string()),
            missing_files: vec!["server".to_string(), "library".to_string()],
            unavailable_reason: Some("not installed".to_string()),
        });
        assert_eq!(not_ready.to_string(), "llama.cpp is not ready for launch (Missing, selected version v1, install root /runtime, missing server, library: not installed)");
        assert!(matches!(
            *not_ready,
            ManagedBinaryFacadeError::RuntimeNotReady { .. }
        ));
        let command = Box::new(ManagedBinaryFacadeError::RuntimeCommandResolution {
            key: ManagedBinaryKey::runtime(ManagedBinaryId::LlamaCpp),
            display_name: "llama.cpp".to_string(),
            selected_version: Some("v1".to_string()),
            install_root: Some("/runtime".to_string()),
            source: "denied".to_string(),
        });
        assert_eq!(command.to_string(), "failed to resolve llama.cpp launch command for selected version v1 at /runtime: denied");
        assert!(matches!(
            *command,
            ManagedBinaryFacadeError::RuntimeCommandResolution { .. }
        ));
        let status = Box::new(ManagedBinaryFacadeError::RuntimeStatus(
            "unreadable".to_string(),
        ));
        assert_eq!(
            status.to_string(),
            "failed to read managed runtime status: unreadable"
        );
    }

    #[test]
    fn successful_command_projection_preserves_launch_fields_and_argument_boundaries() {
        let command = resolved_command_from_dependency_command(ResolvedManagedDependencyCommand {
            key: ManagedDependencyKey::RuntimeSidecar(RuntimeSidecarDependencyId::LlamaCpp),
            executable_path: "/runtime/server".to_string(),
            working_directory: "/runtime".to_string(),
            args: vec!["--model".to_string(), "models/a b.gguf".to_string()],
            env_overrides: vec![("RUNTIME_MODE".to_string(), "test".to_string())],
            pid_file: Some("/runtime/server.pid".to_string()),
        });
        assert_eq!(command.executable_path, PathBuf::from("/runtime/server"));
        assert_eq!(command.working_directory, PathBuf::from("/runtime"));
        assert_eq!(
            command.args,
            vec![OsString::from("--model"), OsString::from("models/a b.gguf")]
        );
        assert_eq!(
            command.env_overrides,
            vec![(OsString::from("RUNTIME_MODE"), OsString::from("test"))]
        );
        assert_eq!(command.pid_file, Some(PathBuf::from("/runtime/server.pid")));
    }
}
