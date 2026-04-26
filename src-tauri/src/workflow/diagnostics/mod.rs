//! Backend-owned diagnostics projection for workflow execution traces.
//!
//! This module replaces the previous TypeScript-side diagnostics accumulator so
//! workflow run state, node lifecycle state, and retained event history are all
//! derived in Rust before the GUI renders them.

mod attempts;
mod overlay;
mod store;
mod timing;
mod trace;
mod types;

pub use store::{
    SharedWorkflowDiagnosticsStore, WorkflowDiagnosticsStore, WorkflowRuntimeSnapshotUpdate,
    WorkflowSchedulerSnapshotUpdate,
};
#[cfg(test)]
pub use store::{WorkflowRuntimeSnapshotRecord, WorkflowSchedulerSnapshotRecord};
pub(crate) use trace::node_engine_workflow_trace_event;
#[allow(unused_imports)]
pub use types::{
    DiagnosticsEventRecord, DiagnosticsNodeStatus, DiagnosticsNodeTrace, DiagnosticsRunStatus,
    DiagnosticsRunTrace, DiagnosticsRuntimeLifecycleSnapshot, DiagnosticsRuntimeSnapshot,
    DiagnosticsSchedulerSnapshot, DiagnosticsTraceRuntimeMetrics, DiagnosticsWorkflowTimingHistory,
    WorkflowDiagnosticsProjection, WorkflowDiagnosticsProjectionContext,
    WorkflowDiagnosticsSnapshotRequest,
};

#[cfg(test)]
mod tests;
