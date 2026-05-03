use std::path::Path;

use pantograph_managed_dependencies::{
    ManagedDependencyCategory, ManagedDependencyInstallState, ManagedDependencyKey,
    ManagedDependencyReadinessState, ManagedDependencySelectionState, ManagedDependencyStatus,
    ManagedDependencyVersionStatus, MediaToolDependencyId, NativeArtifactDependencyId,
};

use super::{
    list_managed_redistributable_statuses, managed_redistributable_status,
    ManagedRedistributableCategory, ManagedRedistributableId, ManagedRedistributableInstallState,
    ManagedRedistributableReadiness, ManagedRedistributableStatus,
    ManagedRedistributableVersionStatus,
};

pub fn list_managed_dependency_statuses(app_data_dir: &Path) -> Vec<ManagedDependencyStatus> {
    list_managed_redistributable_statuses(app_data_dir)
        .into_iter()
        .map(managed_dependency_status_from_redistributable)
        .collect()
}

pub fn managed_dependency_status(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
) -> ManagedDependencyStatus {
    managed_dependency_status_from_redistributable(managed_redistributable_status(app_data_dir, id))
}

fn managed_dependency_status_from_redistributable(
    status: ManagedRedistributableStatus,
) -> ManagedDependencyStatus {
    ManagedDependencyStatus {
        key: managed_dependency_key(status.id),
        display_name: status.display_name,
        category: managed_dependency_category(status.category),
        install_state: managed_dependency_install_state(status.install_state),
        readiness_state: managed_dependency_readiness_state(status.readiness),
        available: status.available,
        missing_files: status.missing_files,
        selection: ManagedDependencySelectionState {
            selected_version: status.selection.selected_version,
            active_version: status.selection.active_version,
            default_version: status.selection.default_version,
        },
        versions: status
            .versions
            .into_iter()
            .map(managed_dependency_version_status)
            .collect(),
        unavailable_reason: None,
    }
}

fn managed_dependency_version_status(
    status: ManagedRedistributableVersionStatus,
) -> ManagedDependencyVersionStatus {
    ManagedDependencyVersionStatus {
        version: Some(status.version),
        platform_key: status.platform_key,
        install_root: Some(status.install_root),
        expected_files: status.expected_files,
        missing_files: status.missing_files,
        install_state: managed_dependency_install_state(status.install_state),
        readiness_state: managed_dependency_readiness_state(status.readiness),
        selected: status.selected,
        active: status.active,
    }
}

fn managed_dependency_key(id: ManagedRedistributableId) -> ManagedDependencyKey {
    match id {
        ManagedRedistributableId::Ffmpeg => {
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ffmpeg)
        }
        ManagedRedistributableId::Ocioconvert => {
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ocioconvert)
        }
        ManagedRedistributableId::Oiiotool => {
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Oiiotool)
        }
        ManagedRedistributableId::OpenColorIo => {
            ManagedDependencyKey::NativeArtifact(NativeArtifactDependencyId::OpenColorIo)
        }
    }
}

fn managed_dependency_category(
    category: ManagedRedistributableCategory,
) -> ManagedDependencyCategory {
    match category {
        ManagedRedistributableCategory::ToolBinary => ManagedDependencyCategory::MediaTool,
        ManagedRedistributableCategory::NativeLibraryArtifact => {
            ManagedDependencyCategory::NativeArtifact
        }
    }
}

fn managed_dependency_install_state(
    state: ManagedRedistributableInstallState,
) -> ManagedDependencyInstallState {
    match state {
        ManagedRedistributableInstallState::Installed => ManagedDependencyInstallState::Installed,
        ManagedRedistributableInstallState::Missing => ManagedDependencyInstallState::Missing,
        ManagedRedistributableInstallState::Unsupported => {
            ManagedDependencyInstallState::Unsupported
        }
    }
}

fn managed_dependency_readiness_state(
    state: ManagedRedistributableReadiness,
) -> ManagedDependencyReadinessState {
    match state {
        ManagedRedistributableReadiness::Missing => ManagedDependencyReadinessState::Missing,
        ManagedRedistributableReadiness::Ready => ManagedDependencyReadinessState::Ready,
        ManagedRedistributableReadiness::Unsupported => {
            ManagedDependencyReadinessState::Unsupported
        }
    }
}
