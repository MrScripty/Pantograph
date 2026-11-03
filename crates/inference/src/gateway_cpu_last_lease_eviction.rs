//! Explicit drained-success to supervised actual last-lease stop handoff.
use super::*;

impl CandleCpuDrainedAttempt {
    /// Consume this custody-bearing actual success/drain receipt to evict its
    /// sole ephemeral lease. Retaining successors/pins refuse before stop.
    /// Cancellation before the irreversible boundary leaves execution fenced;
    /// after begin, supervision completes stop and ACK even if collector drops.
    /// This never recovers abandoned execution or accepts replacement snapshots.
    pub async fn evict_last(
        self,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<RuntimeRetentionDisposition, GatewayError> {
        let mut binding = self.binding;
        let gateway = binding.gateway.clone();
        if binding.custody.is_none() || binding.residency.is_some() {
            return Err(refusal());
        }
        gateway.embedding_replacement.drain().await?;
        let backend = gateway.backend.clone().write_owned().await;
        binding.current(backend.as_ref())?;
        let lifecycle = gateway.runtime_lifecycle.read().await;
        let config = gateway.current_runtime_config.read().await;
        if canonical_backend_key(backend.name()) != "candle"
            || !backend.is_ready()
            || !lifecycle.active
            || lifecycle.runtime_id.as_deref() != Some("candle")
            || lifecycle.runtime_instance_id != binding.runtime_instance_id
            || config.as_ref().and_then(config_model_target).as_deref()
                != Some(binding.model_target.as_str())
            || config.as_ref().and_then(|c| c.model_name.as_deref())
                != Some(binding.owner.model_ref().model_id.as_str())
        {
            return Err(refusal());
        }
        binding.current(backend.as_ref())?;
        reject_cancelled_execution_handle("native CPU eviction", &host)?;
        let ticket = binding
            .custody
            .take()
            .ok_or_else(refusal)?
            .begin_last_lease_eviction(RuntimeRetainedOwnerIdentity {
                runtime_id: "candle",
                source_id: &gateway.resident_source_id,
                runtime_instance_id: lifecycle
                    .runtime_instance_id
                    .as_deref()
                    .ok_or_else(refusal)?,
                model_target: &binding.model_target,
            })
            .map_err(|e| GatewayError::SwitchFailed(e.to_string()))?;
        drop(config);
        drop(lifecycle);
        Ok(
            stop_last_lease_owned(gateway, backend, ticket, binding.owner, host)
                .await?
                .disposition,
        )
    }
}

/// Same held writer and original registry stop ticket for task or idle guard.
/// The physical interval excludes writer admission, hooks and registry ACK.
pub(in crate::gateway) async fn stop_last_lease_owned(
    gateway: Arc<InferenceGateway>,
    mut backend: BackendGuard,
    ticket: pantograph_runtime_registry::RuntimeLastLeaseEviction,
    owner: CandleCpuCleanupOwner,
    host: InferenceExecutionCancellationHandle,
) -> Result<CandleCpuResidencyStopOutcome, GatewayError> {
    let (signal, mut cancel) = caller_signal(host);
    // No await between synchronous stop admission and supervisor ownership.
    let task = tokio::spawn(async move {
        #[cfg(test)]
        eviction_test_phase(&gateway, "eviction_before_stop").await?;
        reject_cancelled_execution_handle("native CPU eviction before stop", &signal)?;
        let instance = backend
            .resident_cpu_calibration_instance()
            .ok_or_else(refusal)?;
        if !gateway
            .candle_cpu_calibration
            .as_ref()
            .is_some_and(|calibration| calibration.matches_cleanup_owner(&owner, instance))
        {
            return Err(refusal());
        }
        // Cancellation becomes advisory at this physical irreversible boundary.
        let started = std::time::Instant::now();
        backend.stop().await?;
        let stop_duration = started.elapsed();
        if backend.is_ready() || backend.resident_cpu_calibration_instance().is_some() {
            return Err(refusal());
        }
        #[cfg(test)]
        eviction_test_phase(&gateway, "eviction_after_stop").await?;
        *gateway.embedding_mode.write().await = false;
        *gateway.reranking_mode.write().await = false;
        *gateway.external_mode.write().await = false;
        *gateway.current_runtime_config.write().await = None;
        let mut lifecycle = gateway.runtime_lifecycle.write().await;
        lifecycle.active = false;
        lifecycle.lifecycle_decision_reason = Some("native_last_lease_evicted".into());
        lifecycle.last_error = None;
        if lifecycle.warmup_completed_at_ms.is_none() {
            lifecycle.warmup_started_at_ms = None;
            lifecycle.warmup_timing_attempt_id = None;
            lifecycle.warmup_duration_ms = None;
        }
        // The producer sequence follows physical stop under this same writer.
        let sequence = cpu_residency::next_observation_sequence(&gateway)?;
        let acknowledged = std::time::Instant::now();
        let disposition = ticket
            .acknowledge_stopped(sequence)
            .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?;
        let acknowledgement_duration = acknowledged.elapsed();
        drop(lifecycle);
        drop(backend);
        Ok(CandleCpuResidencyStopOutcome {
            disposition,
            stop_duration,
            acknowledgement_duration,
        })
    });
    let result = task.await.map_err(|error| {
        GatewayError::SwitchFailed(format!("native CPU eviction supervision failed: {error}"))
    })??;
    cancel.disarm();
    Ok(result)
}

#[cfg(test)]
async fn eviction_test_phase(
    gateway: &InferenceGateway,
    phase: &'static str,
) -> Result<(), GatewayError> {
    let owner = gateway
        .candle_cpu_calibration
        .as_ref()
        .ok_or_else(refusal)?
        .clone();
    tokio::task::spawn_blocking(move || owner.test_attempt_phase(phase))
        .await
        .map_err(|_| refusal())
}
