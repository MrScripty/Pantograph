use super::*;
use crate::runtime_selection_policy::{
    RuntimeSelectionDecisionInput, RuntimeSelectionInputValidationError,
};
use crate::snapshot::{RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot};
use crate::state::RuntimeRegistryStatus;
use serde::Deserialize;

fn deserialize_source_kind(
    raw: &'static str,
) -> Result<RuntimeTechnicalFitCandidateSourceKind, serde::de::value::Error> {
    RuntimeTechnicalFitCandidateSourceKind::deserialize(serde::de::value::StrDeserializer::new(raw))
}

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
fn runtime_capability_source_kind_uses_facts_wire_value() {
    let decoded = deserialize_source_kind("runtime_capability_facts")
        .expect("runtime capability facts should deserialize");

    assert_eq!(
        decoded,
        RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts
    );
}

#[test]
fn runtime_capability_source_kind_rejects_retired_fallback_wire_value() {
    let decoded = deserialize_source_kind("runtime_capability_fallback");

    assert!(
        decoded.is_err(),
        "retired runtime capability fallback source must not deserialize"
    );
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
            runtime_id: Some("llama.cpp".to_string()),
            runtime_variant_id: Some(" llama-cpp/linux-x64/cuda ".to_string()),
            model_id: Some(" model-a ".to_string()),
            backend_key: Some("llama.cpp".to_string()),
        }),
        device_policy: Some(RuntimeTechnicalFitDevicePolicy::Explicit {
            device_class: RuntimeTechnicalFitDeviceClass::Cuda,
            device_id: Some(" cuda:0 ".to_string()),
        }),
        legal_factors: Vec::new(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: " ".to_string(),
            runtime_id: Some("llama.cpp".to_string()),
            backend_key: Some("llama.cpp".to_string()),
            model_id: Some(" model-a ".to_string()),
            runtime_variant_id: Some(" llama-cpp/linux-x64/cuda ".to_string()),
            device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
            selected_device_id: Some(" cuda:0 ".to_string()),
            resource_estimate: Some(RuntimeTechnicalFitResourceEstimate {
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
                estimated_min_vram_mb: Some(2048),
                estimated_min_ram_mb: Some(4096),
            }),
            observed_throughput_hint: Some(RuntimeTechnicalFitObservedThroughputHint {
                tokens_per_second_milli: None,
                images_per_second_milli: Some(125),
                sample_count: Some(3),
            }),
            device_diagnostics: vec![RuntimeTechnicalFitDeviceDiagnostic {
                code: RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable,
                severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning,
                message: " cuda runtime warmup pending ".to_string(),
                device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
                device_id: Some(" cuda:0 ".to_string()),
                runtime_variant_id: Some(" llama-cpp/linux-x64/cuda ".to_string()),
                backend_key: Some("llama.cpp".to_string()),
            }],
            source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
            context_window_tokens: Some(8192),
            residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
            warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
            supports_runtime_requirements: true,
            compatibility_report: Some(RuntimeTechnicalFitCompatibilityReport {
                status: " rejected ".to_string(),
                compatible: false,
                task: " supported ".to_string(),
                model_source: " unsupported ".to_string(),
                preprocessing: " supported ".to_string(),
                postprocessing: " supported ".to_string(),
            }),
            compatibility_issue_count: 1,
            compatibility_issues: vec![RuntimeTechnicalFitCompatibilityIssue {
                kind: " unsupported_model_artifact ".to_string(),
                phase: " model_package_resolution ".to_string(),
                message: " backend cannot load artifact ".to_string(),
                model_id: Some(" model-a ".to_string()),
                path: Some(" model.gguf ".to_string()),
            }],
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
            runtime_id: Some("llama_cpp".to_string()),
            runtime_variant_id: Some("llama-cpp/linux-x64/cuda".to_string()),
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
        })
    );
    assert_eq!(
        normalized.device_policy,
        Some(RuntimeTechnicalFitDevicePolicy::Explicit {
            device_class: RuntimeTechnicalFitDeviceClass::Cuda,
            device_id: Some("cuda:0".to_string()),
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
    assert_eq!(
        normalized.candidates[0].runtime_variant_id.as_deref(),
        Some("llama-cpp/linux-x64/cuda")
    );
    assert_eq!(
        normalized.candidates[0].selected_device_id.as_deref(),
        Some("cuda:0")
    );
    assert_eq!(
        normalized.candidates[0]
            .resource_estimate
            .as_ref()
            .and_then(|estimate| estimate.estimated_peak_vram_mb),
        Some(4096)
    );
    assert_eq!(
        normalized.candidates[0]
            .observed_throughput_hint
            .as_ref()
            .and_then(|hint| hint.images_per_second_milli),
        Some(125)
    );
    assert_eq!(
        normalized.candidates[0].device_diagnostics[0]
            .runtime_variant_id
            .as_deref(),
        Some("llama-cpp/linux-x64/cuda")
    );
    assert_eq!(
        normalized.candidates[0].device_diagnostics[0]
            .backend_key
            .as_deref(),
        Some("llama_cpp")
    );
    assert_eq!(
        normalized.candidates[0]
            .compatibility_report
            .as_ref()
            .map(|report| (report.status.as_str(), report.model_source.as_str())),
        Some(("rejected", "unsupported"))
    );
    assert_eq!(normalized.candidates[0].compatibility_issue_count, 1);
    assert_eq!(
        normalized.candidates[0].compatibility_issues[0].kind,
        "unsupported_model_artifact"
    );
    assert_eq!(
        normalized.candidates[0].compatibility_issues[0]
            .model_id
            .as_deref(),
        Some("model-a")
    );
}

#[test]
fn runtime_selection_input_requires_normalized_request() {
    let request = RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some(" workflow-a ".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: Vec::new(),
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        device_policy: None,
        legal_factors: Vec::new(),
        candidates: Vec::new(),
        resource_pressure: None,
    };

    assert!(matches!(
        RuntimeSelectionDecisionInput::try_from_normalized_request(&request),
        Err(RuntimeSelectionInputValidationError::UnnormalizedRequest)
    ));

    let normalized = request.normalized();
    assert!(RuntimeSelectionDecisionInput::try_from_normalized_request(&normalized).is_ok());
}

#[test]
fn technical_fit_override_drops_empty_fields() {
    let override_selection = RuntimeTechnicalFitOverride {
        runtime_id: None,
        runtime_variant_id: None,
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
        selected_runtime_variant_id: Some(" llama-cpp/linux-x64/cuda ".to_string()),
        selected_backend_key: Some("llama.cpp".to_string()),
        selected_model_id: Some(" model-a ".to_string()),
        selected_device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
        selected_device_id: Some(" cuda:0 ".to_string()),
        resource_estimate: Some(RuntimeTechnicalFitResourceEstimate {
            estimated_peak_vram_mb: Some(4096),
            estimated_peak_ram_mb: Some(8192),
            estimated_min_vram_mb: None,
            estimated_min_ram_mb: None,
        }),
        observed_throughput_hint: Some(RuntimeTechnicalFitObservedThroughputHint {
            tokens_per_second_milli: Some(33000),
            images_per_second_milli: None,
            sample_count: Some(5),
        }),
        device_diagnostics: vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning,
            message: " cuda runtime warmup pending ".to_string(),
            device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
            device_id: Some(" cuda:0 ".to_string()),
            runtime_variant_id: Some(" llama-cpp/linux-x64/cuda ".to_string()),
            backend_key: Some("llama.cpp".to_string()),
        }],
        reasons: vec![RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
            Some(" candidate-1 "),
        )],
        selection_policy_trace: Some(RuntimeTechnicalFitSelectionPolicyTrace {
            policy_version: 1,
            policy_phase: Some(RuntimeTechnicalFitPolicyPhase::CandidateRanking),
            decision_code: Some(RuntimeTechnicalFitDecisionCode::SelectedCandidate),
            history_threshold_state: Some(RuntimeTechnicalFitHistoryThresholdState::NotEvaluated),
            candidate_set_summary: Some(RuntimeTechnicalFitCandidateSetSummary {
                total_candidate_count: 2,
                eligible_candidate_count: 1,
                rejected_candidate_count: 1,
                eligible_candidate_ids: vec![" candidate-1 ".to_string()],
            }),
            ranking_reason: Some(" explicit_backend_override ".to_string()),
            exploration_reason: None,
            seed_basis: Some(" workflow-a:node-a ".to_string()),
        }),
        compatibility_report: Some(RuntimeTechnicalFitCompatibilityReport {
            status: " rejected ".to_string(),
            compatible: false,
            task: " supported ".to_string(),
            model_source: " unsupported ".to_string(),
            preprocessing: " supported ".to_string(),
            postprocessing: " supported ".to_string(),
        }),
        compatibility_issue_count: 1,
        compatibility_issues: vec![RuntimeTechnicalFitCompatibilityIssue {
            kind: " unsupported_model_artifact ".to_string(),
            phase: " model_package_resolution ".to_string(),
            message: " backend cannot load artifact ".to_string(),
            model_id: Some(" model-a ".to_string()),
            path: Some(" model.gguf ".to_string()),
        }],
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
        normalized.selected_runtime_variant_id.as_deref(),
        Some("llama-cpp/linux-x64/cuda")
    );
    assert_eq!(normalized.selected_device_id.as_deref(), Some("cuda:0"));
    assert_eq!(
        normalized
            .resource_estimate
            .as_ref()
            .and_then(|estimate| estimate.estimated_peak_vram_mb),
        Some(4096)
    );
    assert_eq!(
        normalized
            .observed_throughput_hint
            .as_ref()
            .and_then(|hint| hint.tokens_per_second_milli),
        Some(33000)
    );
    assert_eq!(
        normalized.device_diagnostics[0].backend_key.as_deref(),
        Some("llama_cpp")
    );
    assert_eq!(
        normalized.reasons,
        vec![RuntimeTechnicalFitReason {
            code: RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
            candidate_id: Some("candidate-1".to_string()),
        }]
    );
    let trace = normalized
        .selection_policy_trace
        .as_ref()
        .expect("selection policy trace should normalize");
    assert_eq!(trace.policy_version, 1);
    assert_eq!(
        trace.policy_phase,
        Some(RuntimeTechnicalFitPolicyPhase::CandidateRanking)
    );
    assert_eq!(
        trace.decision_code,
        Some(RuntimeTechnicalFitDecisionCode::SelectedCandidate)
    );
    assert_eq!(
        trace.history_threshold_state,
        Some(RuntimeTechnicalFitHistoryThresholdState::NotEvaluated)
    );
    assert_eq!(
        trace.ranking_reason.as_deref(),
        Some("explicit_backend_override")
    );
    assert_eq!(trace.seed_basis.as_deref(), Some("workflow-a:node-a"));
    assert_eq!(
        trace
            .candidate_set_summary
            .as_ref()
            .map(|summary| summary.eligible_candidate_ids.clone()),
        Some(vec!["candidate-1".to_string()])
    );
    assert_eq!(
        normalized
            .compatibility_report
            .as_ref()
            .map(|report| (report.status.as_str(), report.model_source.as_str())),
        Some(("rejected", "unsupported"))
    );
    assert_eq!(normalized.compatibility_issue_count, 1);
    assert_eq!(
        normalized.compatibility_issues[0].kind,
        "unsupported_model_artifact"
    );
    assert_eq!(
        normalized.compatibility_issues[0].model_id.as_deref(),
        Some("model-a")
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
                runtime_snapshot(
                    "runtime-b",
                    vec!["pytorch"],
                    RuntimeRegistryStatus::Ready,
                    0,
                ),
            ],
            reservations: Vec::new(),
        },
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: Vec::new(),
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: Some(RuntimeTechnicalFitOverride {
            runtime_id: None,
            runtime_variant_id: None,
            model_id: None,
            backend_key: Some("pytorch".to_string()),
        }),
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-a".to_string(),
                runtime_id: Some("runtime-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-b".to_string(),
                runtime_id: Some("runtime-b".to_string()),
                backend_key: Some("pytorch".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
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
            selected_runtime_variant_id: None,
            selected_backend_key: Some("pytorch".to_string()),
            selected_model_id: None,
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            reasons: vec![RuntimeTechnicalFitReason {
                code: RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
                candidate_id: Some("runtime-b".to_string()),
            }],
            selection_policy_trace: None,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        }
    );
}

