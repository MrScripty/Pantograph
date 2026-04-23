use super::super::catalog::fetch_managed_runtime_catalog;
use super::super::contracts::{
    ManagedBinaryId, ManagedRuntimeCatalogVersion, ManagedRuntimeJobState,
};
use super::super::definitions::definition;
use super::super::state::{
    ensure_runtime_state_entry, load_managed_runtime_state, runtime_state_entry,
    save_managed_runtime_state,
};
use super::state_transitions::current_unix_timestamp_ms;
use reqwest::StatusCode;
use std::fs;
use std::path::{Path, PathBuf};

const CATALOG_REFRESH_TTL_MS: u64 = 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManagedRuntimeDownloadArtifact {
    pub(super) temp_path: PathBuf,
    pub(super) downloaded_bytes: u64,
    pub(super) total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DownloadResponseMode {
    Fresh { total_size: u64 },
    Resume { total_size: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManagedRuntimeDownloadSource {
    pub(super) version: String,
    pub(super) runtime_key: String,
    pub(super) platform_key: String,
    pub(super) archive_name: String,
    pub(super) download_url: String,
}

pub(super) fn existing_download_artifact(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    runtime_version: &str,
    archive_name: &str,
) -> Result<Option<ManagedRuntimeDownloadArtifact>, String> {
    let state = load_managed_runtime_state(app_data_dir)?;
    let Some(runtime) = runtime_state_entry(&state, id) else {
        return Ok(None);
    };
    let artifact = runtime.active_job_artifact.as_ref();
    let Some(artifact) = artifact else {
        return Ok(None);
    };
    if artifact.version != runtime_version || artifact.archive_name != archive_name {
        return Ok(None);
    }

    let temp_path = PathBuf::from(&artifact.archive_path);
    let metadata = match fs::metadata(&temp_path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    let downloaded_bytes = metadata.len();
    if downloaded_bytes == 0 {
        return Ok(None);
    }

    Ok(Some(ManagedRuntimeDownloadArtifact {
        temp_path,
        downloaded_bytes,
        total_bytes: artifact.total_bytes.max(downloaded_bytes),
    }))
}

pub(super) fn persist_catalog_versions(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    catalog_versions: Vec<ManagedRuntimeCatalogVersion>,
) -> Result<(), String> {
    let mut state = load_managed_runtime_state(app_data_dir)?;
    let runtime = ensure_runtime_state_entry(&mut state, id);
    runtime.catalog_versions = catalog_versions;
    runtime.catalog_refreshed_at_ms = Some(current_unix_timestamp_ms());
    save_managed_runtime_state(app_data_dir, &state)
}

pub(super) async fn resolve_download_source(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    requested_version: Option<&str>,
) -> Result<ManagedRuntimeDownloadSource, String> {
    let definition = definition(id);
    let state = load_managed_runtime_state(app_data_dir)?;
    let runtime = runtime_state_entry(&state, id);

    let preferred_version = requested_version
        .map(str::to_string)
        .or_else(|| {
            runtime.and_then(|runtime| {
                runtime
                    .active_job
                    .as_ref()
                    .filter(|job| job.state == ManagedRuntimeJobState::Paused)
                    .and(runtime.active_job_artifact.as_ref())
                    .map(|artifact| artifact.version.clone())
            })
        })
        .or_else(|| runtime.and_then(|runtime| runtime.selection.selected_version.clone()));

    if let Some(version) = preferred_version.as_deref() {
        if let Some(download_source) = runtime
            .and_then(|runtime| {
                runtime
                    .catalog_versions
                    .iter()
                    .find(|entry| entry.version == version)
            })
            .map(download_source_from_catalog)
        {
            return Ok(download_source);
        }
    }

    let catalog_is_fresh = runtime
        .and_then(|runtime| runtime.catalog_refreshed_at_ms)
        .map(|timestamp| {
            current_unix_timestamp_ms().saturating_sub(timestamp) <= CATALOG_REFRESH_TTL_MS
        })
        .unwrap_or(false);

    if catalog_is_fresh {
        if let Some(download_source) = runtime
            .and_then(|runtime| runtime.catalog_versions.first())
            .map(download_source_from_catalog)
        {
            return Ok(download_source);
        }
    }

    let catalog = fetch_managed_runtime_catalog(id).await?;
    persist_catalog_versions(app_data_dir, id, catalog.clone())?;
    if let Some(version) = preferred_version.as_deref() {
        if let Some(download_source) = catalog
            .iter()
            .find(|entry| entry.version == version)
            .map(download_source_from_catalog)
        {
            return Ok(download_source);
        }

        return Err(format!(
            "{} version {} is not available for the current platform",
            definition.display_name(),
            version
        ));
    }

    if let Some(download_source) = catalog.first().map(download_source_from_catalog) {
        return Ok(download_source);
    }

    let version = definition.default_release_version().to_string();
    let release_asset = definition.release_asset(&version)?;
    Ok(ManagedRuntimeDownloadSource {
        version: version.clone(),
        runtime_key: id.key().to_string(),
        platform_key: definition.platform_key().to_string(),
        archive_name: release_asset.archive_name.clone(),
        download_url: definition.download_url(&version, &release_asset),
    })
}

fn download_source_from_catalog(
    catalog_version: &ManagedRuntimeCatalogVersion,
) -> ManagedRuntimeDownloadSource {
    ManagedRuntimeDownloadSource {
        version: catalog_version.version.clone(),
        runtime_key: catalog_version.runtime_key.clone(),
        platform_key: catalog_version.platform_key.clone(),
        archive_name: catalog_version.archive_name.clone(),
        download_url: catalog_version.download_url.clone(),
    }
}

fn parse_content_range_total(content_range: Option<&str>) -> Option<u64> {
    let content_range = content_range?;
    let (_, total) = content_range.split_once('/')?;
    total.parse::<u64>().ok()
}

pub(super) fn download_response_mode(
    requested_resume_from: u64,
    status: StatusCode,
    content_length: Option<u64>,
    content_range: Option<&str>,
) -> Result<DownloadResponseMode, String> {
    if requested_resume_from == 0 {
        if !status.is_success() {
            return Err(format!("Download failed with status: {status}"));
        }
        return Ok(DownloadResponseMode::Fresh {
            total_size: content_length.unwrap_or(0),
        });
    }

    match status {
        StatusCode::PARTIAL_CONTENT => Ok(DownloadResponseMode::Resume {
            total_size: parse_content_range_total(content_range)
                .or_else(|| content_length.map(|length| requested_resume_from + length))
                .unwrap_or(requested_resume_from),
        }),
        StatusCode::OK => Ok(DownloadResponseMode::Fresh {
            total_size: content_length.unwrap_or(0),
        }),
        _ => Err(format!("Download failed with status: {status}")),
    }
}
