use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    list_managed_redistributable_statuses, list_managed_runtime_snapshots, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRedistributableCategory, ManagedRedistributableId,
    ManagedRedistributableInstallState, ManagedRedistributableReadiness, ManagedRuntimeJobStatus,
    ManagedRuntimeReadinessState, ResolvedCommand,
};
use pantograph_managed_dependencies::{
    ManagedDependencyKey, ResolvedManagedDependencyCommand, RuntimeSidecarDependencyId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedBinaryCategory {
    RuntimeSidecar,
    MediaTool,
    NativeArtifact,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedBinaryKey(String);

impl ManagedBinaryKey {
    pub fn runtime(id: ManagedBinaryId) -> Self {
        Self(format!("runtime:{}", id.key()))
    }

    pub fn redistributable(id: ManagedRedistributableId) -> Self {
        Self(format!("redistributable:{}", id.key()))
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedBinarySource {
    pub owner: Option<String>,
    pub project: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedBinaryVersionStatus {
    pub version: Option<String>,
    pub display_label: String,
    pub platform_key: String,
    pub install_root: Option<String>,
    pub expected_files: Vec<String>,
    pub missing_files: Vec<String>,
    pub install_state: ManagedBinaryInstallState,
    pub readiness_state: ManagedRuntimeReadinessState,
    pub selected: bool,
    pub active: bool,
    pub source: Option<ManagedBinarySource>,
    pub checksum_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedBinaryActionSupport {
    ResolvedCommand,
    Activation,
    StatusOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedBinaryStatus {
    pub key: ManagedBinaryKey,
    pub display_name: String,
    pub category: ManagedBinaryCategory,
    pub install_state: ManagedBinaryInstallState,
    pub readiness_state: ManagedRuntimeReadinessState,
    pub available: bool,
    pub can_install: bool,
    pub can_remove: bool,
    pub missing_files: Vec<String>,
    pub unavailable_reason: Option<String>,
    pub active_job: Option<ManagedRuntimeJobStatus>,
    pub selected_version: Option<String>,
    pub active_version: Option<String>,
    pub default_version: Option<String>,
    pub action_support: ManagedBinaryActionSupport,
    pub versions: Vec<ManagedBinaryVersionStatus>,
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

pub fn list_managed_binary_statuses(
    app_data_dir: &Path,
) -> Result<Vec<ManagedBinaryStatus>, ManagedBinaryFacadeError> {
    let mut statuses = Vec::new();
    statuses.extend(
        list_managed_runtime_snapshots(app_data_dir)
            .map_err(ManagedBinaryFacadeError::RuntimeStatus)?
            .into_iter()
            .map(runtime_status),
    );
    statuses.extend(
        list_managed_redistributable_statuses(app_data_dir)
            .into_iter()
            .map(redistributable_status),
    );
    statuses.sort_by(|left, right| left.key.as_str().cmp(right.key.as_str()));
    Ok(statuses)
}

pub fn resolve_managed_binary_command(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    args: &[&str],
) -> Result<ResolvedCommand, ManagedBinaryFacadeError> {
    let status = list_managed_binary_statuses(app_data_dir)?
        .into_iter()
        .find(|status| status.key == ManagedBinaryKey::runtime(id))
        .ok_or_else(|| {
            ManagedBinaryFacadeError::RuntimeStatus(format!(
                "managed runtime '{}' was not found",
                id.key()
            ))
        })?;

    if !status.available || status.readiness_state != ManagedRuntimeReadinessState::Ready {
        let install_root = selected_install_root(&status);
        return Err(ManagedBinaryFacadeError::RuntimeNotReady {
            key: status.key,
            display_name: status.display_name,
            readiness_state: status.readiness_state,
            selected_version: status.selected_version,
            install_root,
            missing_files: status.missing_files,
            unavailable_reason: status.unavailable_reason,
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
            let install_root = selected_install_root(&status);
            ManagedBinaryFacadeError::RuntimeCommandResolution {
                key: status.key,
                display_name: status.display_name,
                selected_version: status.selected_version,
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
        ManagedBinaryId::Ollama => None,
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

fn runtime_status(snapshot: crate::ManagedRuntimeSnapshot) -> ManagedBinaryStatus {
    ManagedBinaryStatus {
        key: ManagedBinaryKey::runtime(snapshot.id),
        display_name: snapshot.display_name,
        category: ManagedBinaryCategory::RuntimeSidecar,
        install_state: snapshot.install_state,
        readiness_state: snapshot.readiness_state,
        available: snapshot.available,
        can_install: snapshot.can_install,
        can_remove: snapshot.can_remove,
        missing_files: snapshot.missing_files,
        unavailable_reason: snapshot.unavailable_reason,
        active_job: snapshot.active_job,
        selected_version: snapshot.selection.selected_version,
        active_version: snapshot.selection.active_version,
        default_version: snapshot.selection.default_version,
        action_support: ManagedBinaryActionSupport::ResolvedCommand,
        versions: snapshot
            .versions
            .into_iter()
            .map(|version| ManagedBinaryVersionStatus {
                version: version.version,
                display_label: version.display_label,
                platform_key: version.platform_key,
                install_root: version.install_root,
                expected_files: vec![version.executable_name],
                missing_files: Vec::new(),
                install_state: version.install_state,
                readiness_state: version.readiness_state,
                selected: version.selected,
                active: version.active,
                source: None,
                checksum_sha256: None,
            })
            .collect(),
    }
}

fn redistributable_status(status: crate::ManagedRedistributableStatus) -> ManagedBinaryStatus {
    let unavailable_reason = redistributable_unavailable_reason(&status);
    let action_support = match status.category {
        ManagedRedistributableCategory::ToolBinary => ManagedBinaryActionSupport::ResolvedCommand,
        ManagedRedistributableCategory::NativeLibraryArtifact => {
            ManagedBinaryActionSupport::Activation
        }
    };
    let category = match status.category {
        ManagedRedistributableCategory::ToolBinary => ManagedBinaryCategory::MediaTool,
        ManagedRedistributableCategory::NativeLibraryArtifact => {
            ManagedBinaryCategory::NativeArtifact
        }
    };

    ManagedBinaryStatus {
        key: ManagedBinaryKey::redistributable(status.id),
        display_name: status.display_name,
        category,
        install_state: redistributable_install_state(status.install_state),
        readiness_state: redistributable_readiness_state(status.readiness),
        available: status.available,
        can_install: status.catalog.download_url.is_some(),
        can_remove: status.available,
        missing_files: status.missing_files,
        unavailable_reason,
        active_job: None,
        selected_version: status.selection.selected_version,
        active_version: status.selection.active_version,
        default_version: status.selection.default_version,
        action_support,
        versions: status
            .versions
            .into_iter()
            .map(|version| ManagedBinaryVersionStatus {
                version: Some(version.version),
                display_label: status.catalog.version.clone(),
                platform_key: version.platform_key,
                install_root: Some(version.install_root),
                expected_files: version.expected_files,
                missing_files: version.missing_files,
                install_state: redistributable_install_state(version.install_state),
                readiness_state: redistributable_readiness_state(version.readiness),
                selected: version.selected,
                active: version.active,
                source: Some(ManagedBinarySource {
                    owner: Some(status.catalog.source.owner.clone()),
                    project: Some(status.catalog.source.project.clone()),
                }),
                checksum_sha256: status.catalog.checksum_sha256.clone(),
            })
            .collect(),
    }
}

fn redistributable_install_state(
    state: ManagedRedistributableInstallState,
) -> ManagedBinaryInstallState {
    match state {
        ManagedRedistributableInstallState::Installed => ManagedBinaryInstallState::Installed,
        ManagedRedistributableInstallState::Missing => ManagedBinaryInstallState::Missing,
        ManagedRedistributableInstallState::Unsupported => ManagedBinaryInstallState::Unsupported,
    }
}

fn redistributable_readiness_state(
    state: ManagedRedistributableReadiness,
) -> ManagedRuntimeReadinessState {
    match state {
        ManagedRedistributableReadiness::Missing => ManagedRuntimeReadinessState::Missing,
        ManagedRedistributableReadiness::Ready => ManagedRuntimeReadinessState::Ready,
        ManagedRedistributableReadiness::Unsupported => ManagedRuntimeReadinessState::Unsupported,
    }
}

fn redistributable_unavailable_reason(
    status: &crate::ManagedRedistributableStatus,
) -> Option<String> {
    if status.available {
        return None;
    }
    if status.readiness == ManagedRedistributableReadiness::Unsupported {
        return Some(format!(
            "{} is unsupported on {}",
            status.display_name, status.catalog.platform_key
        ));
    }
    if !status.missing_files.is_empty() {
        return Some(format!(
            "{} is missing expected file(s): {}",
            status.display_name,
            status.missing_files.join(", ")
        ));
    }
    Some(format!("{} is not ready", status.display_name))
}

fn selected_install_root(status: &ManagedBinaryStatus) -> Option<String> {
    status
        .versions
        .iter()
        .find(|version| version.selected || version.active)
        .and_then(|version| version.install_root.clone())
        .or_else(|| {
            status
                .versions
                .iter()
                .find_map(|version| version.install_root.clone())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ManagedRedistributableId;

    #[test]
    fn list_managed_binary_statuses_includes_all_categories() {
        let temp = tempfile::tempdir().expect("temp dir");

        let statuses = list_managed_binary_statuses(temp.path()).expect("managed binary statuses");

        assert!(statuses.iter().any(|status| status.key
            == ManagedBinaryKey::runtime(ManagedBinaryId::LlamaCpp)
            && status.category == ManagedBinaryCategory::RuntimeSidecar));
        assert!(!statuses
            .iter()
            .any(|status| status.key == ManagedBinaryKey::runtime(ManagedBinaryId::Ollama)));
        assert!(statuses.iter().any(|status| status.key
            == ManagedBinaryKey::redistributable(ManagedRedistributableId::Ffmpeg)
            && status.category == ManagedBinaryCategory::MediaTool));
        assert!(statuses.iter().any(|status| status.key
            == ManagedBinaryKey::redistributable(ManagedRedistributableId::OpenColorIo)
            && status.category == ManagedBinaryCategory::NativeArtifact));
    }

    #[test]
    fn redistributable_status_preserves_source_and_missing_files() {
        let temp = tempfile::tempdir().expect("temp dir");

        let ffmpeg = list_managed_binary_statuses(temp.path())
            .expect("managed binary statuses")
            .into_iter()
            .find(|status| {
                status.key == ManagedBinaryKey::redistributable(ManagedRedistributableId::Ffmpeg)
            })
            .expect("ffmpeg status");

        assert_eq!(ffmpeg.category, ManagedBinaryCategory::MediaTool);
        assert_eq!(ffmpeg.install_state, ManagedBinaryInstallState::Missing);
        assert_eq!(
            ffmpeg.readiness_state,
            ManagedRuntimeReadinessState::Missing
        );
        assert!(!ffmpeg.missing_files.is_empty());
        assert_eq!(
            ffmpeg.action_support,
            ManagedBinaryActionSupport::ResolvedCommand
        );
        assert!(ffmpeg
            .versions
            .iter()
            .any(|version| version.source.is_some()));
    }

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
