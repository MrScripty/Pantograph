//! Extensions setup for host applications.
//!
//! Hosts call [`setup_extensions`] at startup to initialize optional runtime
//! dependencies and selector access roles in the shared `ExecutorExtensions`.
//! This keeps host crates decoupled from the underlying libraries — they don't
//! need to import `pumas-library` directly.

use node_engine::ExecutorExtensions;

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
}

#[cfg(feature = "model-library")]
#[derive(Debug, Clone)]
pub struct PumasSelectedModelDetail {
    pub selector_row: Option<pumas_library::models::ModelLibrarySelectorSnapshotRow>,
    pub descriptor: Option<pumas_library::models::ModelExecutionDescriptor>,
    pub package_summary_result: Option<pumas_library::models::ModelPackageFactsSummaryResult>,
    pub inference_settings: Vec<pumas_library::models::InferenceParamSchema>,
}

#[cfg(feature = "model-library")]
impl PumasSelectorAccess {
    pub fn role_name(&self) -> &'static str {
        match self {
            Self::Owner(_) => "owner",
            Self::LocalClient(_) => "local-client",
            Self::ReadOnly(_) => "read-only",
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
                api.get_inference_settings_batch(vec![model_id.to_string()])
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
                client
                    .get_inference_settings_batch(vec![model_id.to_string()])
                    .await?,
            ),
            Self::ReadOnly(_) => Ok(PumasSelectedModelDetail {
                selector_row,
                descriptor: None,
                package_summary_result: None,
                inference_settings: Vec::new(),
            }),
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
        }
    }
}

#[cfg(feature = "model-library")]
fn selected_model_detail_from_batch_owner(
    model_id: &str,
    selector_row: Option<pumas_library::models::ModelLibrarySelectorSnapshotRow>,
    descriptors: Vec<pumas_library::models::ModelExecutionDescriptorBatchItem>,
    summaries: Vec<pumas_library::models::ModelPackageFactsSummaryBatchItem>,
    settings: Vec<pumas_library::models::ModelInferenceSettingsBatchItem>,
) -> pumas_library::Result<PumasSelectedModelDetail> {
    let descriptor = descriptors
        .into_iter()
        .find(|item| item.model_id == model_id)
        .and_then(|item| item.descriptor);
    let package_summary_result = summaries
        .into_iter()
        .find(|item| item.model_id == model_id)
        .and_then(|item| item.result);
    let inference_settings = settings
        .into_iter()
        .find(|item| item.model_id == model_id)
        .map(|item| item.settings)
        .unwrap_or_default();

    Ok(PumasSelectedModelDetail {
        selector_row,
        descriptor,
        package_summary_result,
        inference_settings,
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
                "get_inference_settings_batch" => serde_json::to_value(vec![
                    pumas_library::models::ModelInferenceSettingsBatchItem {
                        model_id: model_id.to_string(),
                        settings: vec![pumas_library::models::InferenceParamSchema {
                            key: "temperature".to_string(),
                            label: "Temperature".to_string(),
                            param_type: pumas_library::models::ParamType::Number,
                            default: serde_json::json!(0.7),
                            description: None,
                            constraints: None,
                        }],
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
        assert_eq!(detail.inference_settings.len(), 1);
        assert_eq!(detail.inference_settings[0].key, "temperature");
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
