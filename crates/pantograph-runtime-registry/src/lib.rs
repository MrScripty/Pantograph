mod admission;
mod observation;
mod reservation;
mod snapshot;
mod state;

use admission::RuntimeReservationClaim;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub use admission::{
    RuntimeAdmissionBudget, RuntimeAdmissionFailure, RuntimeReservationRequirements,
};
pub use observation::RuntimeObservation;
use pantograph_runtime_identity::canonical_runtime_id;
use reservation::RuntimeReservationRecord;
pub use reservation::{RuntimeReservationLease, RuntimeReservationRequest};
pub use snapshot::{RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot};
use state::RuntimeTransition as Transition;
pub use state::{
    RuntimeModelResidencyRecord, RuntimeRegistryRecord, RuntimeRegistryStatus, RuntimeTransition,
};

pub type SharedRuntimeRegistry = Arc<RuntimeRegistry>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeRegistryError {
    #[error("runtime '{0}' is not registered")]
    RuntimeNotFound(String),

    #[error("runtime '{runtime_id}' cannot transition from {from:?} to {to:?}")]
    InvalidTransition {
        runtime_id: String,
        from: RuntimeRegistryStatus,
        to: RuntimeRegistryStatus,
    },

    #[error("reservation '{0}' was not found")]
    ReservationNotFound(u64),

    #[error("runtime '{0}' cannot accept reservations while stopping or failed")]
    ReservationRejected(String),

    #[error("runtime '{runtime_id}' admission rejected reservation: {failure}")]
    AdmissionRejected {
        runtime_id: String,
        failure: RuntimeAdmissionFailure,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRegistration {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub admission_budget: Option<RuntimeAdmissionBudget>,
}

impl RuntimeRegistration {
    pub fn new(runtime_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            display_name: display_name.into(),
            backend_keys: Vec::new(),
            admission_budget: None,
        }
    }

    pub fn with_backend_keys(mut self, backend_keys: Vec<String>) -> Self {
        self.backend_keys = backend_keys;
        self
    }

    pub fn with_admission_budget(mut self, admission_budget: RuntimeAdmissionBudget) -> Self {
        self.admission_budget = Some(admission_budget);
        self
    }
}

#[derive(Debug, Default)]
struct RuntimeRegistryState {
    runtimes: BTreeMap<String, RuntimeRegistryRecord>,
    reservations: BTreeMap<u64, RuntimeReservationRecord>,
}

#[derive(Debug, Default)]
pub struct RuntimeRegistry {
    state: Mutex<RuntimeRegistryState>,
    reservation_sequence: AtomicU64,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_runtime(
        &self,
        registration: RuntimeRegistration,
    ) -> RuntimeRegistryRuntimeSnapshot {
        let runtime_id = canonical_runtime_id(&registration.runtime_id);
        let now_ms = unix_timestamp_ms();
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let record = guard.runtimes.entry(runtime_id.clone()).or_insert_with(|| {
            RuntimeRegistryRecord::new(&runtime_id, &registration.display_name, now_ms)
        });

        let RuntimeRegistration {
            runtime_id: _,
            display_name,
            backend_keys,
            admission_budget,
        } = registration;

        record.display_name = display_name.trim().to_string();
        record.set_backend_keys(backend_keys);
        record.runtime_id = runtime_id.clone();
        if let Some(admission_budget) = admission_budget {
            record.admission_budget = Some(admission_budget);
        }
        runtime_snapshot(record)
    }

    pub fn transition_runtime(
        &self,
        runtime_id: &str,
        transition: RuntimeTransition,
    ) -> Result<RuntimeRegistryRuntimeSnapshot, RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(runtime_id);
        let now_ms = unix_timestamp_ms();
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let record = guard
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

        if !transition.can_transition_from(record.status) {
            return Err(RuntimeRegistryError::InvalidTransition {
                runtime_id,
                from: record.status,
                to: transition.target_status(),
            });
        }

        record.status = transition.target_status();
        record.last_transition_at_ms = now_ms;

        match transition {
            Transition::WarmupStarted {
                runtime_instance_id,
            }
            | Transition::Ready {
                runtime_instance_id,
            }
            | Transition::Busy {
                runtime_instance_id,
            } => {
                if let Some(runtime_instance_id) = runtime_instance_id {
                    record.runtime_instance_id = Some(runtime_instance_id);
                }
                if !matches!(
                    record.status,
                    RuntimeRegistryStatus::Unhealthy | RuntimeRegistryStatus::Failed
                ) {
                    record.last_error = None;
                }
            }
            Transition::Unhealthy { message } | Transition::Failed { message } => {
                record.last_error = Some(message);
            }
            Transition::StopRequested => {}
            Transition::Stopped => {
                record.runtime_instance_id = None;
                record.last_error = None;
            }
        }

        Ok(runtime_snapshot(record))
    }

    pub fn acquire_reservation(
        &self,
        request: RuntimeReservationRequest,
    ) -> Result<RuntimeReservationLease, RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(&request.runtime_id);
        let created_at_ms = unix_timestamp_ms();
        let reservation_id = self.reservation_sequence.fetch_add(1, Ordering::Relaxed) + 1;

        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let record = guard
            .runtimes
            .get(&runtime_id)
            .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

        if matches!(
            record.status,
            RuntimeRegistryStatus::Stopping | RuntimeRegistryStatus::Failed
        ) {
            return Err(RuntimeRegistryError::ReservationRejected(runtime_id));
        }

        let claim = RuntimeReservationClaim::from_requirements(request.requirements.as_ref());
        if let Some(failure) = admission_failure(record, claim, &guard.reservations) {
            return Err(RuntimeRegistryError::AdmissionRejected {
                runtime_id,
                failure,
            });
        }

        let record = guard
            .runtimes
            .get_mut(&runtime_id)
            .expect("runtime must exist after admission check");

        let reservation = RuntimeReservationRecord {
            reservation_id,
            runtime_id: runtime_id.clone(),
            workflow_id: request.workflow_id,
            usage_profile: request.usage_profile,
            model_id: request.model_id,
            pin_runtime: request.pin_runtime,
            created_at_ms,
            claim,
        };

        record.active_reservations.insert(reservation_id);
        guard
            .reservations
            .insert(reservation_id, reservation.clone());

        Ok(reservation.into_lease())
    }

    pub fn release_reservation(&self, reservation_id: u64) -> Result<(), RuntimeRegistryError> {
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let reservation = guard
            .reservations
            .remove(&reservation_id)
            .ok_or(RuntimeRegistryError::ReservationNotFound(reservation_id))?;

        if let Some(runtime) = guard.runtimes.get_mut(&reservation.runtime_id) {
            runtime.active_reservations.remove(&reservation_id);
        }

        Ok(())
    }

    pub fn snapshot(&self) -> RuntimeRegistrySnapshot {
        let generated_at_ms = unix_timestamp_ms();
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let mut runtimes = guard
            .runtimes
            .values()
            .map(runtime_snapshot)
            .collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));

        let mut reservations = guard
            .reservations
            .values()
            .cloned()
            .map(RuntimeReservationRecord::into_lease)
            .collect::<Vec<_>>();
        reservations.sort_by(|left, right| {
            left.runtime_id
                .cmp(&right.runtime_id)
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        });

        RuntimeRegistrySnapshot {
            generated_at_ms,
            runtimes,
            reservations,
        }
    }

    pub fn observe_runtimes(
        &self,
        observations: Vec<RuntimeObservation>,
    ) -> Vec<RuntimeRegistryRuntimeSnapshot> {
        let now_ms = unix_timestamp_ms();
        let observed_runtime_ids = observations
            .iter()
            .map(|observation| canonical_runtime_id(&observation.runtime_id))
            .collect::<BTreeSet<_>>();
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");

        for observation in observations {
            apply_runtime_observation(&mut guard, observation, now_ms);
        }

        for record in guard.runtimes.values_mut() {
            if observed_runtime_ids.contains(&record.runtime_id)
                || !record.active_reservations.is_empty()
            {
                continue;
            }

            record.status = RuntimeRegistryStatus::Stopped;
            record.runtime_instance_id = None;
            record.last_error = None;
            record.models.clear();
            record.last_transition_at_ms = now_ms;
        }

        let mut snapshots = guard
            .runtimes
            .values()
            .map(runtime_snapshot)
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        snapshots
    }
}

