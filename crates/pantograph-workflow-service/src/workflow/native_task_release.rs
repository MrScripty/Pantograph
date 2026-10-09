//! Opt-in sealed task-release acknowledgement. No production provider is added.
use super::WorkflowServiceError;
use inference::gateway::CandleCpuVerifiedTaskRelease;
use pantograph_runtime_host_contracts::RuntimeHostBatchExecutionRequest;
use pantograph_scheduler::SchedulerSerialAttemptIdentity;

/// Synchronous, bounded, non-reentrant owner callbacks. The native port checks
/// dispatch before awaited resolution and acknowledges only after real cleanup
/// returns with all native/registry guards dropped. Refusal holds protection.
pub trait WorkflowNativeTaskReleaseObserver: Send + Sync {
    /// Record a historical producer fact. It cannot mint result acceptance.
    fn observe_task_output(
        &self,
        _proof: &inference::gateway::CandleCpuVerifiedTaskOutput<'_>,
    ) -> Result<(), WorkflowServiceError> {
        // Release-only observers remain compatible. A no-op never gives the
        // same-store result owner a producer fingerprint or acceptance latch.
        Ok(())
    }
    fn check_dispatch(
        &self,
        request: &RuntimeHostBatchExecutionRequest,
        identity: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError>;
    fn acknowledge_task_release(
        &self,
        proof: &CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError>;
}
#[cfg(any(test, feature = "test-support"))]
#[path = "native_task_release_test_support.rs"]
mod test_support;
#[cfg(any(test, feature = "test-support"))]
pub use test_support::WorkflowControlledNativeRelease;

pub(crate) fn identity_is_bounded(id: SchedulerSerialAttemptIdentity<'_>) -> bool {
    [
        id.workflow_id,
        id.workflow_run_id,
        id.node_id,
        id.task_id,
        id.attempt_id,
        id.execution_request_id,
        id.candidate_id,
    ]
    .iter()
    .all(|s| !s.is_empty() && s.len() <= 128)
        && !id.reservation_lease_id.is_empty()
        && id.reservation_lease_id.len() <= 64
}
