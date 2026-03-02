//! Extensions setup for host applications.
//!
//! Hosts call [`setup_extensions`] at startup to initialize optional runtime
//! dependencies (like `PumasApi`) in the shared `ExecutorExtensions`. This
//! keeps host crates decoupled from the underlying libraries — they don't
//! need to import `pumas-library` directly.

use node_engine::ExecutorExtensions;

/// Initialize optional runtime dependencies in `ExecutorExtensions`.
///
/// Currently handles:
/// - **PumasApi** (`model-library` feature): Tries explicit/local paths first
///   (`library_path`, then `PUMAS_LIBRARY_PATH`) and falls back to
///   `PumasApi::discover()` (global registry at `~/.config/pumas/registry.db`).
///
/// # Example
///
/// ```ignore
/// let mut extensions = node_engine::ExecutorExtensions::new();
/// workflow_nodes::setup_extensions(&mut extensions).await;
/// // extensions now has PumasApi (if available)
/// ```
#[cfg(feature = "model-library")]
pub async fn setup_extensions(extensions: &mut ExecutorExtensions) {
    setup_extensions_with_path(extensions, None).await;
}

/// Initialize extensions with an explicit library path fallback.
///
/// Tries in order:
/// 1. `library_path` parameter (if provided)
/// 2. `PUMAS_LIBRARY_PATH` environment variable
/// 3. `PumasApi::discover()` (global registry)
#[cfg(feature = "model-library")]
pub async fn setup_extensions_with_path(
    extensions: &mut ExecutorExtensions,
    library_path: Option<&std::path::Path>,
) {
    use std::sync::Arc;

    fn is_launcher_root(path: &std::path::Path) -> bool {
        path.join("shared-resources").exists() && path.join("launcher-data").exists()
    }

    fn push_unique(
        out: &mut Vec<std::path::PathBuf>,
        seen: &mut std::collections::HashSet<std::path::PathBuf>,
        path: std::path::PathBuf,
    ) {
        if seen.insert(path.clone()) {
            out.push(path);
        }
    }

    // Accept either launcher root paths or build output dirs like:
    // <repo>/rust/target/release (or debug) by deriving <repo>.
    fn expand_candidate_path(path: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();

        push_unique(&mut out, &mut seen, path.to_path_buf());

        // If user points at the pumas release/debug binary dir, derive launcher root.
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
                    push_unique(&mut out, &mut seen, root.to_path_buf());
                }
            }
        }

        // Generic fallback: walk ancestors to find a valid launcher root.
        for ancestor in path.ancestors() {
            if is_launcher_root(ancestor) {
                push_unique(&mut out, &mut seen, ancestor.to_path_buf());
            }
        }

        out
    }

    // Build candidate paths: explicit parameter first, then env var
    let mut raw_candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(p) = library_path {
        raw_candidates.push(p.to_path_buf());
    }
    if let Ok(env_path) = std::env::var("PUMAS_LIBRARY_PATH") {
        raw_candidates.push(std::path::PathBuf::from(env_path));
    }

    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in &raw_candidates {
        for expanded in expand_candidate_path(raw) {
            push_unique(&mut candidates, &mut seen, expanded);
        }
    }

    let mut api = None;
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
                api = Some(found);
                break;
            }
            Err(e) => {
                log::warn!("PumasApi::builder({:?}) failed: {}", path, e);
            }
        }
    }

    if api.is_none() {
        if raw_candidates.is_empty() {
            log::info!(
                "No pumas-library path configured. \
                 Set PUMAS_LIBRARY_PATH or pass a path to setup_extensions_with_path()."
            );
        }
        match pumas_library::PumasApi::discover().await {
            Ok(found) => {
                log::info!("PumasApi connected via discover()");
                api = Some(found);
            }
            Err(e) => {
                log::info!("PumasApi discover() unavailable: {}", e);
            }
        }
    }

    if let Some(api) = api {
        extensions.set(node_engine::extension_keys::PUMAS_API, Arc::new(api));
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