fn runtime_snapshot(record: &RuntimeRegistryRecord) -> RuntimeRegistryRuntimeSnapshot {
    let mut backend_keys = record.backend_keys.iter().cloned().collect::<Vec<_>>();
    backend_keys.sort();

    let mut active_reservation_ids = record
        .active_reservations
        .iter()
        .copied()
        .collect::<Vec<_>>();
    active_reservation_ids.sort();

    let mut models = record.models.values().cloned().collect::<Vec<_>>();
    models.sort_by(|left, right| left.model_id.cmp(&right.model_id));

    RuntimeRegistryRuntimeSnapshot {
        runtime_id: record.runtime_id.clone(),
        display_name: record.display_name.clone(),
        backend_keys,
        status: record.status,
        runtime_instance_id: record.runtime_instance_id.clone(),
        last_error: record.last_error.clone(),
        last_transition_at_ms: record.last_transition_at_ms,
        active_reservation_ids,
        models,
    }
}

fn admission_failure(
    record: &RuntimeRegistryRecord,
    claim: RuntimeReservationClaim,
    reservations: &BTreeMap<u64, RuntimeReservationRecord>,
) -> Option<RuntimeAdmissionFailure> {
    let budget = record.admission_budget.as_ref()?;

    if let Some(requested_ram_mb) = claim.ram_mb {
        let reserved_ram_mb =
            total_reserved_resource_mb(&record.runtime_id, reservations, |reservation| {
                reservation.claim.ram_mb
            });
        let available_ram_mb = available_budget_mb(
            budget.total_ram_mb,
            budget.safety_margin_ram_mb,
            reserved_ram_mb,
        );
        if requested_ram_mb > available_ram_mb {
            return Some(RuntimeAdmissionFailure::InsufficientRam {
                requested_mb: requested_ram_mb,
                available_mb: available_ram_mb,
                reserved_mb: reserved_ram_mb,
                total_mb: budget.total_ram_mb.unwrap_or(0),
                safety_margin_mb: budget.safety_margin_ram_mb,
            });
        }
    }

    if let Some(requested_vram_mb) = claim.vram_mb {
        let reserved_vram_mb =
            total_reserved_resource_mb(&record.runtime_id, reservations, |reservation| {
                reservation.claim.vram_mb
            });
        let available_vram_mb = available_budget_mb(
            budget.total_vram_mb,
            budget.safety_margin_vram_mb,
            reserved_vram_mb,
        );
        if requested_vram_mb > available_vram_mb {
            return Some(RuntimeAdmissionFailure::InsufficientVram {
                requested_mb: requested_vram_mb,
                available_mb: available_vram_mb,
                reserved_mb: reserved_vram_mb,
                total_mb: budget.total_vram_mb.unwrap_or(0),
                safety_margin_mb: budget.safety_margin_vram_mb,
            });
        }
    }

    None
}

