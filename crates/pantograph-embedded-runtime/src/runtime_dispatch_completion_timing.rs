//! Native startup opt-in. These records are never accepted from session JSON.
use inference::{RuntimeServiceTimingOutcome, RuntimeServiceTimingValue};
use pantograph_runtime_registry::RuntimeReservationAdmissionObservation;
use pantograph_scheduler::{
    completion_diagnostics_bounded, select_scheduler_candidate_with_completion,
    SchedulerCompletionContext, SchedulerCompletionEvidence, SchedulerCompletionEvidenceSource,
    SchedulerCompletionRankingPolicy, SchedulerCompletionSample,
    SchedulerDispatchReservationSelection, SchedulerDispatchSelectionDiagnostic,
    SchedulerDispatchSelectionDiagnosticCode, SchedulerDispatchSelectionDiagnosticSeverity,
    SchedulerTaskStateRecord, ValidatedSchedulerDispatchSelectionRequest,
    SCHEDULER_COMPLETION_MAX_CANDIDATES,
};
use pantograph_workflow_service::WorkflowSchedulerTask;
use std::sync::Arc;

/// Exact advisory snapshot supplied by the embedded owner, including live capacity.
/// `admitted_task_fingerprint` identifies this run/task/Ready version/descriptor.
/// Effective settings and bindings are associated below, not hashed as credentials. `materialized_inputs` are the exact typed inputs resolved by
/// the same mapper used for host execution; the producer must qualify their size
/// and all effective runtime configuration, rather than use coarse trace averages.
/// Unknown physical device, implementation, workload or loaded generation => None.
/// Stopped with no instance is explicitly observed unloaded state, not an invented
/// loaded generation. Old loaded-generation samples must not be rebound blindly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedCompletionTimingQuery {
    pub admitted_task_fingerprint: String,
    pub artifact_fingerprint: String,
    pub runtime_id: String,
    pub runtime_variant_id: Option<String>,
    pub backend_key: String,
    pub task_kind: String,
    pub runtime_trait_settings: Vec<pantograph_scheduler::SchedulerTraitSetting>,
    pub runtime_source_context:
        Option<pantograph_workflow_service::graph::WorkflowRuntimeSourceContext>,
    pub materialized_inputs: Arc<[pantograph_runtime_host_contracts::RuntimeHostExecutionInput]>,
    pub device_id: String,
    pub runtime_residency_key: String,
    pub resource_observation: RuntimeReservationAdmissionObservation,
}

/// One complete successful serialized service cost, or explicitly configured cost.
/// Preparation includes required load/warmup/owner delay; execution includes cleanup.
/// Required transfer must be known explicitly, including a justified zero.
/// Echoing a query is snapshot association, not authentication of history.
#[derive(Debug, Clone)]
pub struct EmbeddedCompletionTimingRecord {
    pub query: EmbeddedCompletionTimingQuery,
    pub observed_at_ms: u64,
    pub preparation: RuntimeServiceTimingValue,
    pub required_transfer: RuntimeServiceTimingValue,
    pub execution: RuntimeServiceTimingValue,
}

/// Trusted in-process owner dependency, NOT a client timing or calibration API.
/// Implementations must be bounded and nonblocking (no I/O/search/runtime calls),
/// and return None unless the immutable model, actual physical device, runtime
/// implementation/configuration/generation and exact admitted workload are known.
/// Existing coarse trace averages and unqualified service attempts are insufficient.
pub trait EmbeddedCompletionTimingSource: Send + Sync {
    fn timing_for(
        &self,
        query: &EmbeddedCompletionTimingQuery,
    ) -> Option<EmbeddedCompletionTimingRecord>;
}

#[derive(Clone)]
pub struct EmbeddedCompletionTimingOptIn {
    pub source: Arc<dyn EmbeddedCompletionTimingSource>,
    /// Host/owner lifetime, not a remote host routing identifier.
    pub owner_epoch: String,
    pub max_sample_age_ms: u64,
    /// Explicit authored controlled estimates; never labeled measured evidence.
    pub allow_configured_estimates: bool,
}

