//! Native, advisory forecasts. No successor admission, lease or fabricated input.
use crate::runtime_dispatch_completion_timing::{
    bounded_inputs, bounded_text, diagnosed_selection, duration, query_bounded,
    EmbeddedCompletionTimingOptIn, EmbeddedCompletionTimingQuery,
};
use inference::RuntimeServiceTimingValue;
use pantograph_runtime_registry::{RuntimeRegistryStatus, RuntimeReservationAdmissionObservation};
use pantograph_scheduler::*;
use pantograph_workflow_service::workflow::WorkflowCompletionSuccessorSnapshot;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedCompletionSuccessorPlacement {
    pub candidate: SchedulerDispatchCandidate,
    pub artifact_fingerprint: String,
    pub backend_key: String,
    pub runtime_residency_key: String,
    pub resource_requirements: pantograph_runtime_registry::RuntimeReservationRequirements,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedCompletionReleaseCondition {
    /// Actual normal Ephemeral lease release, resource reconciliation and producer
    /// reclamation must succeed. Completion state alone does not satisfy this.
    SuccessfulEphemeralReleaseAndReconciliation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddedCompletionProjectedResidency {
    Unloaded,
    /// Must be independently qualified after release, e.g. another owner lease.
    Retained {
        runtime_instance_id: String,
    },
}
/// Symbolic output obligations stay explicit: a source must qualify their class
/// and costs, or return None. Echo association does not prove calibration history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedDependencyCompletionQuery {
    pub predecessor: EmbeddedCompletionTimingQuery,
    pub successor: Arc<WorkflowCompletionSuccessorSnapshot>,
    pub placement: EmbeddedCompletionSuccessorPlacement,
    pub release_condition: EmbeddedCompletionReleaseCondition,
}
#[derive(Debug, Clone)]
pub struct EmbeddedDependencyCompletionRecord {
    pub query: EmbeddedDependencyCompletionQuery,
    pub observed_at_ms: u64,
    pub successful_sample_count: u32,
    /// Shared post-first/pre-second owner state, equal across alternative successor
    /// placements for this predecessor. Association label, not authentication.
    pub post_cleanup_state_fingerprint: String,
    /// Qualified post-release/reconciliation accounting observation, never lease.
    pub resource_observation: RuntimeReservationAdmissionObservation,
    pub resource_fit: SchedulerResourceFitAssessment,
    pub predecessor_residency: EmbeddedCompletionProjectedResidency,
    pub successor_residency: EmbeddedCompletionProjectedResidency,
    /// All three must be Some for Fits, all None for qualified infeasibility.
    pub preparation: Option<RuntimeServiceTimingValue>,
    pub required_transfer: Option<RuntimeServiceTimingValue>,
    pub execution: Option<RuntimeServiceTimingValue>,
}
pub(crate) struct EmbeddedSuccessorOffers {
    pub snapshot: Arc<WorkflowCompletionSuccessorSnapshot>,
    pub request: ValidatedSchedulerCompletionSuccessor,
    pub placements: Vec<EmbeddedCompletionSuccessorPlacement>,
}

pub(crate) fn select_dependency_completion(
    first: &ValidatedSchedulerDispatchSelectionRequest,
    first_rows: &[SchedulerCompletionEvidence<'_>],
    queries: &[Option<EmbeddedCompletionTimingQuery>],
    future: &EmbeddedSuccessorOffers,
    opt_in: &EmbeddedCompletionTimingOptIn,
    baseline: SchedulerCompletionRankingResult,
) -> SchedulerDispatchReservationSelection {
    let fallback = || {
        let mut selection = diagnosed_selection(select_scheduler_candidate_with_completion(
            first,
            first_rows,
            SchedulerCompletionRankingPolicy {
                now_ms: crate::runtime_dispatch_candidate_provider::current_time_ms(),
                max_sample_age_ms: opt_in.max_sample_age_ms,
                minimum_samples: 1,
                allow_synthetic: opt_in.allow_configured_estimates,
            },
        ));
        let diagnostics = match &mut selection {
            SchedulerDispatchReservationSelection::Selected { diagnostics, .. }
            | SchedulerDispatchReservationSelection::NoSelection { diagnostics } => diagnostics,
        };
        diagnostics.push(SchedulerDispatchSelectionDiagnostic { severity: SchedulerDispatchSelectionDiagnosticSeverity::Info,
            code:SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,candidate_id:None,
            message:"Owned dependency forecast unavailable, incomplete, expired or outside its supported bounds; re-evaluated frozen first-task evidence.".into(),
            hint:Some("embedded_runtime_dispatch.dependency_completion.fallback".into()) });
        selection
    };
    let source = match baseline.diagnostic {
        SchedulerCompletionRankingDiagnostic::Ranked { source, .. } => source,
        _ => return fallback(),
    };
    let minimum = if source == SchedulerCompletionEvidenceSource::Measured {
        3
    } else {
        1
    };
    // Fixed native callback bound, before invoking conditional producer code.
    if !opt_in.source.dependency_lookahead_enabled()
        || first.as_ref().candidates.len() > 4
        || future.placements.is_empty()
        || future.placements.len() != future.request.as_ref().candidates.len()
        || future
            .placements
            .iter()
            .zip(&future.request.as_ref().candidates)
            .any(|(p, c)| p.candidate != *c)
        || future.placements.len() > 4
        || first_rows.len() != queries.len()
        || first_rows.iter().any(|r| r.sample.sample_count < minimum)
        || !bounded_inputs(&future.snapshot.known_inputs)
    {
        return fallback();
    }
    let mut records = Vec::new();
    for (i, _) in first.as_ref().candidates.iter().enumerate() {
        let Some(predecessor) = queries.get(i).and_then(Option::as_ref) else {
            return fallback();
        };
        for placement in &future.placements {
            let query = EmbeddedDependencyCompletionQuery {
                predecessor: predecessor.clone(),
                successor: future.snapshot.clone(),
                placement: placement.clone(),
                release_condition:
                    EmbeddedCompletionReleaseCondition::SuccessfulEphemeralReleaseAndReconciliation,
            };
            let Some(record) = opt_in.source.timing_after_successful_completion(&query) else {
                return fallback();
            };
            // Bounds BEFORE equality: trusted producer still cannot make equality
            // scan arbitrary collections. Owner-qualified known future observations
            // cannot replace the authoritative actual successor commit.
            if !Arc::ptr_eq(&record.query.successor, &query.successor)
                || !Arc::ptr_eq(
                    &record.query.predecessor.materialized_inputs,
                    &query.predecessor.materialized_inputs,
                )
                || !record_bounded(&record)
                || record.query != query
                || !projection_qualified(&record)
                || crate::runtime_dispatch_candidate_provider::current_time_ms()
                    .checked_sub(record.observed_at_ms)
                    .is_none_or(|age| age > opt_in.max_sample_age_ms)
                || crate::runtime_dispatch_candidate_provider::current_time_ms()
                    .checked_sub(record.resource_observation.observed_at_ms)
                    .is_none_or(|age| age > opt_in.max_sample_age_ms)
                || record.successful_sample_count < minimum
            {
                return fallback();
            }
            if records
                .iter()
                .rev()
                .take(placements_already(&records, future.placements.len()))
                .any(|old: &EmbeddedDependencyCompletionRecord| {
                    old.post_cleanup_state_fingerprint != record.post_cleanup_state_fingerprint
                        || old.predecessor_residency != record.predecessor_residency
                })
            {
                return fallback();
            }
            records.push(record);
        }
    }
    let mut rows = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let predecessor = &first.as_ref().candidates[index / future.placements.len()];
        let candidate = &future.request.as_ref().candidates[index % future.placements.len()];
        let Some(first_row) = first_rows
            .iter()
            .find(|r| std::ptr::eq(r.candidate, predecessor))
        else {
            return fallback();
        };
        let context = SchedulerCompletionContext {
            host_id: &opt_in.owner_epoch,
            runtime_instance_id: match &record.successor_residency {
                EmbeddedCompletionProjectedResidency::Unloaded => "qualified-post-release-unloaded",
                EmbeddedCompletionProjectedResidency::Retained {
                    runtime_instance_id,
                } => runtime_instance_id,
            },
            artifact_fingerprint: &record.query.placement.artifact_fingerprint,
            workload_fingerprint: &future.request.as_ref().snapshot_identity,
            resource_condition_fingerprint: &record.post_cleanup_state_fingerprint,
            residency_fingerprint: &record.query.placement.runtime_residency_key,
            timing_convention: "owner-serialized-service-us-v1",
        };
        let sample = if record.resource_fit.state == SchedulerResourceFitState::Fits {
            let Some((prep, src)) = record.preparation.as_ref().and_then(duration) else {
                return fallback();
            };
            let Some((transfer, transfer_src)) =
                record.required_transfer.as_ref().and_then(duration)
            else {
                return fallback();
            };
            let Some((exec, exec_src)) = record.execution.as_ref().and_then(duration) else {
                return fallback();
            };
            if src != source || src != transfer_src || src != exec_src {
                return fallback();
            }
            Some(SchedulerCompletionSample {
                candidate,
                context,
                source: src,
                sample_count: record.successful_sample_count,
                observed_at_ms: record.observed_at_ms,
                preparation_us: Some(prep),
                required_transfer_us: Some(transfer),
                execution_us: Some(exec),
            })
        } else {
            None
        };
        rows.push(SchedulerDependencyCompletionEvidence {
            successor: &future.request,
            predecessor_candidate: predecessor,
            predecessor_context: first_row.current_context,
            candidate,
            context,
            transition_fingerprint: &record.post_cleanup_state_fingerprint,
            resource_fit: &record.resource_fit,
            sample,
        });
    }
    let final_now_ms = crate::runtime_dispatch_candidate_provider::current_time_ms();
    // Capacity and qualified infeasibility also expire, including rows without
    // timing samples. Later callbacks must not keep an earlier projection alive.
    if records.iter().any(|r| {
        [r.observed_at_ms, r.resource_observation.observed_at_ms]
            .into_iter()
            .any(|at| {
                final_now_ms
                    .checked_sub(at)
                    .is_none_or(|age| age > opt_in.max_sample_age_ms)
            })
    }) {
        return fallback();
    }
    let result = select_scheduler_candidate_with_dependency_completion(
        first,
        first_rows,
        &future.request,
        &rows,
        SchedulerCompletionRankingPolicy {
            now_ms: final_now_ms,
            max_sample_age_ms: opt_in.max_sample_age_ms,
            minimum_samples: minimum,
            allow_synthetic: opt_in.allow_configured_estimates,
        },
    );
    // Preserve exact one-decision fallback selection and its diagnostics.
    if !matches!(
        result.diagnostic,
        SchedulerTwoCompletionDiagnostic::Ranked(_)
    ) {
        return fallback();
    }
    let mut selection = result.selection;
    if let SchedulerDispatchReservationSelection::Selected { diagnostics, .. } = &mut selection {
        diagnostics.push(SchedulerDispatchSelectionDiagnostic { severity: SchedulerDispatchSelectionDiagnosticSeverity::Info,
            code: SchedulerDispatchSelectionDiagnosticCode::CandidateSelected, candidate_id: None, hint: Some("embedded_runtime_dispatch.dependency_completion".into()),
            message: format!("Owned dependency completion forecast: {:?}; Synthetic means owner-configured estimate.", result.diagnostic) });
    }
    selection
}
fn record_bounded(r: &EmbeddedDependencyCompletionRecord) -> bool {
    // The query owns only bounded owner snapshots. Reject foreign oversized
    // snapshots before equality; serialize with a capped writer via local check.
    bounded_text(&r.post_cleanup_state_fingerprint)
        && query_bounded(&r.query.predecessor)
        && [
            &r.query.placement.artifact_fingerprint,
            &r.query.placement.backend_key,
            &r.query.placement.runtime_residency_key,
        ]
        .into_iter()
        .all(|s| bounded_text(s))
        && r.query
            .placement
            .candidate
            .selected_model_ref
            .model_id
            .len()
            <= 128
        && r.query
            .placement
            .candidate
            .selected_model_ref
            .revision
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && r.query
            .placement
            .candidate
            .selected_model_ref
            .selected_artifact_id
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && r.query
            .placement
            .candidate
            .selected_model_ref
            .selected_artifact_path
            .is_none()
        && r.query
            .placement
            .candidate
            .selected_model_ref
            .migration_diagnostics
            .is_empty()
        && r.query.placement.candidate.runtime_trait_settings.len() <= 32
        && r.query
            .placement
            .candidate
            .runtime_trait_settings
            .iter()
            .all(|t| match &t.value {
                SchedulerTraitValue::String(s) => s.len() <= 1024,
                _ => true,
            })
        && r.query
            .placement
            .candidate
            .resource_fit_assessment
            .is_none()
        && r.query
            .placement
            .candidate
            .candidate_source_diagnostics
            .is_empty()
        && r.query.placement.resource_requirements.claims.len() <= 8
        && r.query.placement.candidate.reservations.is_empty()
        && r.query.placement.candidate.selected_device_ids.len() <= 8
        && r.resource_observation.resources.len() <= 8
        && r.resource_observation.resource_domains.len() <= 8
        && r.resource_fit.diagnostics.len() <= 32
        && r.resource_fit
            .diagnostics
            .iter()
            .all(|d| d.message.len() <= 1024 && d.hint.as_ref().is_none_or(|s| s.len() <= 1024))
        && bounded_text(&r.resource_observation.runtime_id)
        && r.resource_observation
            .runtime_instance_id
            .as_deref()
            .is_none_or(bounded_text)
        && [&r.predecessor_residency, &r.successor_residency]
            .iter()
            .all(|s| match s {
                EmbeddedCompletionProjectedResidency::Unloaded => true,
                EmbeddedCompletionProjectedResidency::Retained {
                    runtime_instance_id,
                } => bounded_text(runtime_instance_id),
            })
}
fn projection_qualified(r: &EmbeddedDependencyCompletionRecord) -> bool {
    let observation = &r.resource_observation;
    if r.query.predecessor.runtime_id == r.query.placement.candidate.selected_runtime_id.as_str()
        && r.predecessor_residency != r.successor_residency
    {
        return false;
    }
    if observation.runtime_id != r.query.placement.candidate.selected_runtime_id.as_str()
        || observation.replaces_reservation_id.is_some()
        || observation.resources.is_empty()
        || observation
            .resources
            .iter()
            .any(|v| v.capacity_bytes.is_none() || v.available_bytes.is_none())
        || matches!(r.resource_fit.state, SchedulerResourceFitState::Unknown)
    {
        return false;
    }
    if r.query.placement.resource_requirements.claims.is_empty()
        || r.query
            .placement
            .resource_requirements
            .claims
            .iter()
            .any(|claim| {
                observation
                    .resources
                    .iter()
                    .filter(|v| v.kind == claim.kind && v.requested_bytes == claim.bytes)
                    .count()
                    != 1
            })
        || observation.resources.iter().any(|v| {
            v.requested_bytes != 0
                && !r
                    .query
                    .placement
                    .resource_requirements
                    .claims
                    .iter()
                    .any(|claim| v.kind == claim.kind && v.requested_bytes == claim.bytes)
        })
    {
        return false;
    }
    if observation.resources.iter().any(|v| {
        v.resident_bytes > v.reserved_bytes
            || v.capacity_bytes.and_then(|capacity| {
                capacity
                    .checked_sub(v.safety_margin_bytes)?
                    .checked_sub(v.reserved_bytes)
            }) != v.available_bytes
    }) || observation.resource_domains.iter().any(|v| {
        v.resident_bytes > v.reserved_bytes
            || v.available_bytes
                != v.total_bytes
                    .saturating_sub(v.safety_margin_bytes)
                    .saturating_sub(v.reserved_bytes)
            || (v.host_ram_capacity_source_bound && v.owner_capacity_ceiling_bytes.is_none())
    }) {
        return false;
    }
    let capacity_fits = observation
        .resources
        .iter()
        .all(|v| v.available_bytes.is_some_and(|a| a >= v.requested_bytes))
        && observation
            .resource_domains
            .iter()
            .all(|v| v.available_bytes >= v.requested_bytes);
    match &r.successor_residency {
        EmbeddedCompletionProjectedResidency::Unloaded
            if observation.runtime_status == RuntimeRegistryStatus::Stopped
                && observation.runtime_instance_id.is_none() => {}
        EmbeddedCompletionProjectedResidency::Retained {
            runtime_instance_id,
        } if observation.runtime_status == RuntimeRegistryStatus::Ready
            && observation.runtime_instance_id.as_ref() == Some(runtime_instance_id) => {}
        _ => return false,
    }
    match r.resource_fit.state {
        SchedulerResourceFitState::Fits => {
            capacity_fits
                && r.preparation.is_some()
                && r.required_transfer.is_some()
                && r.execution.is_some()
        }
        SchedulerResourceFitState::WaitingForResources
        | SchedulerResourceFitState::ImpossibleFit => {
            !capacity_fits
                && r.preparation.is_none()
                && r.required_transfer.is_none()
                && r.execution.is_none()
        }
        _ => false,
    }
}

fn placements_already(records: &[EmbeddedDependencyCompletionRecord], count: usize) -> usize {
    records.len() % count
}
