//! Extensions setup for host applications.
//!
//! Hosts call [`setup_extensions`] at startup to initialize optional runtime
//! dependencies and selector access roles in the shared `ExecutorExtensions`.
//! This keeps host crates decoupled from the underlying libraries — they don't
//! need to import `pumas-library` directly.

use node_engine::ExecutorExtensions;

#[cfg(feature = "model-library")]
use crate::pumas_rpc::{PumasRpcClient, PumasRpcError, PumasRpcOperation};
#[cfg(feature = "model-library")]
use serde::de::DeserializeOwned;
#[cfg(feature = "model-library")]
use std::path::{Path, PathBuf};
#[cfg(feature = "model-library")]
use std::sync::Arc;

#[cfg(feature = "model-library")]
pub const PUMAS_SELECTOR_ACCESS: &str = "pumas_selector_access";

#[cfg(feature = "model-library")]
#[derive(Clone)]
pub enum PumasSelectorAccess {
    Owner(Arc<pumas_library::PumasApi>),
    LocalClient(Arc<pumas_library::PumasLocalClient>),
    ReadOnly(Arc<pumas_library::PumasReadOnlyLibrary>),
    Rpc(Arc<PumasRpcClient>),
}

#[cfg(feature = "model-library")]
#[derive(Debug, Clone)]
pub struct PumasSelectedModelDetail {
    pub selector_row: Option<pumas_library::models::ModelLibrarySelectorSnapshotRow>,
    pub descriptor: Option<pumas_library::models::ModelExecutionDescriptor>,
    pub package_summary_result: Option<pumas_library::models::ModelPackageFactsSummaryResult>,
}