#[test]
fn selector_rejects_ineligible_explicit_backend_override_without_selection() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: Vec::new(),
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: Some(RuntimeTechnicalFitOverride {
            runtime_id: None,
            runtime_variant_id: None,
            model_id: None,
            backend_key: Some("llama_cpp".to_string()),
        }),
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: "llama-image".to_string(),
            runtime_id: Some("llama_cpp".to_string()),
            runtime_variant_id: Some("llama_cpp/linux-x64/cpu".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            model_id: Some("image-model".to_string()),
            device_class: Some(RuntimeTechnicalFitDeviceClass::Cpu),
            selected_device_id: Some("cpu".to_string()),
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: vec![RuntimeTechnicalFitDeviceDiagnostic {
                code: RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible,
                severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
                message: "llama.cpp cannot execute diffusion image generation".to_string(),
                device_class: Some(RuntimeTechnicalFitDeviceClass::Cpu),
                device_id: Some("cpu".to_string()),
                runtime_variant_id: Some("llama_cpp/linux-x64/cpu".to_string()),
                backend_key: Some("llama_cpp".to_string()),
            }],
            source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
            context_window_tokens: Some(8192),
            residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
            warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
            supports_runtime_requirements: false,
            compatibility_report: Some(RuntimeTechnicalFitCompatibilityReport {
                status: "rejected".to_string(),
                compatible: false,
                task: "unsupported".to_string(),
                model_source: "supported".to_string(),
                preprocessing: "supported".to_string(),
                postprocessing: "supported".to_string(),
            }),
            compatibility_issue_count: 1,
            compatibility_issues: vec![RuntimeTechnicalFitCompatibilityIssue {
                kind: "unsupported_task".to_string(),
                phase: "task_validation".to_string(),
                message: "backend cannot execute image generation".to_string(),
                model_id: Some("image-model".to_string()),
                path: None,
            }],
        }],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::ExplicitOverride
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert_eq!(decision.selected_backend_key, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ExplicitBackendOverride
            && reason.candidate_id.is_none()
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingCandidateData
            && reason.candidate_id.is_none()
    }));
    assert_eq!(
        decision.device_diagnostics,
        vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "llama.cpp cannot execute diffusion image generation".to_string(),
            device_class: Some(RuntimeTechnicalFitDeviceClass::Cpu),
            device_id: Some("cpu".to_string()),
            runtime_variant_id: Some("llama_cpp/linux-x64/cpu".to_string()),
            backend_key: Some("llama_cpp".to_string()),
        }]
    );
}

