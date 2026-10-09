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
        // The consumer URI denotes the same relative Pumas identity; the IPC
        // operation accepts the relative identity, never an executable path.
        let model_id = model_id.strip_prefix("pumas://models/").unwrap_or(model_id);
        match self {
            Self::Owner(api) => api.resolve_model_package_facts(model_id).await,
            Self::LocalClient(client) => client.resolve_model_package_facts(model_id).await,
            Self::ReadOnly(_) => Err(pumas_library::PumasError::InvalidParams {
                message: "read-only Pumas selector access does not provide full package facts"
                    .to_string(),
            }),
        }
    }

    pub async fn resolve_model_artifact_load_target(
        &self,
        mut request: pumas_library::models::ResolveModelArtifactLoadTargetRequest,
    ) -> pumas_library::Result<pumas_library::models::ResolveModelArtifactLoadTargetResponse> {
        request.model_ref.model_id = request
            .model_ref
            .model_id
            .strip_prefix("pumas://models/")
            .unwrap_or(&request.model_ref.model_id)
            .to_string();
        match self {
            Self::Owner(api) => api.resolve_model_artifact_load_target(request).await,
            Self::LocalClient(client) => client.resolve_model_artifact_load_target(request).await,
            Self::ReadOnly(library) => library.resolve_model_artifact_load_target(request),
        }
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

/// Initialize extensions with an explicit library path fallback.
///
/// Tries in order:
/// 1. Owner API from configured launcher roots derived from `library_path` and
///    `PUMAS_LIBRARY_PATH`, or the local client for that same root when another
///    process already owns it.
/// 2. Read-only selector access from configured model-library roots containing
///    `models.db`.
/// 3. When no root is configured, local-client ready-instance discovery and
///    then owner API discovery.
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
    let mut ready_instances = None;
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
                let instances = ready_instances.get_or_insert_with(discover_ready_pumas_instances);
                if let Some(client) = connect_local_client(instances, Some(path)).await {
                    selector_access =
                        Some(Arc::new(PumasSelectorAccess::LocalClient(Arc::new(client))));
                    break;
                }
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

    if selector_access.is_none() && raw_candidates.is_empty() {
        let instances = ready_instances.get_or_insert_with(discover_ready_pumas_instances);
        if let Some(client) = connect_local_client(instances, None).await {
            selector_access = Some(Arc::new(PumasSelectorAccess::LocalClient(Arc::new(client))));
        }
    }

    if api.is_none() && selector_access.is_none() && raw_candidates.is_empty() {
        log::info!(
            "No pumas-library path configured. \
             Set PUMAS_LIBRARY_PATH or pass a path to setup_extensions_with_path()."
        );
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
fn discover_ready_pumas_instances() -> Vec<pumas_library::registry::InstanceEntry> {
    pumas_library::PumasLocalClient::discover_ready_instances().unwrap_or_else(|error| {
        log::info!("PumasLocalClient discovery unavailable: {}", error);
        Vec::new()
    })
}

#[cfg(feature = "model-library")]
async fn connect_local_client(
    instances: &[pumas_library::registry::InstanceEntry],
    launcher_root: Option<&Path>,
) -> Option<pumas_library::PumasLocalClient> {
    let canonical_root = match launcher_root {
        Some(root) => Some(std::fs::canonicalize(root).ok()?),
        None => None,
    };
    for instance in instances {
        if let Some(root) = &canonical_root {
            if std::fs::canonicalize(&instance.library_path).ok().as_ref() != Some(root) {
                continue;
            }
        }
        match pumas_library::PumasLocalClient::connect(instance.clone()).await {
            Ok(client) => {
                log::info!(
                    "Pumas selector access connected as local client at {:?}",
                    client.instance().library_path
                );
                return Some(client);
            }
            Err(error) => log::warn!("PumasLocalClient connect failed: {}", error),
        }
    }
    None
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
    use node_engine::extension_keys;
    use pumas_library::registry::InstanceEntry;
    use pumas_library::ModelIndex;
    use tempfile::TempDir;

    async fn create_owner(root: &Path) -> pumas_library::PumasApi {
        pumas_library::PumasApi::builder(root)
            .auto_create_dirs(true)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .expect("isolated public Pumas owner should start")
    }

    fn owner_instance(owner: &pumas_library::PumasApi) -> InstanceEntry {
        let root = std::fs::canonicalize(owner.launcher_root()).unwrap();
        pumas_library::PumasLocalClient::discover_ready_instances()
            .expect("public owner registry discovery")
            .into_iter()
            .find(|instance| {
                std::fs::canonicalize(&instance.library_path).ok().as_ref() == Some(&root)
            })
            .expect("owner should publish an authenticated ready instance")
    }

    async fn import_fixture_model(owner: &pumas_library::PumasApi) -> String {
        let source = owner.launcher_root().join("source.gguf");
        let mut bytes = [0_u8; 24];
        bytes[..4].copy_from_slice(b"GGUF");
        bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());
        std::fs::write(&source, bytes).unwrap();
        let imported = owner
            .import_model(&pumas_library::model_library::ModelImportSpec {
                path: source.display().to_string(),
                family: "fixture".into(),
                official_name: "Local Client Test".into(),
                repo_id: None,
                model_type: Some("llm".into()),
                subtype: None,
                tags: None,
                security_acknowledged: Some(true),
            })
            .await
            .expect("public fixture import");
        assert!(imported.success, "import failed: {:?}", imported.error);
        imported.model_id.expect("imported model identity")
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
    async fn configured_setup_attaches_to_real_owner_before_read_only_fallback() {
        let temp = create_launcher_root();
        let owner = pumas_library::PumasApi::builder(temp.path())
            .auto_create_dirs(true)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .expect("isolated Pumas owner should start");
        let build_dir = temp.path().join("rust/target/release");
        std::fs::create_dir_all(&build_dir).unwrap();
        let mut extensions = ExecutorExtensions::new();

        setup_extensions_with_path(&mut extensions, Some(&build_dir)).await;

        let access = extensions
            .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
            .expect("existing owner should provide selector access");
        let PumasSelectorAccess::LocalClient(client) = access.as_ref() else {
            panic!("expected local-client access, got {}", access.role_name());
        };
        assert_eq!(
            std::fs::canonicalize(&client.instance().library_path).unwrap(),
            std::fs::canonicalize(temp.path()).unwrap()
        );
        assert!(extensions
            .get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
            .is_none());
        let request = pumas_library::models::ModelLibrarySelectorSnapshotRequest {
            limit: Some(1),
            ..Default::default()
        };
        let owner_snapshot = owner
            .model_library_selector_snapshot(request.clone())
            .await
            .expect("owner selector snapshot");
        let client_snapshot = access
            .model_library_selector_snapshot(request.clone())
            .await
            .expect("authenticated selector snapshot over real owner IPC");
        assert_eq!(client_snapshot.cursor, owner_snapshot.cursor);
        assert_eq!(client_snapshot.total_count, owner_snapshot.total_count);
        assert!(client_snapshot.rows.is_empty());

        let instance = client.instance().clone();
        let mut invalid_instance = instance.clone();
        invalid_instance.connection_token = Some("invalid-owner-token".to_string());
        let invalid_client = pumas_library::PumasLocalClient::connect(invalid_instance)
            .await
            .expect("TCP connection does not authenticate owner methods");
        let error = invalid_client
            .model_library_selector_snapshot(request.clone())
            .await
            .expect_err("owner must reject an invalid local-client token");
        assert!(
            matches!(
                &error,
                pumas_library::PumasError::InvalidParams { message }
                    if message == "Invalid local IPC parameters"
            ),
            "invalid token must return the public IPC authentication error: {error}"
        );

        drop(extensions);
        owner
            .model_library_selector_snapshot(request.clone())
            .await
            .expect("dropping attached access must not shut down the owner");
        pumas_library::PumasLocalClient::connect(instance)
            .await
            .expect("owner must still accept clients")
            .model_library_selector_snapshot(request)
            .await
            .expect("owner IPC must survive dropping attached extensions");
    }

    #[tokio::test]
    async fn configured_local_client_full_facts_and_guarded_target_match_real_owner() {
        use pumas_library::models::{
            PumasArtifactConsumer, PumasArtifactLoadTargetResolutionMode,
            ResolveModelArtifactLoadTargetRequest,
        };
        let root = create_launcher_root();
        let owner = pumas_library::PumasApi::builder(root.path())
            .auto_create_dirs(true)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .expect("isolated real owner");
        let source = root.path().join("source.gguf");
        let mut bytes = [0_u8; 24];
        bytes[..4].copy_from_slice(b"GGUF");
        bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());
        std::fs::write(&source, bytes).unwrap();
        let imported = owner
            .import_model(&pumas_library::model_library::ModelImportSpec {
                path: source.display().to_string(),
                family: "fixture".into(),
                official_name: "Consumer Full Facts".into(),
                repo_id: None,
                model_type: Some("llm".into()),
                subtype: None,
                tags: None,
                security_acknowledged: Some(true),
            })
            .await
            .unwrap();
        assert!(imported.success, "import failed: {:?}", imported.error);
        let model_id = imported.model_id.unwrap();
        let expected = owner.resolve_model_package_facts(&model_id).await.unwrap();
        assert_eq!(expected.package_facts_contract_version, 3);
        assert!(expected.gguf.is_some());
        assert!(expected.inspection_manifest.is_some());
        assert!(expected.artifact.logical_size.is_some());
        let mut extensions = ExecutorExtensions::new();
        setup_extensions_with_path(&mut extensions, Some(root.path())).await;
        let access = extensions
            .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
            .unwrap();
        let PumasSelectorAccess::LocalClient(client) = access.as_ref() else {
            panic!("real owner must be attached as a client")
        };
        assert!(extensions
            .get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
            .is_none());
        let consumer_id = format!("pumas://models/{model_id}");
        let actual = access
            .resolve_model_package_facts(&consumer_id)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        let mut request = ResolveModelArtifactLoadTargetRequest {
            model_ref: expected.model_ref.clone(),
            expected_artifact_kind: None,
            caller_observed_entry_path: None,
            caller_observed_package_facts_contract_version: Some(3),
            resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
            consumer: PumasArtifactConsumer {
                consumer_name: "pantograph-consumer-test".into(),
                task_kind: Some("text_generation".into()),
                runtime_family: Some("llamacpp".into()),
            },
        };
        let owner_target = owner
            .resolve_model_artifact_load_target(request.clone())
            .await
            .unwrap();
        request.model_ref.model_id = consumer_id;
        let client_target = access
            .resolve_model_artifact_load_target(request)
            .await
            .unwrap();
        assert_eq!(client_target.artifact_state, owner_target.artifact_state);
        assert_eq!(
            client_target.entry_path_state,
            owner_target.entry_path_state
        );
        assert_eq!(client_target.target, owner_target.target);
        assert_eq!(
            client_target
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            owner_target
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>()
        );
        // Public local import has no explicit selected artifact identity. Preserve
        // the owner's admission failure instead of inventing one from a filename.
        assert_eq!(expected.model_ref.selected_artifact_id, None);
        assert!(!client_target.is_ready());
        assert!(client_target.target.is_none());
        assert!(!client_target.diagnostics.is_empty());
        assert!(client_target
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.field_path.is_none()
                && diagnostic.message == "Artifact load target is not available"));
        let mut wrong = client.instance().clone();
        wrong.connection_token = Some("wrong-owner-token".into());
        let wrong = pumas_library::PumasLocalClient::connect(wrong)
            .await
            .unwrap();
        assert!(matches!(
            wrong.resolve_model_package_facts(&model_id).await,
            Err(pumas_library::PumasError::InvalidParams { .. })
        ));
        drop(extensions);
        owner.resolve_model_package_facts(&model_id).await.unwrap();
    }

    #[tokio::test]
    async fn configured_local_client_skips_other_libraries_and_survives_client_drop() {
        let expected_root = create_launcher_root();
        let other_root = create_launcher_root();
        let expected_owner = create_owner(expected_root.path()).await;
        let other_owner = create_owner(other_root.path()).await;
        let instances = [
            owner_instance(&other_owner),
            owner_instance(&expected_owner),
        ];

        let client = connect_local_client(&instances, Some(expected_root.path()))
            .await
            .expect("matching owner should connect");
        assert_eq!(client.instance().library_path, expected_root.path());
        let access = PumasSelectorAccess::LocalClient(Arc::new(client));
        let snapshot = access
            .model_library_selector_snapshot(Default::default())
            .await
            .expect("authenticated matching owner snapshot");
        assert_eq!(
            snapshot,
            expected_owner
                .model_library_selector_snapshot(Default::default())
                .await
                .unwrap()
        );
        drop(access);

        connect_local_client(&instances, Some(expected_root.path()))
            .await
            .expect("attached client must not own server shutdown")
            .model_library_selector_snapshot(Default::default())
            .await
            .expect("owner IPC should remain available");
    }

    #[tokio::test]
    async fn configured_local_client_does_not_fall_back_to_another_owner() {
        let expected_root = create_launcher_root();
        let other_root = create_launcher_root();
        let expected_owner = create_owner(expected_root.path()).await;
        let other_owner = create_owner(other_root.path()).await;
        let other = owner_instance(&other_owner);
        let mut unavailable = owner_instance(&expected_owner);
        unavailable.connection_token = None;
        let instances = [unavailable, other];

        assert!(connect_local_client(&instances, Some(expected_root.path()))
            .await
            .is_none());
        assert!(
            connect_local_client(&instances, Some(&expected_root.path().join("missing")))
                .await
                .is_none()
        );
        assert!(connect_local_client(&instances, None).await.is_some());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn configured_local_client_matches_canonical_launcher_root() {
        let root = create_launcher_root();
        let alias_parent = TempDir::new().unwrap();
        let alias = alias_parent.path().join("launcher-alias");
        std::os::unix::fs::symlink(root.path(), &alias).unwrap();
        let owner = create_owner(root.path()).await;
        let instance = owner_instance(&owner);

        let client = connect_local_client(&[instance], Some(&alias))
            .await
            .expect("canonical launcher identity should accept an alias");
        assert_eq!(client.instance().library_path, root.path());
    }

    #[tokio::test]
    async fn local_client_update_feed_recovers_events_from_selector_cursor() {
        let root = create_launcher_root();
        let owner = create_owner(root.path()).await;
        let cursor = owner
            .model_library_selector_snapshot(Default::default())
            .await
            .unwrap()
            .cursor;
        let model_id = import_fixture_model(&owner).await;
        let expected = owner
            .list_model_library_updates_since(Some(&cursor), 100)
            .await
            .unwrap();
        assert!(expected
            .events
            .iter()
            .any(|event| event.model_id == model_id));
        let client = pumas_library::PumasLocalClient::connect(owner_instance(&owner))
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
        assert!(feed.events.iter().any(|event| event.model_id == model_id));
        for event in expected.events {
            assert!(
                feed.events.contains(&event),
                "owner update must survive IPC recovery"
            );
        }
    }

    #[tokio::test]
    async fn local_client_selected_model_detail_uses_batch_detail_methods() {
        let root = create_launcher_root();
        let owner = create_owner(root.path()).await;
        let model_id = import_fixture_model(&owner).await;
        // Import publishes the model before the owner's background index work
        // necessarily settles. Await a forced pass before testing cache reuse;
        // only the documented in-flight conflict is eligible for retry.
        let indexed_count = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                match owner.rebuild_model_index().await {
                    Ok(count) => break count,
                    Err(pumas_library::PumasError::ModelIndexRefreshInProgress) => {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                    Err(error) => panic!("fixture index reconciliation failed: {error}"),
                }
            }
        })
        .await
        .expect("fixture index reconciliation must settle");
        assert_eq!(
            indexed_count, 1,
            "the fixture must retain its imported model"
        );
        let expected_descriptor = owner
            .resolve_model_execution_descriptors_batch(vec![model_id.clone()])
            .await
            .unwrap()
            .into_iter()
            .find(|item| item.model_id == model_id)
            .and_then(|item| item.descriptor)
            .expect("owner should resolve an imported GGUF descriptor");
        let expected_row = owner
            .model_library_selector_snapshot(Default::default())
            .await
            .unwrap()
            .rows
            .into_iter()
            .find(|row| row.model_id == model_id)
            .expect("imported selector row before detail hydration");
        let client = pumas_library::PumasLocalClient::connect(owner_instance(&owner))
            .await
            .unwrap();
        let access = PumasSelectorAccess::LocalClient(Arc::new(client));

        let detail = access
            .selected_model_detail(&model_id)
            .await
            .expect("local client selected detail should load from batch APIs");

        assert_eq!(
            serde_json::to_value(detail.selector_row.expect("selector row")).unwrap(),
            serde_json::to_value(expected_row).unwrap()
        );
        assert_eq!(
            serde_json::to_value(detail.descriptor.expect("descriptor should hydrate")).unwrap(),
            serde_json::to_value(expected_descriptor).unwrap()
        );
        // The selected-detail call hydrates a cold summary over IPC. A later
        // owner lookup observes the populated cache, so freshness must change
        // while the complete model identity and summary payload stay equal.
        let cold_summary = detail
            .package_summary_result
            .expect("summary should hydrate");
        let warm_summary = owner
            .resolve_model_package_facts_summaries(vec![model_id.clone()])
            .await
            .unwrap()
            .into_iter()
            .find(|item| item.model_id == model_id)
            .and_then(|item| item.result)
            .expect("owner should resolve the hydrated GGUF summary");
        assert_eq!(
            cold_summary.status,
            pumas_library::models::ModelPackageFactsSummaryStatus::Regenerated
        );
        assert_eq!(
            warm_summary.status,
            pumas_library::models::ModelPackageFactsSummaryStatus::Fresh
        );
        assert_eq!(cold_summary.model_id, model_id);
        assert_eq!(warm_summary.model_id, model_id);
        assert_eq!(
            serde_json::to_value(cold_summary.summary.expect("cold summary payload")).unwrap(),
            serde_json::to_value(warm_summary.summary.as_ref().expect("warm summary payload"))
                .unwrap()
        );
        // Observe the producer-owned cache without reconciling or mutating it.
        // Equal summary payloads alone do not prove equal source fingerprints.
        let model_root = owner.launcher_root().join("shared-resources/models");
        let index = ModelIndex::open_read_only(model_root.join("models.db")).unwrap();
        let selected_artifact_id = warm_summary
            .summary
            .as_ref()
            .unwrap()
            .model_ref
            .selected_artifact_id
            .as_deref();
        let cache_before = index
            .get_model_package_facts_cache(
                &model_id,
                selected_artifact_id,
                pumas_library::index::ModelPackageFactsCacheScope::Summary,
            )
            .unwrap()
            .expect("warm owner summary must have a durable cache row");
        let metadata_path = model_root.join(&model_id).join("metadata.json");
        let metadata_before = std::fs::read_to_string(&metadata_path).unwrap();
        let warm_client_summary = access
            .resolve_model_package_facts_summary(&model_id)
            .await
            .expect("client should observe the same warm summary");
        let cache_after = index
            .get_model_package_facts_cache(
                &model_id,
                selected_artifact_id,
                pumas_library::index::ModelPackageFactsCacheScope::Summary,
            )
            .unwrap();
        let metadata_after = std::fs::read_to_string(&metadata_path).unwrap();
        assert_eq!(
            serde_json::to_value(warm_client_summary).unwrap(),
            serde_json::to_value(warm_summary).unwrap(),
            "warm IPC must preserve freshness; cache before={cache_before:?}, \
             cache after={cache_after:?}, metadata before={metadata_before}, \
             metadata after={metadata_after}"
        );
        assert_eq!(cache_after.as_ref(), Some(&cache_before));
        assert_eq!(metadata_after, metadata_before);
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