pub(crate) fn select_with_owner_timing(
    request: &ValidatedSchedulerDispatchSelectionRequest,
    queries: &[Option<EmbeddedCompletionTimingQuery>],
    opt_in: &EmbeddedCompletionTimingOptIn,
) -> SchedulerDispatchReservationSelection {
    let policy = SchedulerCompletionRankingPolicy {
        now_ms: crate::runtime_dispatch_candidate_provider::current_time_ms(),
        max_sample_age_ms: opt_in.max_sample_age_ms,
        minimum_samples: 1,
        allow_synthetic: opt_in.allow_configured_estimates,
    };
    let input = request.as_ref();
    let bounded = input.candidates.len() <= SCHEDULER_COMPLETION_MAX_CANDIDATES
        && completion_diagnostics_bounded(&input.diagnostics)
        && input
            .candidates
            .iter()
            .all(|c| c.selected_device_ids.len() <= 8)
        && queries.len() == input.candidates.len()
        && bounded_text(&opt_in.owner_epoch)
        && opt_in.max_sample_age_ms > 0;
    // One immutable admitted workload for every alternative. Validate its bounded
    // serialization once, before callbacks; shared payloads avoid N large copies.
    let bounded = bounded && {
        let workload = queries
            .iter()
            .flatten()
            .next()
            .map(|q| &q.materialized_inputs);
        workload.is_none_or(|workload| {
            bounded_inputs(workload)
                && queries
                    .iter()
                    .flatten()
                    .all(|q| Arc::ptr_eq(workload, &q.materialized_inputs))
        })
    };
    if !bounded {
        return diagnosed_selection(select_scheduler_candidate_with_completion(
            request,
            &[],
            policy,
        ));
    }
    // Do not invoke owner code outside the policy's cohort bound.
    let records: Vec<_> = if queries.len() <= SCHEDULER_COMPLETION_MAX_CANDIDATES {
        queries
            .iter()
            .map(|q| {
                q.as_ref().filter(|q| query_bounded(q)).and_then(|q| {
                    opt_in
                        .source
                        .timing_for(q)
                        .filter(|record| record.query == *q)
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    // Rows all belong to this immutable request and echo their full current
    // observations. This same-decision label is never a historical profile key.
    let cohort = "same-evaluated-dispatch-snapshot";
    let rows: Vec<_> = request
        .as_ref()
        .candidates
        .iter()
        .zip(&records)
        .filter_map(|(candidate, record)| {
            let record = record.as_ref()?;
            let query = &record.query;
            let (preparation, source) = duration(&record.preparation)?;
            let (transfer, transfer_source) = duration(&record.required_transfer)?;
            let (execution, execution_source) = duration(&record.execution)?;
            if source != transfer_source || source != execution_source {
                return None;
            }
            let context = SchedulerCompletionContext {
                host_id: &opt_in.owner_epoch,
                // For unloaded candidates this label only denotes observed unloaded
                // state. It never fabricates a loaded runtime generation.
                runtime_instance_id: query
                    .resource_observation
                    .runtime_instance_id
                    .as_deref()
                    .unwrap_or("observed-unloaded"),
                artifact_fingerprint: &query.artifact_fingerprint,
                workload_fingerprint: &query.admitted_task_fingerprint,
                resource_condition_fingerprint: cohort,
                residency_fingerprint: &query.runtime_residency_key,
                timing_convention: "owner-serialized-service-us-v1",
            };
            Some(SchedulerCompletionEvidence {
                request,
                candidate,
                current_context: context,
                sample: SchedulerCompletionSample {
                    candidate,
                    context,
                    source,
                    sample_count: 1,
                    observed_at_ms: record.observed_at_ms,
                    preparation_us: Some(preparation),
                    required_transfer_us: Some(transfer),
                    execution_us: Some(execution),
                },
            })
        })
        .collect();
    let result = select_scheduler_candidate_with_completion(
        request,
        &rows,
        SchedulerCompletionRankingPolicy {
            now_ms: crate::runtime_dispatch_candidate_provider::current_time_ms(),
            ..policy
        },
    );
    diagnosed_selection(result)
}

fn diagnosed_selection(
    result: pantograph_scheduler::SchedulerCompletionRankingResult,
) -> SchedulerDispatchReservationSelection {
    let message = format!(
        "Owner completion timing: {:?}; Synthetic means owner-configured estimate.",
        result.diagnostic
    );
    let mut selection = result.selection;
    let selected = matches!(
        selection,
        SchedulerDispatchReservationSelection::Selected { .. }
    );
    let diagnostics = match &mut selection {
        SchedulerDispatchReservationSelection::Selected { diagnostics, .. }
        | SchedulerDispatchReservationSelection::NoSelection { diagnostics } => diagnostics,
    };
    diagnostics.push(SchedulerDispatchSelectionDiagnostic {
        severity: SchedulerDispatchSelectionDiagnosticSeverity::Info,
        code: if selected {
            SchedulerDispatchSelectionDiagnosticCode::CandidateSelected
        } else {
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
        },
        candidate_id: None,
        message,
        hint: Some("embedded_runtime_dispatch.completion_timing".into()),
    });
    selection
}

fn duration(value: &RuntimeServiceTimingValue) -> Option<(u64, SchedulerCompletionEvidenceSource)> {
    let (ns, source) = match value {
        RuntimeServiceTimingValue::Observed {
            elapsed_ns,
            outcome: RuntimeServiceTimingOutcome::Completed,
        } => (*elapsed_ns, SchedulerCompletionEvidenceSource::Measured),
        RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns } => {
            (*elapsed_ns, SchedulerCompletionEvidenceSource::Synthetic)
        }
        _ => return None,
    };
    // Round upward, never make a positive sub-microsecond measurement free.
    Some((ns / 1000 + u64::from(ns % 1000 != 0), source))
}

pub(crate) fn admitted_task_fingerprint(
    task: &WorkflowSchedulerTask,
    ready: &SchedulerTaskStateRecord,
) -> Option<String> {
    // Admitted metadata is already contract validated; hash only the sealed
    // identity/version, not unbounded diagnostic collections or graph templates.
    let bytes = serde_json::to_vec(&(
        &task.workflow_id,
        &task.workflow_run_id,
        &task.task_id,
        &task.node_id,
        ready.state_version,
        &task.inference_descriptor_fingerprint,
    ))
    .ok()?;
    Some(blake3::hash(&bytes).to_hex().to_string())
}

fn bounded_text(value: &str) -> bool {
    value.len() <= 128 && !value.trim().is_empty() && !value.chars().any(char::is_control)
}
fn query_bounded(q: &EmbeddedCompletionTimingQuery) -> bool {
    [
        &q.admitted_task_fingerprint,
        &q.artifact_fingerprint,
        &q.runtime_id,
        &q.backend_key,
        &q.device_id,
        &q.runtime_residency_key,
        &q.resource_observation.runtime_id,
    ]
    .into_iter()
    .all(|s| bounded_text(s))
        && bounded_text(&q.task_kind)
        && q.runtime_trait_settings.len() <= 32
        && q.runtime_trait_settings.iter().all(|t| match &t.value {
            pantograph_scheduler::SchedulerTraitValue::String(s) => s.len() <= 1024,
            _ => true,
        })
        && q.runtime_source_context.as_ref().is_none_or(|c| {
            [
                &c.operation_type,
                &c.context_shape_key,
                &c.cancellation_mode,
            ]
            .into_iter()
            .all(|s| bounded_text(s))
        })
        && q.runtime_variant_id.as_deref().is_none_or(bounded_text)
        && q.resource_observation.resources.len() <= 8
        && q.resource_observation.resource_domains.len() <= 8
        && q.resource_observation
            .resource_domains
            .iter()
            .all(|d| bounded_text(&d.domain_id))
        && q.resource_observation
            .runtime_instance_id
            .as_deref()
            .is_none_or(bounded_text)
}

// Stop serialization at 64 KiB, before cloning inputs or invoking owner code.
pub(crate) fn bounded_inputs(
    inputs: &[pantograph_runtime_host_contracts::RuntimeHostExecutionInput],
) -> bool {
    if inputs.len() > 16 {
        return false;
    }
    let mut remaining = 64 * 1024usize;
    for input in inputs {
        use pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue as Value;
        if input.port_id.len() > 128 {
            return false;
        }
        let bytes = match &input.value {
            Value::String(s) => s.len(),
            Value::MediaArtifactRef(r) => match r
                .artifact_id
                .len()
                .checked_add(r.media_type.as_ref().map_or(0, String::len))
            {
                Some(n) => n,
                None => return false,
            },
            _ => 0,
        };
        let Some(left) = remaining.checked_sub(bytes) else {
            return false;
        };
        remaining = left;
    }
    struct Limit(usize);
    impl std::io::Write for Limit {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.0 {
                return Err(std::io::Error::other("completion workload limit"));
            }
            self.0 -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Limit(64 * 1024), inputs).is_ok()
}
