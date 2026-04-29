use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::managed_redistributables::{
    acquire_managed_redistributable_lease, managed_redistributable_status,
    release_managed_redistributable_lease, ManagedRedistributableId,
    ManagedRedistributableLeaseToken, ManagedRedistributableReadiness,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaConversionJobKind {
    Image,
    Audio,
    Video,
    ThreeD,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaConversionDependencyId {
    Ffmpeg,
    Ocioconvert,
    Oiiotool,
    OpenColorIo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenColorIoActivationValidationState {
    NotValidated,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OpenColorIoActivationValidation {
    pub state: OpenColorIoActivationValidationState,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OpenColorIoActivation {
    pub dependency: MediaConversionDependency,
    pub abi_validation: OpenColorIoActivationValidation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionDependency {
    pub id: MediaConversionDependencyId,
    pub display_name: String,
    pub version: String,
    pub install_root: String,
    pub expected_files: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionDependencyLeaseToken {
    pub id: MediaConversionDependencyId,
    pub version: String,
    pub lease_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionDependencyLease {
    pub dependency: MediaConversionDependency,
    pub token: MediaConversionDependencyLeaseToken,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionDependencyPlanRequest {
    pub job_kind: MediaConversionJobKind,
    pub color_managed: bool,
    pub holder: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionDependencyPlan {
    pub job_kind: MediaConversionJobKind,
    pub color_managed: bool,
    pub leases: Vec<MediaConversionDependencyLease>,
    pub open_color_io_activation: Option<OpenColorIoActivation>,
}

pub fn validate_open_color_io_activation(
    app_data_dir: &Path,
) -> Result<OpenColorIoActivation, String> {
    let dependency = resolve_active_dependency(
        app_data_dir,
        ManagedRedistributableId::OpenColorIo,
        MediaConversionDependencyId::OpenColorIo,
    )?;

    Ok(OpenColorIoActivation {
        dependency,
        abi_validation: OpenColorIoActivationValidation {
            state: OpenColorIoActivationValidationState::NotValidated,
            reason: "OpenColorIO managed artifact expected files are present and active, but ABI validation is not performed in this scaffold because this slice does not dynamically load native libraries".to_string(),
        },
    })
}

pub fn open_color_io_activation_validation_state(
    app_data_dir: &Path,
) -> OpenColorIoActivationValidation {
    match validate_open_color_io_activation(app_data_dir) {
        Ok(activation) => activation.abi_validation,
        Err(reason) => OpenColorIoActivationValidation {
            state: OpenColorIoActivationValidationState::Unavailable,
            reason,
        },
    }
}

pub fn acquire_media_conversion_dependency_plan(
    app_data_dir: &Path,
    request: MediaConversionDependencyPlanRequest,
) -> Result<MediaConversionDependencyPlan, String> {
    if request.holder.trim().is_empty() {
        return Err("Media conversion dependency lease holder must not be empty".to_string());
    }

    let dependencies = dependency_ids_for_request(&request);
    let mut acquired = Vec::new();

    for (redistributable_id, dependency_id) in dependencies {
        match acquire_dependency_lease(
            app_data_dir,
            redistributable_id,
            dependency_id,
            &request.holder,
        ) {
            Ok(lease) => acquired.push(lease),
            Err(error) => {
                release_acquired_leases(app_data_dir, &acquired);
                return Err(error);
            }
        }
    }

    let open_color_io_activation = if request.color_managed {
        match validate_open_color_io_activation(app_data_dir) {
            Ok(activation) => Some(activation),
            Err(error) => {
                release_acquired_leases(app_data_dir, &acquired);
                return Err(error);
            }
        }
    } else {
        None
    };

    Ok(MediaConversionDependencyPlan {
        job_kind: request.job_kind,
        color_managed: request.color_managed,
        leases: acquired,
        open_color_io_activation,
    })
}

pub fn release_media_conversion_dependency_plan(
    app_data_dir: &Path,
    plan: &MediaConversionDependencyPlan,
) -> Result<(), String> {
    let mut errors = Vec::new();
    for lease in plan.leases.iter().rev() {
        let managed_token = managed_lease_token_from_media_token(&lease.token);
        if let Err(error) = release_managed_redistributable_lease(app_data_dir, &managed_token) {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Failed to release media conversion dependency lease(s): {}",
            errors.join("; ")
        ))
    }
}

fn dependency_ids_for_request(
    request: &MediaConversionDependencyPlanRequest,
) -> Vec<(ManagedRedistributableId, MediaConversionDependencyId)> {
    let mut seen = HashSet::new();
    let mut dependencies = Vec::new();

    match request.job_kind {
        MediaConversionJobKind::Image | MediaConversionJobKind::ThreeD => push_dependency(
            &mut seen,
            &mut dependencies,
            ManagedRedistributableId::Oiiotool,
            MediaConversionDependencyId::Oiiotool,
        ),
        MediaConversionJobKind::Audio | MediaConversionJobKind::Video => push_dependency(
            &mut seen,
            &mut dependencies,
            ManagedRedistributableId::Ffmpeg,
            MediaConversionDependencyId::Ffmpeg,
        ),
    }

    if request.color_managed {
        push_dependency(
            &mut seen,
            &mut dependencies,
            ManagedRedistributableId::Ocioconvert,
            MediaConversionDependencyId::Ocioconvert,
        );
        push_dependency(
            &mut seen,
            &mut dependencies,
            ManagedRedistributableId::OpenColorIo,
            MediaConversionDependencyId::OpenColorIo,
        );
    }

    dependencies
}

fn push_dependency(
    seen: &mut HashSet<ManagedRedistributableId>,
    dependencies: &mut Vec<(ManagedRedistributableId, MediaConversionDependencyId)>,
    redistributable_id: ManagedRedistributableId,
    dependency_id: MediaConversionDependencyId,
) {
    if seen.insert(redistributable_id) {
        dependencies.push((redistributable_id, dependency_id));
    }
}

fn acquire_dependency_lease(
    app_data_dir: &Path,
    redistributable_id: ManagedRedistributableId,
    dependency_id: MediaConversionDependencyId,
    holder: &str,
) -> Result<MediaConversionDependencyLease, String> {
    ensure_active_dependency_available(app_data_dir, redistributable_id)?;
    let lease = acquire_managed_redistributable_lease(app_data_dir, redistributable_id, holder)?;
    let dependency =
        match resolve_active_dependency(app_data_dir, redistributable_id, dependency_id) {
            Ok(dependency) => dependency,
            Err(error) => {
                let _ = release_managed_redistributable_lease(app_data_dir, &lease);
                return Err(error);
            }
        };
    let token = MediaConversionDependencyLeaseToken {
        id: dependency_id,
        version: lease.version,
        lease_id: lease.lease_id,
    };

    Ok(MediaConversionDependencyLease { dependency, token })
}

fn ensure_active_dependency_available(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
) -> Result<(), String> {
    let status = managed_redistributable_status(app_data_dir, id);
    let Some(active_version) = status.selection.active_version.as_deref() else {
        return Err(format!(
            "{} does not have an active managed dependency version",
            status.display_name
        ));
    };

    let Some(version) = status
        .versions
        .iter()
        .find(|version| version.version == active_version && version.active)
    else {
        return Err(format!(
            "{} active managed dependency version {} is not available in the current catalog",
            status.display_name, active_version
        ));
    };

    if version.readiness != ManagedRedistributableReadiness::Ready {
        return Err(format!(
            "{} active managed dependency version {} is not ready; missing expected file(s): {}",
            status.display_name,
            active_version,
            version.missing_files.join(", ")
        ));
    }

    Ok(())
}

fn resolve_active_dependency(
    app_data_dir: &Path,
    redistributable_id: ManagedRedistributableId,
    dependency_id: MediaConversionDependencyId,
) -> Result<MediaConversionDependency, String> {
    ensure_active_dependency_available(app_data_dir, redistributable_id)?;
    let status = managed_redistributable_status(app_data_dir, redistributable_id);
    let active_version = status.selection.active_version.as_deref().ok_or_else(|| {
        format!(
            "{} does not have an active managed dependency version",
            status.display_name
        )
    })?;
    let version = status
        .versions
        .iter()
        .find(|version| version.version == active_version && version.active)
        .ok_or_else(|| {
            format!(
                "{} active managed dependency version {} is not available in the current catalog",
                status.display_name, active_version
            )
        })?;

    Ok(MediaConversionDependency {
        id: dependency_id,
        display_name: status.display_name,
        version: version.version.clone(),
        install_root: version.install_root.clone(),
        expected_files: version.expected_files.clone(),
    })
}

fn release_acquired_leases(app_data_dir: &Path, leases: &[MediaConversionDependencyLease]) {
    for lease in leases.iter().rev() {
        let managed_token = managed_lease_token_from_media_token(&lease.token);
        let _ = release_managed_redistributable_lease(app_data_dir, &managed_token);
    }
}

fn managed_lease_token_from_media_token(
    token: &MediaConversionDependencyLeaseToken,
) -> ManagedRedistributableLeaseToken {
    ManagedRedistributableLeaseToken {
        id: redistributable_id_for_dependency_id(token.id),
        version: token.version.clone(),
        lease_id: token.lease_id.clone(),
    }
}

fn redistributable_id_for_dependency_id(
    id: MediaConversionDependencyId,
) -> ManagedRedistributableId {
    match id {
        MediaConversionDependencyId::Ffmpeg => ManagedRedistributableId::Ffmpeg,
        MediaConversionDependencyId::Ocioconvert => ManagedRedistributableId::Ocioconvert,
        MediaConversionDependencyId::Oiiotool => ManagedRedistributableId::Oiiotool,
        MediaConversionDependencyId::OpenColorIo => ManagedRedistributableId::OpenColorIo,
    }
}