#[test]
fn selector_honors_explicit_runtime_variant_override() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["pytorch".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: Some(RuntimeTechnicalFitOverride {
            runtime_id: Some("pytorch".to_string()),
            runtime_variant_id: Some("pytorch/linux-x64/cuda".to_string()),
            model_id: None,
            backend_key: Some("pytorch".to_string()),
        }),
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "pytorch-cpu".to_string(),
                runtime_id: Some("pytorch".to_string()),
                runtime_variant_id: Some("pytorch/linux-x64/cpu".to_string()),
                backend_key: Some("pytorch".to_string()),
                model_id: None,
                device_class: Some(RuntimeTechnicalFitDeviceClass::Cpu),
                selected_device_id: Some("cpu".to_string()),
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "pytorch-cuda".to_string(),
                runtime_id: Some("pytorch".to_string()),
                runtime_variant_id: Some("pytorch/linux-x64/cuda".to_string()),
                backend_key: Some("pytorch".to_string()),
                model_id: None,
                device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
                selected_device_id: Some("cuda:0".to_string()),
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
        ],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selected_candidate_id.as_deref(),
        Some("pytorch-cuda")
    );
    assert_eq!(
        decision.selected_runtime_variant_id.as_deref(),
        Some("pytorch/linux-x64/cuda")
    );
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ExplicitRuntimeOverride
            && reason.candidate_id.as_deref() == Some("pytorch-cuda")
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ExplicitRuntimeVariantOverride
            && reason.candidate_id.as_deref() == Some("pytorch-cuda")
    }));
}

