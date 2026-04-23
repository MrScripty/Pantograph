use super::*;
use crate::snapshot::{RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot};
use crate::state::RuntimeRegistryStatus;

fn empty_snapshot() -> RuntimeRegistrySnapshot {
    RuntimeRegistrySnapshot {
        generated_at_ms: 123,
        runtimes: Vec::new(),
        reservations: Vec::new(),
    }
}

fn runtime_snapshot(
    runtime_id: &str,
    backend_keys: Vec<&str>,
    status: RuntimeRegistryStatus,
    active_reservation_count: usize,
) -> RuntimeRegistryRuntimeSnapshot {
    RuntimeRegistryRuntimeSnapshot {
        runtime_id: runtime_id.to_string(),
        display_name: runtime_id.to_string(),
        backend_keys: backend_keys.into_iter().map(ToOwned::to_owned).collect(),
        status,
        runtime_instance_id: Some(format!("{runtime_id}-instance")),
        last_error: None,
        last_transition_at_ms: 123,
        active_reservation_ids: (0..active_reservation_count as u64).collect(),
        models: Vec::new(),
    }
}

#[test]
fn technical_fit_request_normalizes_inputs_and_defaults_legal_factors() {
    let request = RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("  workflow-a  ".to_string()),
        required_model_ids: vec![" model-a ".to_string(), "model-a".to_string()],
        required_backend_keys: vec!["llama.cpp".to_string(), "llama_cpp".to_string()],
        required_extensions: vec![" kv_cache ".to_string(), "kv_cache".to_string()],
        required_context_window_tokens: Some(8192),
        override_selection: Some(RuntimeTechnicalFitOverride {
            model_id: Some(" model-a ".to_string()),
            backend_key: Some("llama.cpp".to_string()),
        }),
        legal_factors: Vec::new(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: " ".to_string(),
            runtime_id: Some("llama.cpp".to_string()),
            backend_key: Some("llama.cpp".to_string()),
            model_id: Some(" model-a ".to_string()),
            source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasFeasible,
            context_window_tokens: Some(8192),
            residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
            warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
            supports_runtime_requirements: true,
        }],
        resource_pressure: Some(RuntimeTechnicalFitResourcePressure {
            queued_run_count: Some(2),
            loaded_runtime_count: Some(1),
            loaded_runtime_capacity: Some(2),
            estimated_peak_vram_mb: Some(4096),
            estimated_peak_ram_mb: Some(8192),
        }),
    };

    let normalized = request.normalized();

    assert_eq!(normalized.workflow_id.as_deref(), Some("workflow-a"));
    assert_eq!(normalized.required_model_ids, vec!["model-a".to_string()]);
    assert_eq!(
        normalized.required_backend_keys,
        vec!["llama_cpp".to_string()]
    );
    assert_eq!(normalized.required_extensions, vec!["kv_cache".to_string()]);
    assert_eq!(
        normalized.override_selection,
        Some(RuntimeTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
        })
    );
    assert_eq!(normalized.legal_factors, RuntimeTechnicalFitFactor::all());
    assert_eq!(
        normalized.candidates[0].candidate_id,
        "llama_cpp|llama_cpp|model-a"
    );
    assert_eq!(
        normalized.candidates[0].runtime_id.as_deref(),
        Some("llama_cpp")
    );
    assert_eq!(
        normalized.candidates[0].backend_key.as_deref(),
        Some("llama_cpp")
    );
}

#[test]
fn technical_fit_override_drops_empty_fields() {
    let override_selection = RuntimeTechnicalFitOverride {
        model_id: Some("  ".to_string()),
        backend_key: Some(" ".to_string()),
    };

    assert_eq!(override_selection.normalized(), None);
}

#[test]
fn technical_fit_decision_normalizes_selected_identifiers() {
    let decision = RuntimeTechnicalFitDecision {
        selection_mode: RuntimeTechnicalFitSelectionMode::ExplicitOverride,
        selected_candidate_id: Some(" candidate-1 ".to_string()),
        selected_runtime_id: Some("llama.cpp".to_string()),
        selected_backend_key: Some("llama.cpp".to_string()),
        selected_model_id: Some(" model-a ".to_string()),
        reasons: vec![RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
            Some(" candidate-1 "),
        )],
    };

    let normalized = decision.normalized();

    assert_eq!(
        normalized.selected_candidate_id.as_deref(),
        Some("candidate-1")
    );
    assert_eq!(normalized.selected_runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(
        normalized.selected_backend_key.as_deref(),
        Some("llama_cpp")
    );
    assert_eq!(normalized.selected_model_id.as_deref(), Some("model-a"));
    assert_eq!(
        normalized.reasons,
        vec![RuntimeTechnicalFitReason {
            code: RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
            candidate_id: Some("candidate-1".to_string()),
        }]
    );
}

