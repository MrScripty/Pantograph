//! Scoped telemetry collection for one inference execution boundary.
//!
//! The gateway owns scope creation and lifecycle event emission. Backends may
//! receive a recorder to report typed observations, but they must not emit
//! lifecycle diagnostics directly or place telemetry in task outputs.

use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::{InferenceExecutionResourceObservation, InferenceExecutionResourceObservationError};

#[derive(Debug, Default)]
struct InferenceExecutionTelemetryState {
    resource_observation: Option<InferenceExecutionResourceObservation>,
}

/// Owns telemetry collected for one inference execution boundary.
#[derive(Debug, Default)]
#[must_use]
pub struct InferenceExecutionTelemetryScope {
    state: Arc<Mutex<InferenceExecutionTelemetryState>>,
}

impl InferenceExecutionTelemetryScope {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn recorder(&self) -> InferenceExecutionTelemetryRecorder {
        InferenceExecutionTelemetryRecorder {
            state: Arc::clone(&self.state),
        }
    }

    /// Drain and merge the collected resource observations.
    ///
    /// This is intentionally terminal-summary behavior. Live event streaming is
    /// a separate future contract and should not be inferred from this method.
    pub fn drain_resource_observation(
        &self,
    ) -> Result<Option<InferenceExecutionResourceObservation>, InferenceExecutionTelemetryError>
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| InferenceExecutionTelemetryError::CollectorPoisoned)?;
        Ok(state.resource_observation.take())
    }
}

/// Minimal backend execution context for one backend call.
#[derive(Debug, Clone)]
pub struct BackendExecutionContext {
    telemetry_recorder: InferenceExecutionTelemetryRecorder,
}

impl BackendExecutionContext {
    #[must_use]
    pub fn new(telemetry_recorder: InferenceExecutionTelemetryRecorder) -> Self {
        Self { telemetry_recorder }
    }

    #[must_use]
    pub fn telemetry_recorder(&self) -> &InferenceExecutionTelemetryRecorder {
        &self.telemetry_recorder
    }
}

/// Backend-owned provider for runtime-native terminal telemetry.
///
/// Implementations may call structured runtime APIs or return typed
/// unavailable observations. They must not parse unbounded process logs or
/// simulate runtime-native metrics from OS process RSS.
pub trait RuntimeNativeTelemetryProvider: Send + Sync {
    fn finish_resource_observation(
        &self,
    ) -> Result<Option<InferenceExecutionResourceObservation>, InferenceExecutionTelemetryError>;
}

/// Cloneable backend-facing telemetry recorder.
#[derive(Debug, Clone)]
pub struct InferenceExecutionTelemetryRecorder {
    state: Arc<Mutex<InferenceExecutionTelemetryState>>,
}

impl InferenceExecutionTelemetryRecorder {
    pub fn record_resource_observation(
        &self,
        observation: InferenceExecutionResourceObservation,
    ) -> Result<(), InferenceExecutionTelemetryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| InferenceExecutionTelemetryError::CollectorPoisoned)?;
        state.resource_observation = match state.resource_observation.take() {
            Some(existing) => match existing.clone().merge(observation) {
                Ok(merged) => Some(merged),
                Err(error) => {
                    state.resource_observation = Some(existing);
                    return Err(error.into());
                }
            },
            None => Some(observation),
        };
        Ok(())
    }
}