#[test]
fn selector_rejects_unmatched_runtime_variant_override_without_synthetic_candidate() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["pytorch".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: Some(RuntimeTechnicalFitOverride {
            runtime_id: Some("pytorch".to_string()),
            runtime_variant_id: Some("pytorch/linux-x64/cuda".to_string()),
            model_id: None,
            backend_key: Some("pytorch".to_string()),
        }),
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: "pytorch-cpu".to_string(),
            runtime_id: Some("pytorch".to_string()),
            runtime_variant_id: Some("pytorch/linux-x64/cpu".to_string()),
            backend_key: Some("pytorch".to_string()),
            model_id: None,
            device_class: Some(RuntimeTechnicalFitDeviceClass::Cpu),
            selected_device_id: Some("cpu".to_string()),
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
            context_window_tokens: Some(8192),
            residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
            warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
            supports_runtime_requirements: true,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        }],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::ExplicitOverride
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ExplicitRuntimeVariantOverride
            && reason.candidate_id.is_none()
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingCandidateData
            && reason.candidate_id.is_none()
    }));
    assert_eq!(
        decision.device_diagnostics,
        vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "technical-fit could not satisfy the explicit runtime variant override"
                .to_string(),
            device_class: None,
            device_id: None,
            runtime_variant_id: Some("pytorch/linux-x64/cuda".to_string()),
            backend_key: Some("pytorch".to_string()),
        }]
    );
}

