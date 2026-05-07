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
    use node_engine::extension_keys;
    use pumas_library::ModelIndex;
    use tempfile::TempDir;

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