#[cfg(feature = "model-library")]
impl PumasSelectorAccess {
    pub fn role_name(&self) -> &'static str {
        match self {
            Self::Owner(_) => "owner",
            Self::LocalClient(_) => "local-client",
            Self::ReadOnly(_) => "read-only",
            Self::Rpc(_) => "rpc",
        }
    }

    pub async fn model_library_selector_snapshot(
        &self,
        request: pumas_library::models::ModelLibrarySelectorSnapshotRequest,
    ) -> pumas_library::Result<pumas_library::models::ModelLibrarySelectorSnapshot> {
        match self {
            Self::Owner(api) => api.model_library_selector_snapshot(request).await,
            Self::LocalClient(client) => client.model_library_selector_snapshot(request).await,
            Self::ReadOnly(library) => library.model_library_selector_snapshot(request),
            Self::Rpc(client) => rpc_selector_snapshot(client, request).await,
        }
    }

    pub async fn list_model_library_updates_since(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> pumas_library::Result<pumas_library::models::ModelLibraryUpdateFeed> {
        match self {
            Self::Owner(api) => api.list_model_library_updates_since(cursor, limit).await,
            Self::LocalClient(client) => {
                let cursor = cursor.ok_or_else(|| pumas_library::PumasError::InvalidParams {
                    message: "local-client model-library update handoff requires a selector cursor"
                        .to_string(),
                })?;
                let stream = client
                    .subscribe_model_library_update_stream_since(cursor)
                    .await?;
                let handshake = stream.handshake();
                Ok(pumas_library::models::ModelLibraryUpdateFeed {
                    cursor: handshake.cursor_after_recovery.clone(),
                    events: handshake.recovered_events.clone(),
                    stale_cursor: handshake.stale_cursor,
                    snapshot_required: handshake.snapshot_required,
                })
            }
            Self::ReadOnly(_) => Err(pumas_library::PumasError::InvalidParams {
                message: "read-only Pumas selector access does not provide update feeds"
                    .to_string(),
            }),
            Self::Rpc(client) => decode_rpc_value(
                client
                    .call(PumasRpcOperation::ListModelLibraryUpdatesSince {
                        cursor: cursor.map(str::to_string),
                        limit,
                    })
                    .await
                    .map_err(map_rpc_error)?,
            ),
        }
    }

    pub async fn selected_model_detail(
        &self,
        model_id: &str,
    ) -> pumas_library::Result<PumasSelectedModelDetail> {
        let selector_row = self
            .model_library_selector_snapshot(
                pumas_library::models::ModelLibrarySelectorSnapshotRequest {
                    search: Some(model_id.to_string()),
                    limit: Some(25),
                    ..Default::default()
                },
            )
            .await?
            .rows
            .into_iter()
            .find(|row| row.model_id == model_id || row.model_ref.model_id == model_id);

        match self {
            Self::Owner(api) => selected_model_detail_from_batch_owner(
                model_id,
                selector_row,
                api.resolve_model_execution_descriptors_batch(vec![model_id.to_string()])
                    .await?,
                api.resolve_model_package_facts_summaries(vec![model_id.to_string()])
                    .await?,
            ),
            Self::LocalClient(client) => selected_model_detail_from_batch_owner(
                model_id,
                selector_row,
                client
                    .resolve_model_execution_descriptors_batch(vec![model_id.to_string()])
                    .await?,
                client
                    .resolve_model_package_facts_summaries(vec![model_id.to_string()])
                    .await?,
            ),
            Self::ReadOnly(_) => Ok(PumasSelectedModelDetail {
                selector_row,
                descriptor: None,
                package_summary_result: None,
            }),
            Self::Rpc(client) => Ok(PumasSelectedModelDetail {
                selector_row,
                descriptor: rpc_optional_model::<pumas_library::models::ModelExecutionDescriptor>(
                    client,
                    PumasRpcOperation::ResolveModelExecutionDescriptor {
                        model_id: model_id.to_string(),
                    },
                )
                .await?,
                package_summary_result: rpc_optional_model::<
                    pumas_library::models::ModelPackageFactsSummaryResult,
                >(
                    client,
                    PumasRpcOperation::ResolveModelPackageFactsSummary {
                        model_id: model_id.to_string(),
                    },
                )
                .await?,
            }),
        }
    }

    pub async fn model_record(
        &self,
        model_id: &str,
    ) -> pumas_library::Result<Option<pumas_library::ModelRecord>> {
        match self {
            Self::Owner(api) => api.get_model(model_id).await,
            Self::LocalClient(_) | Self::ReadOnly(_) => Ok(None),
            Self::Rpc(client) => rpc_model_record(client, model_id).await,
        }
    }

    pub async fn model_package_facts_summary_snapshot(
        &self,
        limit: usize,
        offset: usize,
    ) -> pumas_library::Result<pumas_library::models::ModelPackageFactsSummarySnapshot> {
        match self {
            Self::Owner(api) => {
                api.model_package_facts_summary_snapshot(limit, offset)
                    .await
            }
            Self::LocalClient(_) | Self::ReadOnly(_) => {
                let snapshot = self
                    .model_library_selector_snapshot(
                        pumas_library::models::ModelLibrarySelectorSnapshotRequest {
                            offset: Some(offset.min(u32::MAX as usize) as u32),
                            limit: Some(limit.min(u32::MAX as usize) as u32),
                            ..Default::default()
                        },
                    )
                    .await?;
                Ok(package_facts_summary_snapshot_from_selector(snapshot))
            }
            Self::Rpc(client) => decode_rpc_value(
                client
                    .call(PumasRpcOperation::ModelPackageFactsSummarySnapshot { limit, offset })
                    .await
                    .map_err(map_rpc_error)?,
            ),
        }
    }

    pub async fn resolve_model_package_facts_summary(
        &self,
        model_id: &str,
    ) -> pumas_library::Result<pumas_library::models::ModelPackageFactsSummaryResult> {
        match self {
            Self::Owner(api) => api.resolve_model_package_facts_summary(model_id).await,
            Self::LocalClient(client) => client
                .resolve_model_package_facts_summaries(vec![model_id.to_string()])
                .await?
                .into_iter()
                .find(|item| item.model_id == model_id)
                .and_then(|item| item.result)
                .ok_or_else(|| pumas_library::PumasError::NotFound {
                    resource: format!("model package facts summary '{model_id}'"),
                }),
            Self::ReadOnly(_) => {
                let snapshot = self
                    .model_library_selector_snapshot(
                        pumas_library::models::ModelLibrarySelectorSnapshotRequest {
                            search: Some(model_id.to_string()),
                            limit: Some(25),
                            ..Default::default()
                        },
                    )
                    .await?;
                snapshot
                    .rows
                    .into_iter()
                    .find(|row| row.model_id == model_id || row.model_ref.model_id == model_id)
                    .map(package_facts_summary_result_from_selector_row)
                    .ok_or_else(|| pumas_library::PumasError::NotFound {
                        resource: format!("model package facts summary '{model_id}'"),
                    })
            }
            Self::Rpc(client) => decode_rpc_value(
                client
                    .call(PumasRpcOperation::ResolveModelPackageFactsSummary {
                        model_id: model_id.to_string(),
                    })
                    .await
                    .map_err(map_rpc_error)?,
            ),
        }
    }

    pub async fn resolve_model_package_facts(
        &self,
        model_id: &str,
    ) -> pumas_library::Result<pumas_library::models::ResolvedModelPackageFacts> {
        match self {
            Self::Owner(api) => api.resolve_model_package_facts(model_id).await,
            Self::LocalClient(_) => Err(pumas_library::PumasError::InvalidParams {
                message:
                    "local-client Pumas selector access does not provide full package facts yet"
                        .to_string(),
            }),
            Self::ReadOnly(_) => Err(pumas_library::PumasError::InvalidParams {
                message: "read-only Pumas selector access does not provide full package facts"
                    .to_string(),
            }),
            Self::Rpc(client) => decode_rpc_value(
                client
                    .call(PumasRpcOperation::ResolveModelPackageFacts {
                        model_id: model_id.to_string(),
                    })
                    .await
                    .map_err(map_rpc_error)?,
            ),
        }
    }

    pub async fn resolve_model_artifact_load_target(
        &self,
        request: pumas_library::models::ResolveModelArtifactLoadTargetRequest,
    ) -> pumas_library::Result<pumas_library::models::ResolveModelArtifactLoadTargetResponse> {
        match self {
            Self::Owner(api) => api.resolve_model_artifact_load_target(request).await,
            Self::LocalClient(client) => client.resolve_model_artifact_load_target(request).await,
            Self::ReadOnly(library) => library.resolve_model_artifact_load_target(request),
            Self::Rpc(client) => decode_rpc_value(
                client
                    .call(PumasRpcOperation::ResolveModelArtifactLoadTarget { request })
                    .await
                    .map_err(map_rpc_error)?,
            ),
        }
    }
}

