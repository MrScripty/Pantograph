//! Host task executor for Pantograph-specific node types.
//!
//! Only handles node types that require Pantograph host resources
//! (for example RAG search or Python-backed execution). All other nodes are
//! handled by `CoreTaskExecutor` via `CompositeTaskExecutor`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use node_engine::{
    core_executor::resolve_node_type, extension_keys, Context, DependencyState, EventSink,
    ExecutorExtensions, ModelDependencyRequest, ModelDependencyRequirements,
    ModelDependencyResolver, ModelDependencyStatus, NodeEngineError, Result, TaskExecutor,
    WorkflowEvent,
};
use pantograph_runtime_identity::canonical_engine_backend_key;

use crate::python_runtime::{
    ProcessPythonRuntimeAdapter, PythonNodeExecutionRequest, PythonRuntimeAdapter,
    PythonStreamHandler,
};
pub use crate::python_runtime_execution::{
    PythonRuntimeExecutionMetadata, PythonRuntimeExecutionRecorder,
};
use crate::rag::RagBackend;
use crate::runtime_health::failed_runtime_health_assessment;

/// Host task executor that handles only Pantograph host-dependent nodes.
///
/// Currently handles:
/// - `rag-search`: requires an injected `RagBackend`
/// - `pytorch-inference`: python sidecar execution
/// - `diffusion-inference`: python sidecar execution
/// - `audio-generation`: python sidecar execution
/// - `onnx-inference`: python sidecar execution
///
/// All other node types should be handled by `CoreTaskExecutor` via
/// `CompositeTaskExecutor`. Unknown types return the sentinel error
/// that `CompositeTaskExecutor` uses for fallthrough.
pub struct TauriTaskExecutor {
    /// Optional host-provided RAG backend for document search.
    rag_backend: Option<Arc<dyn RagBackend>>,
    /// Host adapter for python-backed nodes (pytorch/diffusion/audio/onnx).
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
}

/// Pantograph-specific extension keys used by host executors.
pub mod runtime_extension_keys {
    /// `Arc<dyn node_engine::EventSink>` for streaming host-side events.
    pub const EVENT_SINK: &str = "pantograph_event_sink";
    /// Execution identifier for host-side stream/progress events.
    pub const EXECUTION_ID: &str = "pantograph_execution_id";
    /// Recorder for Python-backed runtime execution metadata captured during a run.
    pub const PYTHON_RUNTIME_EXECUTION_RECORDER: &str =
        "pantograph_python_runtime_execution_recorder";
}

mod dependency_environment;
mod puma_lib;
mod python_execution;
mod rag_search;
impl TauriTaskExecutor {
    const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV64_PRIME: u64 = 0x0000_0100_0000_01B3;
    const PYTHON_RUNTIME_FAILURE_THRESHOLD: u32 = 3;

    fn canonical_backend_key(value: Option<&str>) -> Option<String> {
        canonical_engine_backend_key(value)
    }

    /// Create a new task executor with the default process Python runtime.
    pub fn new(rag_backend: Option<Arc<dyn RagBackend>>) -> Self {
        Self::with_python_runtime(rag_backend, Arc::new(ProcessPythonRuntimeAdapter))
    }

    /// Create a task executor with a custom python runtime adapter.
    pub fn with_python_runtime(
        rag_backend: Option<Arc<dyn RagBackend>>,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        Self {
            rag_backend,
            python_runtime,
        }
    }
}

#[async_trait]
impl TaskExecutor for TauriTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        extensions: &node_engine::ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);

        match node_type.as_str() {
            "rag-search" => self.execute_rag_search(&inputs).await,
            "puma-lib" => self.execute_puma_lib(&inputs, extensions).await,
            "dependency-environment" => {
                self.execute_dependency_environment(&inputs, extensions)
                    .await
            }
            "pytorch-inference" | "diffusion-inference" | "audio-generation" | "onnx-inference" => {
                self.execute_python_node(task_id, &node_type, &inputs, extensions)
                    .await
            }
            _ => {
                // Signal to CompositeTaskExecutor that this node type
                // requires host-specific executor (i.e., fall through to core)
                Err(NodeEngineError::ExecutionFailed(format!(
                    "Node type '{}' requires host-specific executor",
                    node_type
                )))
            }
        }
    }
}

#[cfg(test)]
#[path = "task_executor_tests.rs"]
mod tests;
