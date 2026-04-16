//! Shared runtime construction helpers for Tauri workflow transport.
//!
//! These helpers compose host resources into the backend-owned embedded runtime
//! without coupling that wiring to one specific command surface.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_embedded_runtime::{
    EmbeddedRuntime, EmbeddedRuntimeConfig, HostRuntimeModeSnapshot, RagBackend, RagDocument,
};
use tauri::{AppHandle, Manager};

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
use crate::project_root::resolve_project_root;

use super::commands::{SharedExtensions, SharedWorkflowService};

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data dir: {error}"))
}

struct TauriRagBackend {
    rag_manager: SharedRagManager,
}

#[async_trait]
impl RagBackend for TauriRagBackend {
    async fn search_as_docs(&self, query: &str, limit: usize) -> Result<Vec<RagDocument>, String> {
        let guard = self.rag_manager.read().await;
        let docs = guard
            .search_as_docs(query, limit)
            .await
            .map_err(|error| error.to_string())?;
        Ok(docs
            .into_iter()
            .map(|doc| RagDocument {
                id: doc.id,
                title: doc.title,
                section: doc.section,
                summary: doc.summary,
                content: doc.content,
            })
            .collect())
    }
}

pub(crate) async fn build_runtime(
    app: &AppHandle,
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    extensions: &SharedExtensions,
    workflow_service: &SharedWorkflowService,
    rag_manager: Option<&SharedRagManager>,
) -> Result<EmbeddedRuntime, String> {
    let config = EmbeddedRuntimeConfig::new(app_data_dir(app)?, resolve_project_root()?);
    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let rag_backend = rag_manager.cloned().map(|manager| {
        Arc::new(TauriRagBackend {
            rag_manager: manager,
        }) as Arc<dyn RagBackend>
    });

    Ok(EmbeddedRuntime::hosted_with_default_python_runtime(
        config,
        gateway.inner_arc(),
        extensions.clone(),
        workflow_service.clone(),
        rag_backend,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await)
}
