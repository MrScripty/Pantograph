use std::path::{Path, PathBuf};

use pantograph_media_conversion as media_conversion;
use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub holder: String,
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

pub fn format_media_conversion_dependency_lease_holder(
    workflow_run_id: &str,
    node_id: &str,
    port_id: &str,
    conversion_id: &str,
) -> Result<String, String> {
    media_conversion::format_managed_media_dependency_lease_holder(
        workflow_run_id,
        node_id,
        port_id,
        conversion_id,
    )
}

pub fn validate_media_conversion_dependency_lease_holder(holder: &str) -> Result<(), String> {
    media_conversion::validate_managed_media_dependency_lease_holder(holder)
}

pub fn media_conversion_dependency_lease_holder_convention() -> &'static str {
    media_conversion::managed_media_dependency_lease_holder_convention()
}

pub fn validate_open_color_io_activation(
    app_data_dir: &Path,
) -> Result<OpenColorIoActivation, String> {
    media_conversion::validate_open_color_io_activation(app_data_dir).map(legacy_activation)
}

pub fn open_color_io_activation_validation_state(
    app_data_dir: &Path,
) -> OpenColorIoActivationValidation {
    legacy_activation_validation(media_conversion::open_color_io_activation_validation_state(
        app_data_dir,
    ))
}

pub fn acquire_media_conversion_dependency_plan(
    app_data_dir: &Path,
    request: MediaConversionDependencyPlanRequest,
) -> Result<MediaConversionDependencyPlan, String> {
    let media_request = media_conversion::ManagedMediaDependencyPlanRequest {
        kind: media_kind_from_job_kind(request.job_kind),
        color_managed: request.color_managed,
        holder: request.holder,
    };
    media_conversion::acquire_managed_media_dependency_plan(app_data_dir, media_request)
        .map(|plan| legacy_plan(request.job_kind, plan))
}

pub fn release_media_conversion_dependency_plan(
    app_data_dir: &Path,
    plan: &MediaConversionDependencyPlan,
) -> Result<(), String> {
    media_conversion::release_managed_media_dependency_plan(app_data_dir, &media_plan(plan))
}

pub fn resolve_media_conversion_dependency_executable_path(
    dependency: &MediaConversionDependency,
) -> Result<PathBuf, String> {
    media_conversion::resolve_managed_media_dependency_executable_path(&media_dependency(
        dependency,
    ))
}

fn legacy_plan(
    job_kind: MediaConversionJobKind,
    plan: media_conversion::ManagedMediaDependencyPlan,
) -> MediaConversionDependencyPlan {
    MediaConversionDependencyPlan {
        job_kind,
        color_managed: plan.color_managed,
        leases: plan.leases.into_iter().map(legacy_lease).collect(),
        open_color_io_activation: plan.open_color_io_activation.map(legacy_activation),
    }
}

fn media_plan(
    plan: &MediaConversionDependencyPlan,
) -> media_conversion::ManagedMediaDependencyPlan {
    media_conversion::ManagedMediaDependencyPlan {
        kind: media_kind_from_job_kind(plan.job_kind),
        color_managed: plan.color_managed,
        leases: plan.leases.iter().map(media_lease).collect(),
        open_color_io_activation: plan.open_color_io_activation.as_ref().map(media_activation),
    }
}

fn legacy_lease(
    lease: media_conversion::ManagedMediaDependencyLease,
) -> MediaConversionDependencyLease {
    MediaConversionDependencyLease {
        dependency: legacy_dependency(lease.dependency),
        token: legacy_lease_token(lease.token),
    }
}

fn media_lease(
    lease: &MediaConversionDependencyLease,
) -> media_conversion::ManagedMediaDependencyLease {
    media_conversion::ManagedMediaDependencyLease {
        dependency: media_dependency(&lease.dependency),
        token: media_lease_token(&lease.token),
    }
}

fn legacy_lease_token(
    token: media_conversion::ManagedMediaDependencyLeaseToken,
) -> MediaConversionDependencyLeaseToken {
    MediaConversionDependencyLeaseToken {
        id: dependency_id_from_media_id(token.id),
        version: token.version,
        lease_id: token.lease_id,
        holder: token.holder,
    }
}

fn media_lease_token(
    token: &MediaConversionDependencyLeaseToken,
) -> media_conversion::ManagedMediaDependencyLeaseToken {
    media_conversion::ManagedMediaDependencyLeaseToken {
        id: media_id_from_dependency_id(token.id),
        version: token.version.clone(),
        lease_id: token.lease_id.clone(),
        holder: token.holder.clone(),
    }
}

