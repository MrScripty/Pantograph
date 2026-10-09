//! Immediate native stop acknowledgement under the actual backend writer.
use super::*;
use pantograph_runtime_registry::{
    RuntimeObservation, RuntimeProducerAllocationState, RuntimeProducerObservation,
    RuntimeRegistry, RuntimeRegistryStatus, RuntimeRetainedOwnerIdentity,
};

impl InferenceGateway {
    /// Stop the actual producer and retire a configured CPU resident envelope
    /// only with an exact built-in native receipt and accepted registry apply.
    /// No Released snapshot or reusable receipt escapes the held writer.
    /// Cancellation before ACK remains charged; after ACK, release is real even
    /// if later metadata cleanup or the collector is cancelled.
    pub async fn stop_and_publish_cpu_retirement(
        &self,
        registry: &RuntimeRegistry,
    ) -> Result<(), GatewayError> {
        self.embedding_replacement.drain().await?;
        let mut backend = self.backend.write().await;
        if canonical_backend_key(backend.name()) != "candle"
            || self.candle_cpu_calibration.is_none()
        {
            if let Err(error) = backend.stop().await {
                self.runtime_lifecycle.write().await.last_error = Some(error.to_string());
                return Err(error.into());
            }
            self.record_resident_release(&runtime_id_for_backend_name(backend.name()), true);
            self.finish_stopped_metadata().await;
            return Ok(());
        }

        let expected = backend
            .resident_cpu_calibration_instance()
            .and_then(|instance| {
                self.candle_cpu_calibration
                    .as_ref()?
                    .cleanup_owner(instance)
            });
        let lifecycle = self.runtime_lifecycle.read().await;
        let config = self.current_runtime_config.read().await;
        let identity = expected.as_ref().and_then(|owner| {
            (lifecycle.active
                && lifecycle.runtime_id.as_deref() == Some("candle")
                && config.as_ref()?.model_name.as_deref()
                    == Some(owner.model_ref().model_id.as_str()))
            .then(|| {
                Some((
                    lifecycle.runtime_instance_id.clone()?,
                    config.as_ref().and_then(config_model_target)?,
                ))
            })?
        });
        drop(config);
        drop(lifecycle);
        let proof = match backend.stop_with_cpu_retirement().await {
            Ok(proof) => proof,
            Err(error) => {
                self.runtime_lifecycle.write().await.last_error = Some(error.to_string());
                return Err(error.into());
            }
        };

        // Synchronous proof comparison, checked sequence and registry transaction:
        // no await/callback/delivery gap can admit a new load before this apply.
        let result = if registry.requires_resident_retirement_ack("candle") {
            (|| {
                let (instance, model) = identity.ok_or_else(retirement_refusal)?;
                if backend.is_ready()
                    || backend.resident_cpu_calibration_instance().is_some()
                    || !proof
                        .ok_or_else(retirement_refusal)?
                        .matches(expected.as_ref().ok_or_else(retirement_refusal)?)
                {
                    return Err(retirement_refusal());
                }
                let sequence = self
                    .resident_observation_sequence
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |s| s.checked_add(1))
                    .map_err(|_| retirement_refusal())?
                    + 1;
                let applied = registry
                    .observe_runtime_producer_release_for_owner(
                        RuntimeProducerObservation {
                            source_id: self.resident_source_id.clone(),
                            sequence,
                            allocation_state: RuntimeProducerAllocationState::Released,
                            observation: RuntimeObservation {
                                runtime_id: "candle".into(),
                                display_name: "Candle".into(),
                                backend_keys: vec!["candle".into()],
                                model_id: None,
                                runtime_instance_id: Some(instance.clone()),
                                status: RuntimeRegistryStatus::Stopped,
                                last_error: None,
                            },
                        },
                        RuntimeRetainedOwnerIdentity {
                            runtime_id: "candle",
                            source_id: &self.resident_source_id,
                            runtime_instance_id: &instance,
                            model_target: &model,
                        },
                    )
                    .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?;
                // An envelope fence can accept an observation as Unknown. That
                // preserves the charge and is not a successful retirement ACK.
                if applied.model_resource_residency.is_some()
                    || applied.resident_resources_uncertain
                    || applied.status != RuntimeRegistryStatus::Stopped
                    || !applied.models.is_empty()
                {
                    return Err(retirement_refusal());
                }
                Ok(())
            })()
        } else {
            Ok(())
        };
        self.finish_stopped_metadata().await;
        if let Err(error) = &result {
            self.runtime_lifecycle.write().await.last_error = Some(error.to_string());
        }
        result
    }
}

fn retirement_refusal() -> GatewayError {
    GatewayError::SwitchFailed("exact native CPU retirement acknowledgement required".into())
}