#[test]
fn selector_rejects_unavailable_explicit_device_without_cpu_fallback() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["pytorch".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        device_policy: Some(RuntimeTechnicalFitDevicePolicy::Explicit {
            device_class: RuntimeTechnicalFitDeviceClass::Cuda,
            device_id: Some("cuda:0".to_string()),
        }),
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: "pytorch-cpu".to_string(),
            runtime_id: Some("pytorch".to_string()),
            runtime_variant_id: Some("pytorch/linux-x64/cpu".to_string()),
            backend_key: Some("pytorch".to_string()),
            model_id: None,
            device_class: Some(RuntimeTechnicalFitDeviceClass::Cpu),
            selected_device_id: Some("cpu".to_string()),
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
            context_window_tokens: Some(8192),
            residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
            warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
            supports_runtime_requirements: true,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        }],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::Automatic
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert_eq!(decision.selected_runtime_id, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingCandidateData
            && reason.candidate_id.as_deref() == Some("pytorch-cpu")
    }));
    assert_eq!(
        decision.device_diagnostics,
        vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "technical-fit could not satisfy the explicit device policy".to_string(),
            device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
            device_id: Some("cuda:0".to_string()),
            runtime_variant_id: None,
            backend_key: None,
        }]
    );
}

#[test]
fn selector_honors_explicit_device_when_candidate_facts_match() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["pytorch".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        device_policy: Some(RuntimeTechnicalFitDevicePolicy::Explicit {
            device_class: RuntimeTechnicalFitDeviceClass::Cuda,
            device_id: Some("cuda:0".to_string()),
        }),
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "pytorch-cpu".to_string(),
                runtime_id: Some("pytorch".to_string()),
                runtime_variant_id: Some("pytorch/linux-x64/cpu".to_string()),
                backend_key: Some("pytorch".to_string()),
                model_id: None,
                device_class: Some(RuntimeTechnicalFitDeviceClass::Cpu),
                selected_device_id: Some("cpu".to_string()),
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "pytorch-cuda".to_string(),
                runtime_id: Some("pytorch".to_string()),
                runtime_variant_id: Some("pytorch/linux-x64/cuda".to_string()),
                backend_key: Some("pytorch".to_string()),
                model_id: None,
                device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
                selected_device_id: Some("cuda:0".to_string()),
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
        ],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selected_candidate_id.as_deref(),
        Some("pytorch-cuda")
    );
    assert_eq!(
        decision.selected_runtime_variant_id.as_deref(),
        Some("pytorch/linux-x64/cuda")
    );
    assert_eq!(
        decision.selected_device_class,
        Some(RuntimeTechnicalFitDeviceClass::Cuda)
    );
    assert_eq!(decision.selected_device_id.as_deref(), Some("cuda:0"));
    assert!(decision.device_diagnostics.is_empty());
}

