//! Canonical local-host planning scope shared by graph Resolve and admission.
//!
//! Workflow-service owns both seams. Caller attribution may differ between
//! draft validation and a submitted run; requirements scope must not differ.

use pantograph_dependency_planning::{
    DependencyBindingId, DependencyOverridePatchV1, DependencyPlanningCallerContext,
    DependencyPlanningContractError, DependencyPlanningPlatformContext, DependencyPlanningRequest,
    DependencyTaskId, DependencyTraitIntent, PumasModelRef, SchedulerIntent,
};

pub(crate) struct InferenceDependencyPlanningInput {
    pub model_ref: PumasModelRef,
    pub task_type: DependencyTaskId,
    pub scheduler_intent: SchedulerIntent,
    pub selected_binding_ids: Vec<DependencyBindingId>,
    pub dependency_override_patches: Vec<DependencyOverridePatchV1>,
    pub trait_intents: Vec<DependencyTraitIntent>,
    pub caller_context: DependencyPlanningCallerContext,
}

pub(crate) fn inference_dependency_planning_request(
    input: InferenceDependencyPlanningInput,
) -> Result<DependencyPlanningRequest, DependencyPlanningContractError> {
    Ok(DependencyPlanningRequest {
        model_ref: input.model_ref,
        task_id: input.task_type.clone(),
        task_type: Some(input.task_type),
        expected_artifact_kind: None,
        scheduler_intent: input.scheduler_intent,
        platform_context: Some(DependencyPlanningPlatformContext::from_os_arch(
            std::env::consts::OS,
            std::env::consts::ARCH,
        )?),
        selected_binding_ids: input.selected_binding_ids,
        dependency_override_patches: input.dependency_override_patches,
        trait_intents: input.trait_intents,
        caller_context: input.caller_context,
    })
}