#[cfg(feature = "model-library")]
async fn rpc_selector_snapshot(
    client: &PumasRpcClient,
    request: pumas_library::models::ModelLibrarySelectorSnapshotRequest,
) -> pumas_library::Result<pumas_library::models::ModelLibrarySelectorSnapshot> {
    let value = client
        .call(PumasRpcOperation::GetModels)
        .await
        .map_err(map_rpc_error)?;
    let records = rpc_model_records(value)?;
    let search = request.search.as_deref().map(str::to_ascii_lowercase);
    let model_type = request.model_type.as_deref();
    let offset = request.offset.unwrap_or(0) as usize;
    let limit = request.limit.unwrap_or(100) as usize;
    let mut rows = records
        .into_iter()
        .filter_map(|(key, record)| {
            if search.as_ref().is_some_and(|search| {
                !key.to_ascii_lowercase().contains(search)
                    && !record.official_name.to_ascii_lowercase().contains(search)
            }) {
                return None;
            }
            if model_type.is_some_and(|expected| record.model_type != expected) {
                return None;
            }
            let model_id = if record.id.is_empty() { key } else { record.id };
            Some(pumas_library::models::ModelLibrarySelectorSnapshotRow {
                model_ref: pumas_library::models::PumasModelRef {
                    model_id: model_id.clone(),
                    ..Default::default()
                },
                model_id,
                repo_id: None,
                selected_artifact_id: None,
                selected_artifact_path: None,
                entry_path: None,
                entry_path_state: pumas_library::models::ModelEntryPathState::NeedsDetail,
                artifact_state: pumas_library::models::ModelArtifactState::NeedsDetail,
                display_name: record.official_name,
                model_type: Some(record.model_type),
                tags: record.tags,
                indexed_path: None,
                task_type_primary: None,
                pipeline_tag: None,
                recommended_backend: None,
                runtime_engine_hints: Vec::new(),
                storage_kind: None,
                validation_state: None,
                package_facts_summary_status:
                    pumas_library::models::ModelPackageFactsSummaryStatus::Missing,
                package_facts_summary: None,
                detail_state:
                    pumas_library::models::ModelLibrarySelectorDetailState::NeedsPackageFacts,
                updated_at: Some(record.updated_at),
            })
        })
        .collect::<Vec<_>>();
    let total_count = rows.len() as u64;
    rows = rows.into_iter().skip(offset).take(limit).collect();
    Ok(pumas_library::models::ModelLibrarySelectorSnapshot {
        selector_snapshot_contract_version:
            pumas_library::models::MODEL_LIBRARY_SELECTOR_SNAPSHOT_CONTRACT_VERSION,
        cursor: format!("rpc:get_models:{}", client.endpoint()),
        rows,
        total_count: Some(total_count),
    })
}

#[cfg(feature = "model-library")]
async fn rpc_model_record(
    client: &PumasRpcClient,
    model_id: &str,
) -> pumas_library::Result<Option<pumas_library::ModelRecord>> {
    let value = client
        .call(PumasRpcOperation::GetModels)
        .await
        .map_err(map_rpc_error)?;
    Ok(rpc_model_records(value)?
        .into_iter()
        .find(|(key, record)| key == model_id || record.id == model_id)
        .map(|(_, mut record)| {
            record.path.clear();
            redact_path_fields(&mut record.metadata);
            record
        }))
}

#[cfg(feature = "model-library")]
fn rpc_model_records(
    value: serde_json::Value,
) -> pumas_library::Result<std::collections::BTreeMap<String, pumas_library::ModelRecord>> {
    let models = value.get("models").cloned().unwrap_or(value);
    decode_rpc_value(models)
}

#[cfg(feature = "model-library")]
fn redact_path_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let path_keys = map
                .keys()
                .filter(|key| key.to_ascii_lowercase().contains("path"))
                .cloned()
                .collect::<Vec<_>>();
            for key in path_keys {
                map.remove(&key);
            }
            for child in map.values_mut() {
                redact_path_fields(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_path_fields(item);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "model-library")]
async fn rpc_optional_model<T: DeserializeOwned>(
    client: &PumasRpcClient,
    operation: PumasRpcOperation,
) -> pumas_library::Result<Option<T>> {
    match client.call(operation).await {
        Ok(value) => decode_rpc_value(value).map(Some),
        Err(PumasRpcError::Remote { code: -32002, .. }) => Ok(None),
        Err(error) => Err(map_rpc_error(error)),
    }
}