#[test]
fn selector_rejects_unmatched_override_without_synthetic_candidate() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: Vec::new(),
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: Some(RuntimeTechnicalFitOverride {
            runtime_id: None,
            runtime_variant_id: None,
            model_id: Some("model-b".to_string()),
            backend_key: Some("pytorch".to_string()),
        }),
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: "runtime-a".to_string(),
            runtime_id: Some("runtime-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            model_id: Some("model-a".to_string()),
            runtime_variant_id: None,
            device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
            context_window_tokens: Some(8192),
            residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
            warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
            supports_runtime_requirements: true,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        }],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::ExplicitOverride
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert_eq!(decision.selected_runtime_id, None);
    assert_eq!(decision.selected_backend_key, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ExplicitModelOverride
            && reason.candidate_id.is_none()
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ExplicitBackendOverride
            && reason.candidate_id.is_none()
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingCandidateData
            && reason.candidate_id.is_none()
    }));
}

#[test]
fn selector_uses_controlled_exploration_for_equal_ranked_auto_candidates() {
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
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-b".to_string(),
                runtime_id: Some("runtime-b".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-a".to_string(),
                runtime_id: Some("runtime-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
        ],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::Automatic
    );
    let selected_candidate_id = decision
        .selected_candidate_id
        .as_deref()
        .expect("equal-ranked valid candidates should select through controlled exploration");
    assert!(matches!(selected_candidate_id, "runtime-a" | "runtime-b"));
    assert_eq!(
        decision.selected_runtime_id.as_deref(),
        Some(selected_candidate_id)
    );
    assert!(decision.device_diagnostics.is_empty());
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::AutomaticRanking
            && reason.candidate_id.as_deref() == Some(selected_candidate_id)
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::ControlledExploration
            && reason.candidate_id.as_deref() == Some(selected_candidate_id)
    }));

    let selection_policy_trace = decision
        .selection_policy_trace
        .as_ref()
        .expect("automatic selection should record its policy trace");
    assert_eq!(selection_policy_trace.policy_version, 1);
    assert_eq!(
        selection_policy_trace.ranking_reason.as_deref(),
        Some("candidate_priority")
    );
    assert_eq!(
        selection_policy_trace.exploration_reason.as_deref(),
        Some("equal_priority_seeded_choice")
    );
    assert_eq!(
        selection_policy_trace.seed_basis.as_deref(),
        Some("workflow:workflow-a|snapshot:123|candidates:runtime-a,runtime-b")
    );
    let candidate_set_summary = selection_policy_trace
        .candidate_set_summary
        .as_ref()
        .expect("automatic selection should summarize the candidate set");
    assert_eq!(candidate_set_summary.total_candidate_count, 2);
    assert_eq!(candidate_set_summary.eligible_candidate_count, 2);
    assert_eq!(candidate_set_summary.rejected_candidate_count, 0);
    assert_eq!(
        candidate_set_summary.eligible_candidate_ids,
        vec!["runtime-a".to_string(), "runtime-b".to_string()]
    );
}

#[test]
fn selector_rejects_when_required_context_is_missing() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["llama_cpp".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: Some(8192),
        override_selection: None,
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: "runtime-a".to_string(),
            runtime_id: Some("runtime-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            model_id: None,
            runtime_variant_id: None,
            device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
            context_window_tokens: None,
            residency_state: None,
            warmup_state: None,
            supports_runtime_requirements: true,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        }],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::Automatic
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert_eq!(decision.selected_runtime_id, None);
    assert_eq!(decision.selected_backend_key, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingRuntimeState
            && reason.candidate_id.as_deref() == Some("runtime-a")
    }));
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingCandidateData
            && reason.candidate_id.as_deref() == Some("runtime-a")
    }));
    assert_eq!(
        decision.device_diagnostics,
        vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "technical-fit auto policy found no valid candidate".to_string(),
            device_class: None,
            device_id: None,
            runtime_variant_id: None,
            backend_key: None,
        }]
    );
}