/// Telemetry collection errors.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceExecutionTelemetryError {
    #[error("inference execution telemetry collector lock was poisoned")]
    CollectorPoisoned,
    #[error(transparent)]
    InvalidResourceObservation(#[from] InferenceExecutionResourceObservationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InferenceExecutionResourceObservation, InferenceMemoryFailureKind,
        InferenceResourceObservationMetricKind, InferenceResourceObservationSource,
        InferenceResourceObservationSourceKind,
    };

    #[test]
    fn telemetry_scope_drains_none_without_observations() {
        let scope = InferenceExecutionTelemetryScope::new();

        let drained = scope
            .drain_resource_observation()
            .expect("empty scope drains");

        assert_eq!(drained, None);
    }

    #[test]
    fn telemetry_recorder_merges_resource_observations_for_terminal_summary() {
        let scope = InferenceExecutionTelemetryScope::new();
        let recorder = scope.recorder();

        recorder
            .record_resource_observation(
                InferenceExecutionResourceObservation::peak_ram(
                    2048,
                    InferenceResourceObservationSourceKind::OsProcessRss,
                )
                .expect("valid RAM observation"),
            )
            .expect("record RAM observation");
        recorder
            .record_resource_observation(
                InferenceExecutionResourceObservation::peak_ram(
                    1024,
                    InferenceResourceObservationSourceKind::ManagedRuntimeTelemetry,
                )
                .expect("valid lower RAM observation"),
            )
            .expect("record lower RAM observation");
        recorder
            .record_resource_observation(
                InferenceExecutionResourceObservation::peak_vram(
                    4096,
                    InferenceResourceObservationSourceKind::PytorchCuda,
                )
                .expect("valid VRAM observation"),
            )
            .expect("record VRAM observation");
        recorder
            .record_resource_observation(InferenceExecutionResourceObservation::memory_failure(
                InferenceMemoryFailureKind::OutOfMemory,
            ))
            .expect("record memory failure observation");

        let drained = scope
            .drain_resource_observation()
            .expect("scope drains")
            .expect("resource observation is present");

        assert_eq!(drained.peak_ram_bytes(), Some(2048));
        assert_eq!(drained.peak_vram_bytes(), Some(4096));
        assert_eq!(
            drained.memory_failure_kind(),
            Some(InferenceMemoryFailureKind::OutOfMemory)
        );
        assert_eq!(drained.sources().len(), 3);
    }

    #[test]
    fn telemetry_scope_drain_is_terminal_and_one_shot() {
        let scope = InferenceExecutionTelemetryScope::new();
        let recorder = scope.recorder();
        recorder
            .record_resource_observation(
                InferenceExecutionResourceObservation::peak_ram(
                    1024,
                    InferenceResourceObservationSourceKind::OsProcessRss,
                )
                .expect("valid RAM observation"),
            )
            .expect("record observation");

        assert!(scope
            .drain_resource_observation()
            .expect("first drain succeeds")
            .is_some());
        assert_eq!(
            scope
                .drain_resource_observation()
                .expect("second drain succeeds"),
            None
        );
    }

    #[test]
    fn telemetry_recorder_preserves_existing_observation_when_merge_fails() {
        let scope = InferenceExecutionTelemetryScope::new();
        let recorder = scope.recorder();
        let source_kinds = [
            InferenceResourceObservationSourceKind::PytorchCuda,
            InferenceResourceObservationSourceKind::PytorchMps,
            InferenceResourceObservationSourceKind::PytorchCpu,
            InferenceResourceObservationSourceKind::OsProcessRss,
            InferenceResourceObservationSourceKind::ManagedRuntimeTelemetry,
            InferenceResourceObservationSourceKind::ExternalRuntimeAdapter,
        ];
        let mut sources: Vec<_> = source_kinds
            .iter()
            .copied()
            .map(|source_kind| {
                InferenceResourceObservationSource::new(
                    InferenceResourceObservationMetricKind::PeakRamBytes,
                    source_kind,
                )
            })
            .collect();
        sources.extend(source_kinds.iter().take(2).copied().map(|source_kind| {
            InferenceResourceObservationSource::new(
                InferenceResourceObservationMetricKind::PeakVramBytes,
                source_kind,
            )
        }));
        let existing = InferenceExecutionResourceObservation::new(
            Some(2048),
            Some(4096),
            None,
            sources,
            Vec::new(),
        )
        .expect("source-limit observation is valid");
        recorder
            .record_resource_observation(existing)
            .expect("record initial observation");

        let error = recorder
            .record_resource_observation(
                InferenceExecutionResourceObservation::peak_vram(
                    8192,
                    InferenceResourceObservationSourceKind::PytorchCuda,
                )
                .expect("valid duplicate-source observation"),
            )
            .expect_err("merged source list should exceed the source limit");

        assert!(matches!(
            error,
            InferenceExecutionTelemetryError::InvalidResourceObservation(
                InferenceExecutionResourceObservationError::TooManySources { .. }
            )
        ));
        let drained = scope
            .drain_resource_observation()
            .expect("scope drains")
            .expect("existing observation is preserved");
        assert_eq!(drained.peak_ram_bytes(), Some(2048));
        assert_eq!(drained.peak_vram_bytes(), Some(4096));
    }
}
