use super::super::contracts::{
    ManagedBinaryCapability, ManagedBinaryInstallState, ManagedRuntimeCatalogVersion,
    ManagedRuntimeJobArtifactStatus, ManagedRuntimeJobState, ManagedRuntimeReadinessState,
    ManagedRuntimeSelectionState, ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus,
};
use super::super::definitions::{ManagedBinaryDefinition, definition};
use super::super::paths::managed_install_dir;
use super::super::state::{ManagedRuntimePersistedRuntime, ManagedRuntimePersistedVersion};
use std::path::Path;

pub(super) fn snapshot_from_capability(
    app_data_dir: &Path,
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
) -> ManagedRuntimeSnapshot {
    let definition = definition(capability.id);
    let selection = persisted_runtime
        .map(|runtime| runtime.selection.clone())
        .unwrap_or_default();
    let active_job = persisted_runtime.and_then(|runtime| runtime.active_job.clone());
    let job_artifact = persisted_runtime.and_then(projected_job_artifact_status);
    let versions = projected_version_statuses(
        app_data_dir,
        capability,
        persisted_runtime,
        &selection,
        definition,
    );
    let readiness_state = projected_snapshot_readiness_state(capability, persisted_runtime);
    let available = projected_snapshot_available(capability, readiness_state);
    let unavailable_reason = projected_snapshot_unavailable_reason(capability, persisted_runtime);

    ManagedRuntimeSnapshot {
        id: capability.id,
        display_name: capability.display_name.clone(),
        install_state: capability.install_state,
        readiness_state,
        available,
        can_install: capability.can_install,
        can_remove: capability.can_remove,
        missing_files: capability.missing_files.clone(),
        unavailable_reason,
        versions,
        selection,
        active_job,
        job_artifact,
    }
}

fn projected_job_artifact_status(
    runtime: &ManagedRuntimePersistedRuntime,
) -> Option<ManagedRuntimeJobArtifactStatus> {
    let artifact = runtime.active_job_artifact.as_ref()?;
    Some(ManagedRuntimeJobArtifactStatus {
        version: artifact.version.clone(),
        archive_name: artifact.archive_name.clone(),
        downloaded_bytes: artifact.downloaded_bytes,
        total_bytes: artifact.total_bytes,
        retained: Path::new(&artifact.archive_path).exists(),
    })
}

fn projected_snapshot_readiness_state(
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
) -> ManagedRuntimeReadinessState {
    let Some(runtime) = persisted_runtime else {
        return readiness_state_for_capability(capability);
    };

    if let Some(selected_version) = runtime.selection.selected_version.as_deref() {
        if let Some(version) = runtime
            .versions
            .iter()
            .find(|version| version.version == selected_version)
        {
            return version.readiness_state;
        }

        return match capability.install_state {
            ManagedBinaryInstallState::Unsupported => ManagedRuntimeReadinessState::Unsupported,
            _ => ManagedRuntimeReadinessState::Missing,
        };
    }

    runtime
        .active_job
        .as_ref()
        .map(|job| job_readiness_state(job.state))
        .unwrap_or_else(|| readiness_state_for_capability(capability))
}

fn projected_snapshot_available(
    capability: &ManagedBinaryCapability,
    readiness_state: ManagedRuntimeReadinessState,
) -> bool {
    capability.available && readiness_state == ManagedRuntimeReadinessState::Ready
}

fn projected_snapshot_unavailable_reason(
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
) -> Option<String> {
    if let Some(reason) = capability
        .unavailable_reason
        .as_ref()
        .filter(|reason| !reason.trim().is_empty())
    {
        return Some(reason.clone());
    }

    let runtime = persisted_runtime?;
    let selected_version = runtime.selection.selected_version.as_deref()?;
    runtime
        .versions
        .iter()
        .find(|version| version.version == selected_version)
        .and_then(|version| version.last_error.clone())
        .filter(|reason| !reason.trim().is_empty())
}

fn job_readiness_state(state: ManagedRuntimeJobState) -> ManagedRuntimeReadinessState {
    match state {
        ManagedRuntimeJobState::Queued | ManagedRuntimeJobState::Downloading => {
            ManagedRuntimeReadinessState::Downloading
        }
        ManagedRuntimeJobState::Extracting => ManagedRuntimeReadinessState::Extracting,
        ManagedRuntimeJobState::Validating => ManagedRuntimeReadinessState::Validating,
        ManagedRuntimeJobState::Ready => ManagedRuntimeReadinessState::Ready,
        ManagedRuntimeJobState::Failed
        | ManagedRuntimeJobState::Paused
        | ManagedRuntimeJobState::Cancelled => ManagedRuntimeReadinessState::Failed,
    }
}

pub(super) fn readiness_state_for_capability(
    capability: &ManagedBinaryCapability,
) -> ManagedRuntimeReadinessState {
    match capability.install_state {
        ManagedBinaryInstallState::Installed | ManagedBinaryInstallState::SystemProvided => {
            ManagedRuntimeReadinessState::Ready
        }
        ManagedBinaryInstallState::Missing => ManagedRuntimeReadinessState::Missing,
        ManagedBinaryInstallState::Unsupported => ManagedRuntimeReadinessState::Unsupported,
    }
}

