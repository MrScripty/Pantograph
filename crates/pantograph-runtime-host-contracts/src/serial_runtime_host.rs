//! Explicit in-process serial-owner capability. Default execution ports do not
//! gain this capability or scheduling behavior from a timing label.
use crate::{
    ReservationLifecycleApplication, ReservationLifecycleEvent, ReservationLifecyclePort,
    ReservationLifecyclePortError, RuntimeHostBatchExecutionPort, RuntimeHostBatchExecutionRequest,
    RuntimeHostBatchExecutionResponse, RuntimeHostExecutionCancellationHandle,
    RuntimeHostExecutionPortError,
};
use async_trait::async_trait;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerSerialBoundDispatch, SchedulerSerialDrainedDispatch,
    SchedulerSerialOwnerSnapshot,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[async_trait]
pub trait SerialRuntimeHostBatchExecutionPort:
    RuntimeHostBatchExecutionPort + ReservationLifecyclePort
{
    /// Read-only nonblocking query on the exact admitted CPU model/config. None
    /// means baseline order, not permission to fabricate an owner/drain proof.
    fn resident_serial_cpu_owner(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence>;

    /// Exactly one member. The actual execution owner consumes the bound guard
    /// inside its held backend lease and returns the drain proof only after
    /// actual worker completion. Return collection evidence only from that actually
    /// held owner, rechecked after drain (None keeps baseline order). Expected-owner mismatch refuses before forward;
    /// never downgrade a ranked attempt to unranked execution after Start.
    async fn execute_serial_singleton(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        ownership: SchedulerSerialBoundDispatch,
    ) -> Result<
        (
            RuntimeHostBatchExecutionResponse,
            SchedulerSerialDrainedDispatch,
            Option<SerialRuntimeHostCpuOwnerEvidence>,
        ),
        RuntimeHostExecutionPortError,
    >;

    /// Apply the real owned release/reconciliation. For qualified warm attempts,
    /// release must atomically fence this expected loaded owner or exclude every
    /// relevant producer entry path through cleanup. Failed fencing returns Err;
    /// it cannot acknowledge or stop a replacement producer.
    async fn apply_serial_cleanup(
        &self,
        event: ReservationLifecycleEvent,
        expected_owner: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError>;
}

/// Issued by the constructor-owned CPU owner. Generation reads are atomic and
/// safe under the scheduler store lock; no callback executes at final Start.
#[derive(Clone)]
pub struct SerialRuntimeHostCpuOwnerEvidence {
    pub snapshot: SchedulerSerialOwnerSnapshot,
    pub generation: Arc<AtomicU64>,
}
impl SerialRuntimeHostCpuOwnerEvidence {
    pub fn is_current(&self) -> bool {
        self.snapshot.generation > 0
            && self.snapshot.generation.is_multiple_of(2)
            && self.generation.load(Ordering::Acquire) == self.snapshot.generation
    }
}
