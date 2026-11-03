//! Session-owned advisory facts. Neither a successor Ready proof nor admission.
use super::{WorkflowSchedulerTask, WorkflowSchedulerTaskInputBinding};
use pantograph_dependency_planning::{DependencyEnvironmentRef, DependencyReadinessProofEnvelope};
use pantograph_runtime_host_contracts::RuntimeHostExecutionInput;
use pantograph_scheduler::{SchedulerTaskStateRecord, SchedulerTraitValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowCompletionSuccessorSnapshot {
    pub predecessor_task_id: String,
    pub predecessor_state_version: u64,
    pub task: WorkflowSchedulerTask,
    pub record: SchedulerTaskStateRecord,
    pub known_inputs: Vec<RuntimeHostExecutionInput>,
    /// Exact missing predecessor-output obligations; their values are NOT known.
    pub predecessor_inputs: Vec<WorkflowSchedulerTaskInputBinding>,
    /// Present environment evidence shared with first's actual Ready proof.
    pub shared_environment: DependencyEnvironmentRef,
}

pub(crate) fn bounded_task(t: &WorkflowSchedulerTask) -> bool {
    let Some(intent) = &t.schedulable_intent else {
        return false;
    };
    let Some(template) = &t.schedulable_intent_template else {
        return false;
    };
    let model = &intent.model_ref;
    t.node_type.len() <= 128
        && t.dependency_task_ids.len() <= 16
        && t.input_bindings.len() <= 16
        && t.input_bindings
            .iter()
            .all(|b| b.source_port_id.len() <= 128 && b.target_port_id.len() <= 128)
        && t.diagnostics.is_empty()
        && t.non_runtime_task_template.is_none()
        && t.source_input_task_template.is_none()
        && t.runtime_source_context.as_ref().is_none_or(|c| {
            c.operation_type.len() <= 128
                && c.context_shape_key.len() <= 128
                && c.cancellation_mode.len() <= 128
        })
        && model.model_id.len() <= 128
        && model.revision.as_ref().is_none_or(|s| s.len() <= 128)
        && model
            .selected_artifact_id
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && model.selected_artifact_path.is_none()
        && model.migration_diagnostics.is_empty()
        && intent.trait_settings.len() <= 32
        && template.trait_settings.len() <= 32
        && intent
            .trait_settings
            .iter()
            .chain(&template.trait_settings)
            .all(|s| match &s.value {
                SchedulerTraitValue::String(s) => s.len() <= 1024,
                _ => true,
            })
        && intent.dependency_override_patches.is_empty()
        && template.dependency_override_patches.is_empty()
        && intent.estimate_hints.len() <= 32
        && template.estimate_hints.len() <= 32
        && template
            .dependency_readiness_source
            .selected_binding_ids
            .len()
            <= 32
        && bounded_serialized(t)
}

pub(crate) fn equivalent_environment(
    a: &WorkflowSchedulerTask,
    b: &WorkflowSchedulerTask,
    proof: &DependencyReadinessProofEnvelope,
) -> bool {
    if !bounded_task(a)
        || !bounded_task(b)
        || !bounded_proof(proof)
        || proof.execution_context.selected_binding_ids.len() > 32
        || proof
            .preflight_result
            .identity_key
            .selected_binding_ids
            .len()
            > 32
    {
        return false;
    }
    let first = a.schedulable_intent.as_ref().unwrap();
    let next = b.schedulable_intent.as_ref().unwrap();
    let source = &a
        .schedulable_intent_template
        .as_ref()
        .unwrap()
        .dependency_readiness_source;
    let ctx = &proof.execution_context;
    first.model_ref == next.model_ref
        && first.task_type == next.task_type
        && first.constraints == next.constraints
        && first.trait_settings == next.trait_settings
        && source
            == &b
                .schedulable_intent_template
                .as_ref()
                .unwrap()
                .dependency_readiness_source
        && ctx.graph_revision == source.graph_revision
        && ctx.validation_session_id == source.validation_session_id
        && ctx.validation_snapshot_id == source.validation_snapshot_id
        && ctx.descriptor_fingerprint == source.descriptor_fingerprint
        && ctx.dependency_requirements_id == source.dependency_requirements_id
        && ctx.selected_binding_ids == source.selected_binding_ids
        && ctx.dependency_override_fingerprint.as_ref()
            == Some(&source.dependency_override_fingerprint)
        && proof.preflight_result.dependency_requirements_id.as_ref()
            == Some(&source.dependency_requirements_id)
        && proof.preflight_result.identity_key.model_ref == first.model_ref
        && proof.preflight_result.identity_key.task_id == first.task_type
        && proof.preflight_result.identity_key.selected_binding_ids == source.selected_binding_ids
        && proof
            .preflight_result
            .identity_key
            .scheduler_intent
            .requested_runtime_id
            == first.constraints.requested_runtime_id
        && proof
            .preflight_result
            .identity_key
            .scheduler_intent
            .requested_device_id
            == first.constraints.requested_device_id
        && a.inference_descriptor_fingerprint == b.inference_descriptor_fingerprint
}

pub(crate) fn bounded_serialized<T: serde::Serialize>(value: &T) -> bool {
    struct Limit(usize);
    impl std::io::Write for Limit {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.0 {
                return Err(std::io::Error::other("completion snapshot bound"));
            }
            self.0 -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Limit(64 * 1024), value).is_ok()
}

pub(crate) fn bounded_proof(p: &DependencyReadinessProofEnvelope) -> bool {
    let model = &p.preflight_result.identity_key.model_ref;
    p.execution_context.selected_binding_ids.len() <= 32
        && p.preflight_result.identity_key.selected_binding_ids.len() <= 32
        && model.model_id.len() <= 128
        && model.revision.as_ref().is_none_or(|s| s.len() <= 128)
        && model
            .selected_artifact_id
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && model.selected_artifact_path.is_none()
        && model.migration_diagnostics.is_empty()
        && p.preflight_result.diagnostics.len() <= 32
        && p.preflight_result.diagnostics.iter().all(|d| {
            d.message.len() <= 1024
                && d.model_id.as_ref().is_none_or(|s| s.len() <= 128)
                && d.field_path.as_ref().is_none_or(|s| s.len() <= 1024)
        })
        && bounded_serialized(p)
}