fn legacy_activation(activation: media_conversion::OpenColorIoActivation) -> OpenColorIoActivation {
    OpenColorIoActivation {
        dependency: legacy_dependency(activation.dependency),
        abi_validation: legacy_activation_validation(activation.abi_validation),
    }
}

fn media_activation(activation: &OpenColorIoActivation) -> media_conversion::OpenColorIoActivation {
    media_conversion::OpenColorIoActivation {
        dependency: media_dependency(&activation.dependency),
        abi_validation: media_activation_validation(activation.abi_validation.clone()),
    }
}

fn legacy_activation_validation(
    validation: media_conversion::OpenColorIoActivationValidation,
) -> OpenColorIoActivationValidation {
    OpenColorIoActivationValidation {
        state: match validation.state {
            media_conversion::OpenColorIoActivationValidationState::NotValidated => {
                OpenColorIoActivationValidationState::NotValidated
            }
            media_conversion::OpenColorIoActivationValidationState::Unavailable => {
                OpenColorIoActivationValidationState::Unavailable
            }
        },
        reason: validation.reason,
    }
}

fn media_activation_validation(
    validation: OpenColorIoActivationValidation,
) -> media_conversion::OpenColorIoActivationValidation {
    media_conversion::OpenColorIoActivationValidation {
        state: match validation.state {
            OpenColorIoActivationValidationState::NotValidated => {
                media_conversion::OpenColorIoActivationValidationState::NotValidated
            }
            OpenColorIoActivationValidationState::Unavailable => {
                media_conversion::OpenColorIoActivationValidationState::Unavailable
            }
        },
        reason: validation.reason,
    }
}

fn legacy_dependency(
    dependency: media_conversion::ManagedMediaDependency,
) -> MediaConversionDependency {
    MediaConversionDependency {
        id: dependency_id_from_media_id(dependency.id),
        display_name: dependency.display_name,
        version: dependency.version,
        install_root: dependency.install_root,
        expected_files: dependency.expected_files,
    }
}

fn media_dependency(
    dependency: &MediaConversionDependency,
) -> media_conversion::ManagedMediaDependency {
    media_conversion::ManagedMediaDependency {
        id: media_id_from_dependency_id(dependency.id),
        display_name: dependency.display_name.clone(),
        version: dependency.version.clone(),
        install_root: dependency.install_root.clone(),
        expected_files: dependency.expected_files.clone(),
    }
}

fn media_kind_from_job_kind(
    job_kind: MediaConversionJobKind,
) -> media_conversion::ConversionMediaKind {
    match job_kind {
        MediaConversionJobKind::Image => media_conversion::ConversionMediaKind::Image,
        MediaConversionJobKind::Audio => media_conversion::ConversionMediaKind::Audio,
        MediaConversionJobKind::Video => media_conversion::ConversionMediaKind::Video,
        MediaConversionJobKind::ThreeD => media_conversion::ConversionMediaKind::ThreeD,
    }
}

fn media_id_from_dependency_id(
    id: MediaConversionDependencyId,
) -> media_conversion::ManagedMediaDependencyId {
    match id {
        MediaConversionDependencyId::Ffmpeg => media_conversion::ManagedMediaDependencyId::Ffmpeg,
        MediaConversionDependencyId::Ocioconvert => {
            media_conversion::ManagedMediaDependencyId::Ocioconvert
        }
        MediaConversionDependencyId::Oiiotool => {
            media_conversion::ManagedMediaDependencyId::Oiiotool
        }
        MediaConversionDependencyId::OpenColorIo => {
            media_conversion::ManagedMediaDependencyId::OpenColorIo
        }
    }
}

fn dependency_id_from_media_id(
    id: media_conversion::ManagedMediaDependencyId,
) -> MediaConversionDependencyId {
    match id {
        media_conversion::ManagedMediaDependencyId::Ffmpeg => MediaConversionDependencyId::Ffmpeg,
        media_conversion::ManagedMediaDependencyId::Ocioconvert => {
            MediaConversionDependencyId::Ocioconvert
        }
        media_conversion::ManagedMediaDependencyId::Oiiotool => {
            MediaConversionDependencyId::Oiiotool
        }
        media_conversion::ManagedMediaDependencyId::OpenColorIo => {
            MediaConversionDependencyId::OpenColorIo
        }
    }
}