#[cfg(feature = "model-library")]
fn decode_rpc_value<T: DeserializeOwned>(value: serde_json::Value) -> pumas_library::Result<T> {
    serde_json::from_value(value).map_err(|error| pumas_library::PumasError::Json {
        message: error.to_string(),
        source: Some(error),
    })
}

#[cfg(feature = "model-library")]
fn map_rpc_error(error: PumasRpcError) -> pumas_library::PumasError {
    match error {
        PumasRpcError::Remote {
            code: -32002,
            message,
            ..
        } => pumas_library::PumasError::ModelNotFound { model_id: message },
        PumasRpcError::Remote {
            code: -32601,
            message,
            ..
        }
        | PumasRpcError::Remote {
            code: -32602,
            message,
            ..
        } => pumas_library::PumasError::InvalidParams { message },
        PumasRpcError::Timeout { timeout, .. } => pumas_library::PumasError::Timeout(timeout),
        PumasRpcError::Transport { message, .. } => pumas_library::PumasError::Network {
            message,
            cause: None,
        },
        other => pumas_library::PumasError::Other(other.to_string()),
    }
}

#[cfg(feature = "model-library")]
fn selected_model_detail_from_batch_owner(
    model_id: &str,
    selector_row: Option<pumas_library::models::ModelLibrarySelectorSnapshotRow>,
    descriptors: Vec<pumas_library::models::ModelExecutionDescriptorBatchItem>,
    summaries: Vec<pumas_library::models::ModelPackageFactsSummaryBatchItem>,
) -> pumas_library::Result<PumasSelectedModelDetail> {
    let descriptor = descriptors
        .into_iter()
        .find(|item| item.model_id == model_id)
        .and_then(|item| item.descriptor);
    let package_summary_result = summaries
        .into_iter()
        .find(|item| item.model_id == model_id)
        .and_then(|item| item.result);

    Ok(PumasSelectedModelDetail {
        selector_row,
        descriptor,
        package_summary_result,
    })
}

#[cfg(feature = "model-library")]
fn package_facts_summary_snapshot_from_selector(
    snapshot: pumas_library::models::ModelLibrarySelectorSnapshot,
) -> pumas_library::models::ModelPackageFactsSummarySnapshot {
    pumas_library::models::ModelPackageFactsSummarySnapshot {
        cursor: snapshot.cursor,
        items: snapshot
            .rows
            .into_iter()
            .map(
                |row| pumas_library::models::ModelPackageFactsSummarySnapshotItem {
                    model_id: row.model_ref.model_id,
                    status: row.package_facts_summary_status,
                    summary: row.package_facts_summary,
                },
            )
            .collect(),
    }
}

#[cfg(feature = "model-library")]
fn package_facts_summary_result_from_selector_row(
    row: pumas_library::models::ModelLibrarySelectorSnapshotRow,
) -> pumas_library::models::ModelPackageFactsSummaryResult {
    pumas_library::models::ModelPackageFactsSummaryResult {
        model_id: row.model_ref.model_id,
        status: row.package_facts_summary_status,
        summary: row.package_facts_summary,
    }
}

/// Initialize optional runtime dependencies in `ExecutorExtensions`.
///
/// Currently handles:
/// - **PumasApi** (`model-library` feature): Tries explicit/local launcher
///   roots first (`library_path`, then `PUMAS_LIBRARY_PATH`) and falls back to
///   `PumasApi::discover()` (global registry at `~/.config/pumas/registry.db`).
/// - **Pumas selector access** (`model-library` feature): Registers the
///   explicit selector access role selected during setup: owner API, read-only
///   local model index, or local-client IPC.
///
/// # Example
///
/// ```ignore
/// let mut extensions = node_engine::ExecutorExtensions::new();
/// workflow_nodes::setup_extensions(&mut extensions).await;
/// // extensions now has PumasApi and/or Pumas selector access (if available)
/// ```
#[cfg(feature = "model-library")]
pub async fn setup_extensions(extensions: &mut ExecutorExtensions) {
    setup_extensions_with_path(extensions, None).await;
}

/// Configure an explicit, library-only Pumas HTTP RPC endpoint.
///
/// This path intentionally does not inspect launcher roots, open a Pumas
/// database, discover local instances, or fall back to the global registry.
#[cfg(feature = "model-library")]
pub fn setup_extensions_with_rpc_endpoint(
    extensions: &mut ExecutorExtensions,
    endpoint: &str,
) -> Result<(), PumasRpcError> {
    let client = Arc::new(PumasRpcClient::new(endpoint)?);
    extensions.set(
        PUMAS_SELECTOR_ACCESS,
        Arc::new(PumasSelectorAccess::Rpc(client)),
    );
    Ok(())
}

