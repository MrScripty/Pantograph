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
/// - **PumasApi** (`model-library` feature): Tries `PumasApi::discover()` first
///   (uses the global registry at `~/.config/pumas/registry.db`). If that fails,
///   falls back to `PUMAS_LIBRARY_PATH` environment variable and opens the library
///   directly via `PumasApi::new()`.
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
/// 1. `PumasApi::discover()` (global registry)
/// 2. `library_path` parameter (if provided)
/// 3. `PUMAS_LIBRARY_PATH` environment variable
#[cfg(feature = "model-library")]
pub async fn setup_extensions_with_path(
    extensions: &mut ExecutorExtensions,
    library_path: Option<&std::path::Path>,
) {
    use std::sync::Arc;

    // Try global registry discovery first
    let api = match pumas_library::PumasApi::discover().await {
        Ok(api) => {
            log::info!("PumasApi connected via discover()");
            Some(api)
        }
        Err(e) => {
            log::info!("PumasApi discover() unavailable: {}", e);

            // Build candidate paths: explicit parameter first, then env var
            let mut candidates: Vec<std::path::PathBuf> = Vec::new();
            if let Some(p) = library_path {
                candidates.push(p.to_path_buf());
            }
            if let Ok(env_path) = std::env::var("PUMAS_LIBRARY_PATH") {
                candidates.push(std::path::PathBuf::from(env_path));
            }

            let mut result = None;
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
                    Ok(api) => {
                        log::info!("PumasApi initialized from {:?}", path);
                        result = Some(api);
                        break;
                    }
                    Err(e2) => {
                        log::warn!("PumasApi::builder({:?}) failed: {}", path, e2);
                    }
                }
            }

            if result.is_none() && candidates.is_empty() {
                log::info!(
                    "No pumas-library path configured. \
                     Set PUMAS_LIBRARY_PATH or pass a path to setup_extensions_with_path()."
                );
            }

            result
        }
    };

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
) {}
