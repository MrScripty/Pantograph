//! Allocation evidence sampled from the existing gateway lifecycle owner.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentAllocationState {
    Resident,
    Released,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ResidentLifecycleSnapshot {
    pub source_id: String,
    pub sequence: u64,
    pub allocation_state: ResidentAllocationState,
    pub model_target: Option<String>,
    pub lifecycle: crate::RuntimeLifecycleSnapshot,
}

impl ResidentLifecycleSnapshot {
    /// Publish owner facts; registry startup configuration supplies the estimates.
    /// No task envelope is reduced and no allocator measurement is inferred.
    pub fn publish(
        self,
        registry: &pantograph_runtime_registry::RuntimeRegistry,
    ) -> Result<
        pantograph_runtime_registry::RuntimeRegistryRuntimeSnapshot,
        pantograph_runtime_registry::RuntimeRegistryError,
    > {
        use pantograph_runtime_registry::{
            observed_runtime_status_from_lifecycle, RuntimeObservation,
            RuntimeProducerAllocationState, RuntimeProducerObservation,
        };
        let allocation_state = match self.allocation_state {
            ResidentAllocationState::Resident => RuntimeProducerAllocationState::Resident,
            ResidentAllocationState::Released => RuntimeProducerAllocationState::Released,
            ResidentAllocationState::Unknown => RuntimeProducerAllocationState::Unknown,
        };
        let lifecycle = self.lifecycle;
        let status = observed_runtime_status_from_lifecycle(
            lifecycle.active,
            lifecycle.warmup_started_at_ms,
            lifecycle.warmup_completed_at_ms,
            lifecycle.last_error.is_some(),
        );
        registry.observe_runtime_producer(RuntimeProducerObservation {
            source_id: self.source_id,
            sequence: self.sequence,
            allocation_state,
            observation: RuntimeObservation {
                runtime_id: lifecycle.runtime_id.unwrap_or_else(|| "pytorch".into()),
                display_name: "PyTorch".into(),
                backend_keys: vec!["pytorch".into()],
                model_id: self.model_target,
                runtime_instance_id: lifecycle.runtime_instance_id,
                status,
                last_error: lifecycle.last_error,
            },
        })
    }
}
