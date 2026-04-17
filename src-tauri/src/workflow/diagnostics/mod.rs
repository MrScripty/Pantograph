//! Backend-owned diagnostics projection for workflow execution traces.
//!
//! This module replaces the previous TypeScript-side diagnostics accumulator so
//! workflow run state, node lifecycle state, and retained event history are all
//! derived in Rust before the GUI renders them.

mod store;
mod trace;
mod types;

pub use store::{SharedWorkflowDiagnosticsStore, WorkflowDiagnosticsStore};
pub(crate) use trace::node_engine_workflow_trace_event;
#[allow(unused_imports)]
pub use types::{
    DiagnosticsEventRecord, DiagnosticsNodeStatus, DiagnosticsNodeTrace, DiagnosticsRunStatus,
    DiagnosticsRunTrace, DiagnosticsRuntimeLifecycleSnapshot, DiagnosticsRuntimeSnapshot,
    DiagnosticsSchedulerSnapshot, DiagnosticsTraceRuntimeMetrics, WorkflowDiagnosticsProjection,
    WorkflowDiagnosticsSnapshotRequest,
};

#[cfg(test)]
mod tests;