fn projected_version_statuses(
    app_data_dir: &Path,
    capability: &ManagedBinaryCapability,
    persisted_runtime: Option<&ManagedRuntimePersistedRuntime>,
    selection: &ManagedRuntimeSelectionState,
    definition: &'static dyn ManagedBinaryDefinition,
) -> Vec<ManagedRuntimeVersionStatus> {
    let Some(runtime) = persisted_runtime else {
        return vec![version_status_for_capability(app_data_dir, capability)];
    };

    let mut projected_versions = Vec::new();

    for catalog_version in &runtime.catalog_versions {
        projected_versions.push(projected_catalog_version_status(
            capability,
            runtime,
            selection,
            definition,
            catalog_version,
        ));
    }

    for installed_version in &runtime.versions {
        if runtime
            .catalog_versions
            .iter()
            .any(|catalog_version| catalog_version.version == installed_version.version)
        {
            continue;
        }

        projected_versions.push(projected_installed_version_status(
            capability,
            selection,
            definition,
            installed_version,
        ));
    }

    if projected_versions.is_empty() {
        projected_versions.push(version_status_for_capability(app_data_dir, capability));
    }

    projected_versions
}

fn projected_catalog_version_status(
    capability: &ManagedBinaryCapability,
    runtime: &ManagedRuntimePersistedRuntime,
    selection: &ManagedRuntimeSelectionState,
    definition: &'static dyn ManagedBinaryDefinition,
    catalog_version: &ManagedRuntimeCatalogVersion,
) -> ManagedRuntimeVersionStatus {
    let installed_version = runtime
        .versions
        .iter()
        .find(|version| version.version == catalog_version.version);

    if let Some(installed_version) = installed_version {
        return projected_installed_version_status(
            capability,
            selection,
            definition,
            installed_version,
        );
    }

    let version = catalog_version.version.as_str();
    ManagedRuntimeVersionStatus {
        version: Some(catalog_version.version.clone()),
        display_label: catalog_version.display_label.clone(),
        runtime_key: catalog_version.runtime_key.clone(),
        platform_key: catalog_version.platform_key.clone(),
        install_root: None,
        executable_name: definition.executable_name().to_string(),
        executable_ready: false,
        install_state: ManagedBinaryInstallState::Missing,
        readiness_state: ManagedRuntimeReadinessState::Missing,
        catalog_available: true,
        installable: capability.can_install,
        selected: selection.selected_version.as_deref() == Some(version),
        active: selection.active_version.as_deref() == Some(version),
    }
}

fn projected_installed_version_status(
    capability: &ManagedBinaryCapability,
    selection: &ManagedRuntimeSelectionState,
    definition: &'static dyn ManagedBinaryDefinition,
    version: &ManagedRuntimePersistedVersion,
) -> ManagedRuntimeVersionStatus {
    ManagedRuntimeVersionStatus {
        version: Some(version.version.clone()),
        display_label: version.version.clone(),
        runtime_key: version
            .runtime_key
            .clone()
            .unwrap_or_else(|| capability.id.key().to_string()),
        platform_key: version
            .platform_key
            .clone()
            .unwrap_or_else(|| definition.platform_key().to_string()),
        install_root: version.install_root.clone(),
        executable_name: definition.executable_name().to_string(),
        executable_ready: version
            .install_root
            .as_deref()
            .map(|install_root| {
                definition
                    .validate_installation(Path::new(install_root))
                    .is_empty()
            })
            .unwrap_or(false),
        install_state: if version.install_root.is_some() {
            ManagedBinaryInstallState::Installed
        } else {
            ManagedBinaryInstallState::Missing
        },
        readiness_state: version.readiness_state,
        catalog_available: true,
        installable: capability.can_install,
        selected: selection.selected_version.as_deref() == Some(version.version.as_str()),
        active: selection.active_version.as_deref() == Some(version.version.as_str()),
    }
}

fn version_status_for_capability(
    app_data_dir: &Path,
    capability: &ManagedBinaryCapability,
) -> ManagedRuntimeVersionStatus {
    let definition = definition(capability.id);
    let install_root = managed_install_dir(app_data_dir, capability.id);
    ManagedRuntimeVersionStatus {
        version: None,
        display_label: match capability.install_state {
            ManagedBinaryInstallState::SystemProvided => "System provided".to_string(),
            ManagedBinaryInstallState::Installed => "Managed install".to_string(),
            ManagedBinaryInstallState::Missing => "No managed install".to_string(),
            ManagedBinaryInstallState::Unsupported => "Unsupported platform".to_string(),
        },
        runtime_key: capability.id.key().to_string(),
        platform_key: definition.platform_key().to_string(),
        install_root: install_root
            .exists()
            .then(|| install_root.display().to_string()),
        executable_name: definition.executable_name().to_string(),
        executable_ready: capability.available,
        install_state: capability.install_state,
        readiness_state: readiness_state_for_capability(capability),
        catalog_available: false,
        installable: false,
        selected: false,
        active: capability.available,
    }
}