#[test]
fn selector_rejects_required_backend_candidate_without_fallback_selection() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: Vec::new(),
        required_backend_keys: vec!["llama_cpp".to_string()],
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "candle".to_string(),
                runtime_id: Some("candle".to_string()),
                backend_key: Some("candle".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "llama_cpp".to_string(),
                runtime_id: Some("llama_cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Unloaded),
                warmup_state: None,
                supports_runtime_requirements: false,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
        ],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::Automatic
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert_eq!(decision.selected_runtime_id, None);
    assert_eq!(decision.selected_backend_key, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingCandidateData
            && reason.candidate_id.as_deref() == Some("llama_cpp")
    }));
    assert_eq!(
        decision.device_diagnostics,
        vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "technical-fit auto policy found no valid candidate".to_string(),
            device_class: None,
            device_id: None,
            runtime_variant_id: None,
            backend_key: None,
        }]
    );
}

#[test]
fn selector_surfaces_scoped_candidate_diagnostics_when_no_candidate_is_valid() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: empty_snapshot(),
        workflow_id: Some("workflow-a".to_string()),
        required_model_ids: vec!["model-a".to_string()],
        required_backend_keys: Vec::new(),
        required_extensions: Vec::new(),
        required_context_window_tokens: None,
        override_selection: None,
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![RuntimeTechnicalFitCandidate {
            candidate_id: "missing_model_package_facts|model-a".to_string(),
            runtime_id: None,
            backend_key: None,
            model_id: Some("model-a".to_string()),
            runtime_variant_id: None,
            device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: vec![RuntimeTechnicalFitDeviceDiagnostic {
                code: RuntimeTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts,
                severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
                message: "required model did not resolve to Pumas package facts".to_string(),
                device_class: None,
                device_id: None,
                runtime_variant_id: None,
                backend_key: None,
            }],
            source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
            context_window_tokens: None,
            residency_state: None,
            warmup_state: None,
            supports_runtime_requirements: false,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        }],
        resource_pressure: None,
    });

    assert_eq!(
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::Automatic
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::MissingCandidateData
            && reason.candidate_id.as_deref() == Some("missing_model_package_facts|model-a")
    }));
    assert_eq!(
        decision.device_diagnostics,
        vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "required model did not resolve to Pumas package facts".to_string(),
            device_class: None,
            device_id: None,
            runtime_variant_id: None,
            backend_key: None,
        }]
    );
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
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-hot".to_string(),
                runtime_id: Some("runtime-hot".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-cool".to_string(),
                runtime_id: Some("runtime-cool".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
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
fn selector_rejects_unrankable_headroom_under_queue_pressure() {
    let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
        runtime_snapshot: RuntimeRegistrySnapshot {
            generated_at_ms: 123,
            runtimes: vec![
                runtime_snapshot(
                    "runtime-overflow",
                    vec!["llama_cpp"],
                    RuntimeRegistryStatus::Ready,
                    u16::MAX as usize + 1,
                ),
                runtime_snapshot(
                    "runtime-roomy",
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
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-overflow".to_string(),
                runtime_id: Some("runtime-overflow".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-roomy".to_string(),
                runtime_id: Some("runtime-roomy".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
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
        decision.selection_mode,
        RuntimeTechnicalFitSelectionMode::Automatic
    );
    assert_eq!(decision.selected_candidate_id, None);
    assert!(decision.reasons.iter().any(|reason| {
        reason.code == RuntimeTechnicalFitReasonCode::QueuePressure
            && reason.candidate_id.as_deref() == Some("runtime-overflow")
    }));
    assert_eq!(decision.device_diagnostics.len(), 1);
    assert_eq!(
        decision.device_diagnostics[0].code,
        RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate
    );
    assert_eq!(
        decision.device_diagnostics[0].severity,
        RuntimeTechnicalFitDeviceDiagnosticSeverity::Error
    );
    assert!(decision.device_diagnostics[0]
        .message
        .contains("active reservation count exceeds the supported range"));
    assert_eq!(
        decision.device_diagnostics[0].backend_key.as_deref(),
        Some("llama_cpp")
    );
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
        device_policy: None,
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: vec![
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-tight".to_string(),
                runtime_id: Some("runtime-tight".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            },
            RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-roomy".to_string(),
                runtime_id: Some("runtime-roomy".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                runtime_variant_id: None,
                device_class: None,
                selected_device_id: None,
                resource_estimate: None,
                observed_throughput_hint: None,
                device_diagnostics: Vec::new(),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                context_window_tokens: Some(8192),
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
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