fn total_reserved_resource_mb<F>(
    runtime_id: &str,
    reservations: &BTreeMap<u64, RuntimeReservationRecord>,
    claim_mb: F,
) -> u64
where
    F: Fn(&RuntimeReservationRecord) -> Option<u64>,
{
    reservations
        .values()
        .filter(|reservation| reservation.runtime_id == runtime_id)
        .filter_map(claim_mb)
        .sum()
}

fn available_budget_mb(total_mb: Option<u64>, safety_margin_mb: u64, reserved_mb: u64) -> u64 {
    total_mb
        .unwrap_or(u64::MAX)
        .saturating_sub(safety_margin_mb)
        .saturating_sub(reserved_mb)
}

fn apply_runtime_observation(
    state: &mut RuntimeRegistryState,
    observation: RuntimeObservation,
    now_ms: u64,
) {
    let runtime_id = canonical_runtime_id(&observation.runtime_id);
    let record = state.runtimes.entry(runtime_id.clone()).or_insert_with(|| {
        RuntimeRegistryRecord::new(&runtime_id, &observation.display_name, now_ms)
    });

    record.runtime_id = runtime_id;
    record.display_name = observation.display_name;
    record.set_backend_keys(observation.backend_keys);
    record.status = observation.status;
    record.runtime_instance_id = match observation.status {
        RuntimeRegistryStatus::Stopped => None,
        _ => observation.runtime_instance_id,
    };
    record.last_error = observation.last_error;
    record.last_transition_at_ms = now_ms;
    sync_observed_models(record, observation.model_id, observation.status, now_ms);
}