#[test]
fn selector_prefers_explicit_override_over_hotter_candidate() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: RuntimeRegistrySnapshot {
            generated_at_ms: 123,
            runtimes: vec![
                runtime_snapshot(
                    "runtime-a",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Busy,
                    1,
                ),
                runtime_snapshot("runtime-b", vec!["ollama"], RuntimeRegistryStatus::Ready, 0),
            ],
            reservations: Vec::new(),
        },
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: Vec::new(),
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: Some(RuntimeTechnicalFitOverride {
            model_id: None,
            backend_key: Some("ollama".to_string()),
        }),
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-a".to_string(),
                runtime_id: Some("runtime-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasFeasible,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                supports_runtime_requirements: true,
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-b".to_string(),
                runtime_id: Some("runtime-b".to_string()),
                backend_key: Some("ollama".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasFeasible,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
                supports_runtime_requirements: true,
            },
        ],
        resource_pressure: None,
    });

    assert_eq!(
        decision,
        RuntimeTechnicalFitDecision {
            selection_mode: RuntimeTechnicalFitSelectionMode::ExplicitOverride,
            selected_candidate_id: Some("runtime-b".to_string()),
            selected_runtime_id: Some("runtime-b".to_string()),
            selected_backend_key: Some("ollama".to_string()),
            selected_model_id: None,
            reasons: vec![RuntimeTechnicalFitReason {
                code: RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
                candidate_id: Some("runtime-b".to_string()),
            }],
        }
    );
}

#[test]
fn selector_uses_snapshot_residency_and_deterministic_tie_break() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: RuntimeRegistrySnapshot {
            generated_at_ms: 123,
            runtimes: vec![
                runtime_snapshot(
                    "runtime-b",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Ready,
                    0,
                ),
                runtime_snapshot(
                    "runtime-a",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Ready,
                    0,
                ),
            ],
            reservations: Vec::new(),
        },
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["llama_cpp".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-b".to_string(),
                runtime_id: Some("runtime-b".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-a".to_string(),
                runtime_id: Some("runtime-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
            },
        ],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::Automatic
    );
    assert_eq!(decision.selected_candidate_id.as_deref(), Some("runtime-a"));
    assert_eq!(decision.selected_runtime_id.as_deref(), Some("runtime-a"));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::DeterministicTieBreak
            && reason.candidate_id.as_deref() == Some("runtime-a")
    }));
}

#[test]
fn selector_falls_back_conservatively_when_required_context_is_missing() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["llama_cpp".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: Some(8192),
        override_selection: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: "runtime-a".to_string(),
            runtime_id: Some("runtime-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            model_id: None,
            source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
            context_window_tokens: None,
            residency_state: None,
            warmup_state: None,
            supports_runtime_requirements: true,
        }],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::ConservativeFallback
    );
    assert_eq!(decision.selected_candidate_id.as_deref(), Some("runtime-a"));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingRuntimeState
            && reason.candidate_id.as_deref() == Some("runtime-a")
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ConservativeFallback
            && reason.candidate_id.as_deref() == Some("runtime-a")
    }));
}

#[test]
fn selector_conservative_fallback_stays_with_required_backend_candidate() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["llama_cpp".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "candle".to_string(),
                runtime_id: Some("candle".to_string()),
                backend_key: Some("candle".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                supports_runtime_requirements: true,
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "llama_cpp".to_string(),
                runtime_id: Some("llama_cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Unloaded),
                warmup_state: None,
                supports_runtime_requirements: false,
            },
        ],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::ConservativeFallback
    );
    assert_eq!(decision.selected_candidate_id.as_deref(), Some("llama_cpp"));
    assert_eq!(decision.selected_runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(decision.selected_backend_key.as_deref(), Some("llama_cpp"));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ConservativeFallback
            && reason.candidate_id.as_deref() == Some("llama_cpp")
    }));
}

#[test]
fn selector_prefers_more_headroom_under_queue_pressure() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: RuntimeRegistrySnapshot {
            generated_at_ms: 123,
            runtimes: vec![
                runtime_snapshot(
                    "runtime-hot",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Ready,
                    3,
                ),
                runtime_snapshot(
                    "runtime-cool",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Ready,
                    1,
                ),
            ],
            reservations: Vec::new(),
        },
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["llama_cpp".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-hot".to_string(),
                runtime_id: Some("runtime-hot".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-cool".to_string(),
                runtime_id: Some("runtime-cool".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
            },
        ],
        resource_pressure: Some(RuntimeTechnicalFitResourcePressure {
            queued_run_count: Some(4),
            loaded_runtime_count: Some(2),
            loaded_runtime_capacity: Some(4),
            estimated_peak_vram_mb: None,
            estimated_peak_ram_mb: None,
        }),
    });

    assert_eq!(
        decision.selected_candidate_id.as_deref(),
        Some("runtime-cool")
    );
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::QueuePressure
            && reason.candidate_id.as_deref() == Some("runtime-cool")
    }));
}

#[test]
fn selector_prefers_more_headroom_under_budget_pressure() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: RuntimeRegistrySnapshot {
            generated_at_ms: 123,
            runtimes: vec![
                runtime_snapshot(
                    "runtime-tight",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Busy,
                    2,
                ),
                runtime_snapshot(
                    "runtime-roomy",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Busy,
                    0,
                ),
            ],
            reservations: Vec::new(),
        },
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["llama_cpp".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-tight".to_string(),
                runtime_id: Some("runtime-tight".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-roomy".to_string(),
                runtime_id: Some("runtime-roomy".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
            },
        ],
        resource_pressure: Some(RuntimeTechnicalFitResourcePressure {
            queued_run_count: Some(0),
            loaded_runtime_count: Some(2),
            loaded_runtime_capacity: Some(2),
            estimated_peak_vram_mb: Some(4096),
            estimated_peak_ram_mb: Some(8192),
        }),
    });

    assert_eq!(
        decision.selected_candidate_id.as_deref(),
        Some("runtime-roomy")
    );
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::BudgetPressure
            && reason.candidate_id.as_deref() == Some("runtime-roomy")
    }));
}