/// Initialize extensions with an explicit library path fallback.
///
/// Tries in order:
/// 1. Owner API from configured launcher roots derived from `library_path` and
///    `PUMAS_LIBRARY_PATH`.
/// 2. Read-only selector access from configured model-library roots containing
///    `models.db`.
/// 3. Local-client selector access from Pumas ready-instance discovery.
/// 4. Owner API from `PumasApi::discover()`.
#[cfg(feature = "model-library")]
pub async fn setup_extensions_with_path(
    extensions: &mut ExecutorExtensions,
    library_path: Option<&std::path::Path>,
) {
    // Build candidate paths: explicit parameter first, then env var
    let mut raw_candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = library_path {
        raw_candidates.push(p.to_path_buf());
    }
    if let Ok(env_path) = std::env::var("PUMAS_LIBRARY_PATH") {
        raw_candidates.push(std::path::PathBuf::from(env_path));
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in &raw_candidates {
        for expanded in expand_candidate_path(raw) {
            push_unique(&mut candidates, &mut seen, expanded);
        }
    }

    let mut api: Option<Arc<pumas_library::PumasApi>> = None;
    let mut selector_access: Option<Arc<PumasSelectorAccess>> = None;
    for path in &candidates {
        if !path.exists() {
            log::info!("Skipping non-existent library path: {:?}", path);
            continue;
        }
        log::info!("Trying PumasApi at {:?}", path);
        match pumas_library::PumasApi::builder(path)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
        {
            Ok(found) => {
                log::info!("PumasApi initialized from {:?}", path);
                let found = Arc::new(found);
                selector_access = Some(Arc::new(PumasSelectorAccess::Owner(found.clone())));
                api = Some(found);
                break;
            }
            Err(e) => {
                log::warn!("PumasApi::builder({:?}) failed: {}", path, e);
            }
        }
    }

    if selector_access.is_none() {
        for raw in &raw_candidates {
            let Some(model_library_root) = resolve_pumas_model_library_root(raw) else {
                continue;
            };
            match pumas_library::PumasReadOnlyLibrary::open(&model_library_root) {
                Ok(library) => {
                    log::info!(
                        "Pumas selector access opened read-only model library at {:?}",
                        model_library_root
                    );
                    selector_access =
                        Some(Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(library))));
                    break;
                }
                Err(error) => {
                    log::warn!(
                        "PumasReadOnlyLibrary::open({:?}) failed: {}",
                        model_library_root,
                        error
                    );
                }
            }
        }
    }

    if selector_access.is_none() {
        match pumas_library::PumasLocalClient::discover_ready_instances() {
            Ok(instances) => {
                for instance in instances {
                    match pumas_library::PumasLocalClient::connect(instance).await {
                        Ok(client) => {
                            log::info!("Pumas selector access connected as local client");
                            selector_access =
                                Some(Arc::new(PumasSelectorAccess::LocalClient(Arc::new(client))));
                            break;
                        }
                        Err(error) => {
                            log::warn!("PumasLocalClient connect failed: {}", error);
                        }
                    }
                }
            }
            Err(error) => {
                log::info!("PumasLocalClient discovery unavailable: {}", error);
            }
        }
    }

    if api.is_none() && selector_access.is_none() {
        if raw_candidates.is_empty() {
            log::info!(
                "No pumas-library path configured. \
                 Set PUMAS_LIBRARY_PATH or pass a path to setup_extensions_with_path()."
            );
        }
        match pumas_library::PumasApi::discover().await {
            Ok(found) => {
                log::info!("PumasApi connected via discover()");
                let found = Arc::new(found);
                selector_access = Some(Arc::new(PumasSelectorAccess::Owner(found.clone())));
                api = Some(found);
            }
            Err(e) => {
                log::info!("PumasApi discover() unavailable: {}", e);
            }
        }
    }

    if let Some(api) = api {
        extensions.set(node_engine::extension_keys::PUMAS_API, api);
    }

    if let Some(selector_access) = selector_access {
        extensions.set(PUMAS_SELECTOR_ACCESS, selector_access);
    }
}

#[cfg(feature = "model-library")]
fn is_launcher_root(path: &Path) -> bool {
    path.join("shared-resources").exists() && path.join("launcher-data").exists()
}

#[cfg(feature = "model-library")]
fn push_unique(
    out: &mut Vec<PathBuf>,
    seen: &mut std::collections::HashSet<PathBuf>,
    path: PathBuf,
) {
    if seen.insert(path.clone()) {
        out.push(path);
    }
}

/// Accept either launcher root paths or build output dirs like:
/// `<repo>/rust/target/release` by deriving the launcher root.
#[cfg(feature = "model-library")]
fn expand_candidate_path(path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(build_kind) = path.file_name().and_then(|n| n.to_str()) {
        if (build_kind == "release" || build_kind == "debug")
            && path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("target")
            && path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("rust")
        {
            if let Some(root) = path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                if is_launcher_root(root) {
                    push_unique(&mut out, &mut seen, root.to_path_buf());
                }
            }
        }
    }

    if is_launcher_root(path) {
        push_unique(&mut out, &mut seen, path.to_path_buf());
    }

    for ancestor in path.ancestors() {
        if is_launcher_root(ancestor) {
            push_unique(&mut out, &mut seen, ancestor.to_path_buf());
        }
    }

    out
}