fn sync_observed_models(
    record: &mut RuntimeRegistryRecord,
    model_id: Option<String>,
    status: RuntimeRegistryStatus,
    now_ms: u64,
) {
    if matches!(
        status,
        RuntimeRegistryStatus::Stopped | RuntimeRegistryStatus::Failed
    ) {
        record.models.clear();
        return;
    }

    let Some(model_id) = model_id else {
        record.models.clear();
        return;
    };

    let existing_loaded_at_ms = record
        .models
        .get(&model_id)
        .map(|model| model.loaded_at_ms)
        .unwrap_or(now_ms);
    record.models.clear();
    record.models.insert(
        model_id.clone(),
        RuntimeModelResidencyRecord {
            model_id,
            usage_profile: None,
            pinned: false,
            loaded_at_ms: existing_loaded_at_ms,
        },
    );
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_runtime_canonicalizes_runtime_and_backend_keys() {
        let registry = RuntimeRegistry::new();

        let snapshot = registry.register_runtime(
            RuntimeRegistration::new("PyTorch", "PyTorch sidecar").with_backend_keys(vec![
                "torch".to_string(),
                "PyTorch".to_string(),
                "torch".to_string(),
            ]),
        );

        assert_eq!(snapshot.runtime_id, "pytorch");
        assert_eq!(snapshot.backend_keys, vec!["pytorch".to_string()]);
        assert_eq!(snapshot.status, RuntimeRegistryStatus::Stopped);
    }

    #[test]
    fn transition_runtime_rejects_invalid_state_changes() {
        let registry = RuntimeRegistry::new();
        registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));

        let err = registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::Busy {
                    runtime_instance_id: Some("runtime-1".to_string()),
                },
            )
            .expect_err("busy from stopped should be rejected");

        assert_eq!(
            err,
            RuntimeRegistryError::InvalidTransition {
                runtime_id: "llama_cpp".to_string(),
                from: RuntimeRegistryStatus::Stopped,
                to: RuntimeRegistryStatus::Busy,
            }
        );
    }

    #[test]
    fn reservation_lifecycle_updates_snapshot_state() {
        let registry = RuntimeRegistry::new();
        registry.register_runtime(
            RuntimeRegistration::new("onnxruntime", "ONNX Runtime")
                .with_backend_keys(vec!["onnxruntime".to_string()]),
        );
        registry
            .transition_runtime(
                "onnx-runtime",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("python-runtime:onnx-runtime".to_string()),
                },
            )
            .expect("ready transition");

        let lease = registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "onnxruntime".to_string(),
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("audio".to_string()),
                model_id: Some("model-a".to_string()),
                pin_runtime: true,
                requirements: None,
            })
            .expect("acquire reservation");

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.runtimes.len(), 1);
        assert_eq!(snapshot.runtimes[0].runtime_id, "onnx-runtime");
        assert_eq!(
            snapshot.runtimes[0].active_reservation_ids,
            vec![lease.reservation_id]
        );
        assert_eq!(snapshot.reservations.len(), 1);
        assert_eq!(snapshot.reservations[0].runtime_id, "onnx-runtime");

        registry
            .release_reservation(lease.reservation_id)
            .expect("release reservation");

        let released = registry.snapshot();
        assert!(released.runtimes[0].active_reservation_ids.is_empty());
        assert!(released.reservations.is_empty());
    }

    #[test]
    fn register_runtime_without_new_budget_preserves_existing_budget() {
        let registry = RuntimeRegistry::new();
        registry.register_runtime(
            RuntimeRegistration::new("llama.cpp", "llama.cpp").with_admission_budget(
                RuntimeAdmissionBudget::new(Some(4096), Some(8192))
                    .with_safety_margin_ram_mb(256)
                    .with_safety_margin_vram_mb(1024),
            ),
        );

        registry.register_runtime(
            RuntimeRegistration::new("llama.cpp", "llama.cpp")
                .with_backend_keys(vec!["llama_cpp".to_string()]),
        );
        registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-1".to_string()),
                },
            )
            .expect("ready transition");

        let err = registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "llama.cpp".to_string(),
                workflow_id: "wf-budget".to_string(),
                usage_profile: None,
                model_id: None,
                pin_runtime: false,
                requirements: Some(RuntimeReservationRequirements {
                    estimated_peak_vram_mb: Some(7200),
                    estimated_peak_ram_mb: None,
                    estimated_min_vram_mb: Some(4096),
                    estimated_min_ram_mb: None,
                }),
            })
            .expect_err("preserved vram budget should still reject oversized request");

        assert_eq!(
            err,
            RuntimeRegistryError::AdmissionRejected {
                runtime_id: "llama_cpp".to_string(),
                failure: RuntimeAdmissionFailure::InsufficientVram {
                    requested_mb: 7200,
                    available_mb: 7168,
                    reserved_mb: 0,
                    total_mb: 8192,
                    safety_margin_mb: 1024,
                },
            }
        );
    }

    #[test]
    fn reservations_are_rejected_while_runtime_is_stopping() {
        let registry = RuntimeRegistry::new();
        registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
        registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-llama.cpp".to_string()),
                },
            )
            .expect("ready transition");
        registry
            .transition_runtime("llama.cpp", RuntimeTransition::StopRequested)
            .expect("stop transition");

        let err = registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "llama.cpp".to_string(),
                workflow_id: "wf-stop".to_string(),
                usage_profile: None,
                model_id: None,
                pin_runtime: false,
                requirements: None,
            })
            .expect_err("stopping runtime should reject reservations");

        assert_eq!(
            err,
            RuntimeRegistryError::ReservationRejected("llama_cpp".to_string())
        );
    }

    #[test]
    fn observe_runtimes_registers_active_and_embedding_runtimes() {
        let registry = RuntimeRegistry::new();

        let snapshots = registry.observe_runtimes(vec![
            RuntimeObservation {
                runtime_id: "llama.cpp".to_string(),
                display_name: "llama.cpp".to_string(),
                backend_keys: vec!["llama_cpp".to_string()],
                model_id: Some("/models/qwen.gguf".to_string()),
                status: RuntimeRegistryStatus::Ready,
                runtime_instance_id: Some("llama-main-1".to_string()),
                last_error: None,
            },
            RuntimeObservation {
                runtime_id: "llama.cpp.embedding".to_string(),
                display_name: "Dedicated embedding runtime".to_string(),
                backend_keys: vec!["llama_cpp".to_string()],
                model_id: Some("/models/embed.gguf".to_string()),
                status: RuntimeRegistryStatus::Warming,
                runtime_instance_id: Some("llama-embed-1".to_string()),
                last_error: None,
            },
        ]);

        assert_eq!(snapshots.len(), 2);
        let active_runtime = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama_cpp")
            .expect("active runtime snapshot");
        assert_eq!(active_runtime.status, RuntimeRegistryStatus::Ready);
        assert_eq!(active_runtime.models[0].model_id, "/models/qwen.gguf");

        let embedding_runtime = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(embedding_runtime.status, RuntimeRegistryStatus::Warming);
        assert_eq!(embedding_runtime.models[0].model_id, "/models/embed.gguf");
    }

    #[test]
    fn observe_runtimes_stops_unobserved_runtimes_without_reservations() {
        let registry = RuntimeRegistry::new();

        registry.observe_runtimes(vec![RuntimeObservation {
            runtime_id: "llama.cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("/models/qwen.gguf".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("llama-main-1".to_string()),
            last_error: None,
        }]);

        let snapshots = registry.observe_runtimes(vec![RuntimeObservation {
            runtime_id: "ollama".to_string(),
            display_name: "ollama".to_string(),
            backend_keys: vec!["ollama".to_string()],
            model_id: Some("llava:13b".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("ollama-1".to_string()),
            last_error: None,
        }]);

        assert_eq!(snapshots.len(), 2);
        let llama = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama_cpp")
            .expect("llama snapshot");
        assert_eq!(llama.status, RuntimeRegistryStatus::Stopped);
        assert!(llama.models.is_empty());

        let ollama = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "ollama")
            .expect("ollama snapshot");
        assert_eq!(ollama.status, RuntimeRegistryStatus::Ready);
        assert_eq!(ollama.models[0].model_id, "llava:13b");
    }

    #[test]
    fn admission_budget_rejects_reservations_that_exceed_remaining_vram() {
        let registry = RuntimeRegistry::new();
        registry.register_runtime(
            RuntimeRegistration::new("llama.cpp", "llama.cpp").with_admission_budget(
                RuntimeAdmissionBudget::new(None, Some(8192)).with_safety_margin_vram_mb(1024),
            ),
        );
        registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-1".to_string()),
                },
            )
            .expect("ready transition");

        registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "llama.cpp".to_string(),
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                model_id: Some("model-a".to_string()),
                pin_runtime: false,
                requirements: Some(RuntimeReservationRequirements {
                    estimated_peak_vram_mb: Some(6144),
                    estimated_peak_ram_mb: None,
                    estimated_min_vram_mb: Some(4096),
                    estimated_min_ram_mb: None,
                }),
            })
            .expect("first reservation should fit available vram");

        let err = registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "llama.cpp".to_string(),
                workflow_id: "wf-2".to_string(),
                usage_profile: None,
                model_id: Some("model-b".to_string()),
                pin_runtime: false,
                requirements: Some(RuntimeReservationRequirements {
                    estimated_peak_vram_mb: Some(2048),
                    estimated_peak_ram_mb: None,
                    estimated_min_vram_mb: Some(1024),
                    estimated_min_ram_mb: None,
                }),
            })
            .expect_err("second reservation should exceed remaining vram");

        assert_eq!(
            err,
            RuntimeRegistryError::AdmissionRejected {
                runtime_id: "llama_cpp".to_string(),
                failure: RuntimeAdmissionFailure::InsufficientVram {
                    requested_mb: 2048,
                    available_mb: 1024,
                    reserved_mb: 6144,
                    total_mb: 8192,
                    safety_margin_mb: 1024,
                },
            }
        );
    }

    #[test]
    fn admission_budget_uses_peak_ram_claim_and_release_restores_capacity() {
        let registry = RuntimeRegistry::new();
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch").with_admission_budget(
                RuntimeAdmissionBudget::new(Some(4096), None).with_safety_margin_ram_mb(512),
            ),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-ram".to_string()),
                },
            )
            .expect("ready transition");

        let lease = registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "pytorch".to_string(),
                workflow_id: "wf-ram-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                model_id: Some("model-ram-a".to_string()),
                pin_runtime: false,
                requirements: Some(RuntimeReservationRequirements {
                    estimated_peak_vram_mb: None,
                    estimated_peak_ram_mb: Some(3584),
                    estimated_min_vram_mb: None,
                    estimated_min_ram_mb: Some(1024),
                }),
            })
            .expect("peak ram claim should fit exactly");

        let err = registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "pytorch".to_string(),
                workflow_id: "wf-ram-2".to_string(),
                usage_profile: None,
                model_id: Some("model-ram-b".to_string()),
                pin_runtime: false,
                requirements: Some(RuntimeReservationRequirements {
                    estimated_peak_vram_mb: None,
                    estimated_peak_ram_mb: Some(1),
                    estimated_min_vram_mb: None,
                    estimated_min_ram_mb: Some(1),
                }),
            })
            .expect_err("no ram should remain after first reservation");

        assert_eq!(
            err,
            RuntimeRegistryError::AdmissionRejected {
                runtime_id: "pytorch".to_string(),
                failure: RuntimeAdmissionFailure::InsufficientRam {
                    requested_mb: 1,
                    available_mb: 0,
                    reserved_mb: 3584,
                    total_mb: 4096,
                    safety_margin_mb: 512,
                },
            }
        );

        registry
            .release_reservation(lease.reservation_id)
            .expect("release reservation");

        registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "pytorch".to_string(),
                workflow_id: "wf-ram-3".to_string(),
                usage_profile: None,
                model_id: Some("model-ram-c".to_string()),
                pin_runtime: false,
                requirements: Some(RuntimeReservationRequirements {
                    estimated_peak_vram_mb: None,
                    estimated_peak_ram_mb: Some(1024),
                    estimated_min_vram_mb: None,
                    estimated_min_ram_mb: Some(512),
                }),
            })
            .expect("released capacity should admit a new reservation");
    }
}