#[cfg(feature = "model-library")]
pub fn resolve_pumas_model_library_root(path: &Path) -> Option<PathBuf> {
    let candidates = [
        path.to_path_buf(),
        path.join("shared-resources").join("models"),
    ];
    for candidate in candidates {
        if candidate.join("models.db").is_file() {
            return Some(candidate);
        }
    }

    for launcher_root in expand_candidate_path(path) {
        let model_library_root = launcher_root.join("shared-resources").join("models");
        if model_library_root.join("models.db").is_file() {
            return Some(model_library_root);
        }
    }

    None
}

#[cfg(all(test, feature = "model-library"))]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use node_engine::extension_keys;
    use pumas_library::ipc::{IpcDispatch, IpcServer};
    use pumas_library::model_library::ModelLibrary;
    use pumas_library::registry::{InstanceEntry, InstanceStatus, LocalInstanceTransportKind};
    use pumas_library::ModelIndex;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use tempfile::TempDir;

    struct UpdateStreamDispatch {
        library: ModelLibrary,
    }

    struct SelectedDetailDispatch;

    #[async_trait]
    impl IpcDispatch for UpdateStreamDispatch {
        async fn dispatch(
            &self,
            method: &str,
            _params: serde_json::Value,
        ) -> pumas_library::Result<serde_json::Value> {
            Err(pumas_library::PumasError::Other(format!(
                "unexpected IPC method: {method}"
            )))
        }

        async fn subscribe_model_library_update_stream_since(
            &self,
            cursor: &str,
            _connection_token: Option<&str>,
        ) -> pumas_library::Result<Option<pumas_library::model_library::ModelLibraryUpdateSubscriber>>
        {
            Ok(Some(
                self.library
                    .subscribe_model_library_update_stream_since(cursor)
                    .await?,
            ))
        }
    }

    #[async_trait]
    impl IpcDispatch for SelectedDetailDispatch {
        async fn dispatch(
            &self,
            method: &str,
            _params: serde_json::Value,
        ) -> pumas_library::Result<serde_json::Value> {
            let model_id = "llm/imported/local-client-test";
            match method {
                "model_library_selector_snapshot" => {
                    serde_json::to_value(pumas_library::models::ModelLibrarySelectorSnapshot {
                        selector_snapshot_contract_version:
                            pumas_library::models::MODEL_LIBRARY_SELECTOR_SNAPSHOT_CONTRACT_VERSION,
                        cursor: "model-library-updates:7".to_string(),
                        rows: vec![pumas_library::models::ModelLibrarySelectorSnapshotRow {
                            model_id: model_id.to_string(),
                            model_ref: pumas_library::models::PumasModelRef {
                                model_id: model_id.to_string(),
                                selected_artifact_id: Some("model.gguf".to_string()),
                                selected_artifact_path: Some(format!("{model_id}/model.gguf")),
                                ..Default::default()
                            },
                            repo_id: None,
                            selected_artifact_id: Some("model.gguf".to_string()),
                            selected_artifact_path: Some(format!("{model_id}/model.gguf")),
                            entry_path: Some("/tmp/pumas/model.gguf".to_string()),
                            entry_path_state: pumas_library::models::ModelEntryPathState::Ready,
                            artifact_state: pumas_library::models::ModelArtifactState::Ready,
                            display_name: "Local Client Test".to_string(),
                            model_type: Some("llm".to_string()),
                            tags: vec!["gguf".to_string()],
                            indexed_path: Some(model_id.to_string()),
                            task_type_primary: Some("text-generation".to_string()),
                            pipeline_tag: Some("text-generation".to_string()),
                            recommended_backend: Some("llamacpp".to_string()),
                            runtime_engine_hints: vec!["llamacpp".to_string()],
                            storage_kind: Some(pumas_library::models::StorageKind::LibraryOwned),
                            validation_state: Some(
                                pumas_library::models::AssetValidationState::Valid,
                            ),
                            package_facts_summary_status:
                                pumas_library::models::ModelPackageFactsSummaryStatus::Cached,
                            package_facts_summary: None,
                            detail_state:
                                pumas_library::models::ModelLibrarySelectorDetailState::Complete,
                            updated_at: Some("2026-05-08T00:00:00Z".to_string()),
                        }],
                        total_count: Some(1),
                    })
                    .map_err(|error| pumas_library::PumasError::Other(error.to_string()))
                }
                "resolve_model_execution_descriptors_batch" => serde_json::to_value(vec![
                    pumas_library::models::ModelExecutionDescriptorBatchItem {
                        model_id: model_id.to_string(),
                        descriptor: Some(pumas_library::models::ModelExecutionDescriptor {
                            execution_contract_version: 1,
                            model_id: model_id.to_string(),
                            entry_path: "/tmp/pumas/model.gguf".to_string(),
                            model_type: "llm".to_string(),
                            task_type_primary: "text-generation".to_string(),
                            recommended_backend: Some("llamacpp".to_string()),
                            runtime_engine_hints: vec!["llamacpp".to_string()],
                            storage_kind: pumas_library::models::StorageKind::LibraryOwned,
                            validation_state: pumas_library::models::AssetValidationState::Valid,
                            dependency_resolution: Some(serde_json::json!({
                                "bindings": [{
                                    "binding_id": "binding-a",
                                    "backend_key": "llamacpp"
                                }]
                            })),
                        }),
                        error: None,
                    },
                ])
                .map_err(|error| pumas_library::PumasError::Other(error.to_string())),
                "resolve_model_package_facts_summaries" => serde_json::to_value(vec![
                    pumas_library::models::ModelPackageFactsSummaryBatchItem {
                        model_id: model_id.to_string(),
                        result: None,
                        error: None,
                    },
                ])
                .map_err(|error| pumas_library::PumasError::Other(error.to_string())),
                _ => Err(pumas_library::PumasError::Other(format!(
                    "unexpected IPC method: {method}"
                ))),
            }
        }
    }

    fn create_models_db(model_root: &Path) {
        std::fs::create_dir_all(model_root).unwrap();
        std::fs::write(model_root.join("models.db"), []).unwrap();
    }

    fn create_model_index(model_root: &Path) {
        std::fs::create_dir_all(model_root).unwrap();
        let _index = ModelIndex::new(model_root.join("models.db")).unwrap();
    }

    fn create_launcher_root() -> TempDir {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("launcher-data")).unwrap();
        std::fs::create_dir_all(temp.path().join("shared-resources/models")).unwrap();
        temp
    }

    fn ready_instance(port: u16) -> InstanceEntry {
        InstanceEntry {
            library_path: PathBuf::from("/tmp/pantograph-pumas-test-library"),
            pid: std::process::id(),
            port,
            transport_kind: LocalInstanceTransportKind::LoopbackTcp,
            endpoint: format!("127.0.0.1:{port}"),
            connection_token: Some("token".to_string()),
            started_at: "2026-05-06T00:00:00Z".to_string(),
            version: None,
            status: InstanceStatus::Ready,
        }
    }

    fn rpc_test_server(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind RPC test server");
        let address = listener.local_addr().expect("RPC test server address");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept RPC test request");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.len(),
                response
            )
            .expect("write RPC test response");
        });
        format!("http://{address}/rpc")
    }

    #[test]
    fn read_only_root_resolves_from_direct_models_root() {
        let temp = TempDir::new().unwrap();
        create_models_db(temp.path());

        let root = resolve_pumas_model_library_root(temp.path());

        assert_eq!(root.as_deref(), Some(temp.path()));
    }

    #[test]
    fn read_only_root_resolves_from_launcher_root() {
        let temp = create_launcher_root();
        let model_root = temp.path().join("shared-resources/models");
        create_models_db(&model_root);

        let root = resolve_pumas_model_library_root(temp.path());

        assert_eq!(root, Some(model_root));
    }

    #[test]
    fn read_only_root_resolves_from_pumas_target_build_dir() {
        let temp = create_launcher_root();
        let model_root = temp.path().join("shared-resources/models");
        create_models_db(&model_root);
        let build_dir = temp.path().join("rust/target/release");
        std::fs::create_dir_all(&build_dir).unwrap();

        let root = resolve_pumas_model_library_root(&build_dir);

        assert_eq!(root, Some(model_root));
    }

    #[test]
    fn read_only_root_resolution_dedupes_candidates() {
        let temp = create_launcher_root();
        let nested = temp.path().join("rust/target/release");
        std::fs::create_dir_all(&nested).unwrap();

        let expanded = expand_candidate_path(&nested);

        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0], temp.path());
    }

    #[test]
    fn read_only_setup_does_not_create_missing_models_db() {
        let temp = create_launcher_root();
        let model_root = temp.path().join("shared-resources/models");

        let root = resolve_pumas_model_library_root(temp.path());

        assert!(root.is_none());
        assert!(!model_root.join("models.db").exists());
    }

    #[tokio::test]
    async fn read_only_setup_uses_direct_models_root_without_owner_api() {
        let temp = TempDir::new().unwrap();
        create_model_index(temp.path());
        let mut extensions = ExecutorExtensions::new();

        setup_extensions_with_path(&mut extensions, Some(temp.path())).await;

        let selector_access = extensions
            .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
            .expect("read-only selector access should be registered");
        assert_eq!(selector_access.role_name(), "read-only");
        assert!(
            extensions
                .get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
                .is_none(),
            "read-only selector setup must not claim owner API access"
        );
    }

    #[tokio::test]
    async fn rpc_selector_access_projects_get_models_without_local_paths() {
        let endpoint = rpc_test_server(
            r#"{"jsonrpc":"2.0","result":{"llm/test":{"id":"llm/test","path":"/private/pumas/model","cleanedName":"tiny","officialName":"Tiny Test","modelType":"llm","tags":["text"],"hashes":{},"metadata":{},"updatedAt":"2026-09-25T00:00:00Z"}},"id":1}"#,
        );
        let access = PumasSelectorAccess::Rpc(Arc::new(
            PumasRpcClient::new(&endpoint).expect("RPC client"),
        ));

        let snapshot = access
            .model_library_selector_snapshot(
                pumas_library::models::ModelLibrarySelectorSnapshotRequest {
                    search: Some("tiny".to_string()),
                    limit: Some(10),
                    ..Default::default()
                },
            )
            .await
            .expect("RPC selector snapshot");

        assert_eq!(access.role_name(), "rpc");
        assert_eq!(snapshot.rows.len(), 1);
        let row = &snapshot.rows[0];
        assert_eq!(row.model_id, "llm/test");
        assert_eq!(row.display_name, "Tiny Test");
        assert!(row.entry_path.is_none());
        assert!(row.selected_artifact_id.is_none());
        assert_eq!(
            row.detail_state,
            pumas_library::models::ModelLibrarySelectorDetailState::NeedsPackageFacts
        );
    }

    #[test]
    fn rpc_model_record_metadata_redacts_path_fields() {
        let mut value = serde_json::json!({
            "entry_path": "/private/pumas/model",
            "nested": {"localPath": "/private/pumas/weights", "size": 1},
            "task": "text_generation"
        });

        redact_path_fields(&mut value);

        assert_eq!(
            value,
            serde_json::json!({
                "nested": {"size": 1},
                "task": "text_generation"
            })
        );
    }

    #[tokio::test]
    async fn local_client_update_feed_recovers_events_from_selector_cursor() {
        let temp = TempDir::new().unwrap();
        let library_root = temp.path().join("models");
        std::fs::create_dir_all(&library_root).unwrap();
        let library = ModelLibrary::new(&library_root).await.unwrap();
        let cursor = library
            .list_model_library_updates_since(None, 100)
            .await
            .unwrap()
            .cursor;
        library
            .notify_model_library_refresh("local-client-handoff-test")
            .unwrap();
        let Some(server) = IpcServer::start(Arc::new(UpdateStreamDispatch {
            library: library.clone(),
        }))
        .await
        .ok() else {
            eprintln!("Skipping local_client_update_feed_recovers_events_from_selector_cursor");
            return;
        };
        let client = pumas_library::PumasLocalClient::connect(ready_instance(server.port))
            .await
            .unwrap();
        let access = PumasSelectorAccess::LocalClient(Arc::new(client));

        let feed = access
            .list_model_library_updates_since(Some(&cursor), 100)
            .await
            .expect("local client update feed should recover durable updates");

        assert!(!feed.stale_cursor);
        assert!(!feed.snapshot_required);
        assert!(feed.cursor.starts_with("model-library-updates:"));
        assert_eq!(feed.events.len(), 1);
        assert_eq!(feed.events[0].model_id, "__library__/model-library-refresh");
    }

    #[tokio::test]
    async fn local_client_selected_model_detail_uses_batch_detail_methods() {
        let Some(server) = IpcServer::start(Arc::new(SelectedDetailDispatch))
            .await
            .ok()
        else {
            eprintln!("Skipping local_client_selected_model_detail_uses_batch_detail_methods");
            return;
        };
        let client = pumas_library::PumasLocalClient::connect(ready_instance(server.port))
            .await
            .unwrap();
        let access = PumasSelectorAccess::LocalClient(Arc::new(client));

        let detail = access
            .selected_model_detail("llm/imported/local-client-test")
            .await
            .expect("local client selected detail should load from batch APIs");

        assert_eq!(
            detail
                .selector_row
                .as_ref()
                .map(|row| row.display_name.as_str()),
            Some("Local Client Test")
        );
        let descriptor = detail.descriptor.expect("descriptor should hydrate");
        assert_eq!(descriptor.recommended_backend.as_deref(), Some("llamacpp"));
    }

    #[tokio::test]
    async fn read_only_update_feed_reports_unavailable_without_lifecycle() {
        let temp = TempDir::new().unwrap();
        create_model_index(temp.path());
        let library = pumas_library::PumasReadOnlyLibrary::open(temp.path()).unwrap();
        let access = PumasSelectorAccess::ReadOnly(Arc::new(library));

        let error = access
            .list_model_library_updates_since(Some("model-library-updates:1"), 100)
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("read-only Pumas selector access does not provide update feeds"),
            "unexpected error: {error}"
        );
    }
}

/// No-op when `model-library` feature is disabled.
#[cfg(not(feature = "model-library"))]
pub async fn setup_extensions(_extensions: &mut ExecutorExtensions) {}

/// No-op when `model-library` feature is disabled.
#[cfg(not(feature = "model-library"))]
pub async fn setup_extensions_with_path(
    _extensions: &mut ExecutorExtensions,
    _library_path: Option<&std::path::Path>,
) {
}
