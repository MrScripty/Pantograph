use pantograph_dependency_planning::{
    DependencyBindingId, DependencyOverridePatchV1, DependencyPlanningCallerContext,
    DependencyPlanningContractError, DependencyPlanningPlatformContext, DependencyPlanningRequest,
    DependencyTaskId, DependencyTraitIntent, PumasModelRef, SchedulerIntent,
};

/// Preserve the graph validation producer's planning policy when scheduler
/// admission reconstructs the same requirements proof. Caller attribution is
/// retained separately from the facts hashed by the requirements producer.
pub(crate) fn inference_dependency_planning_request(
    model_ref: PumasModelRef,
    task_id: DependencyTaskId,
    scheduler_intent: SchedulerIntent,
    selected_binding_ids: Vec<DependencyBindingId>,
    dependency_override_patches: Vec<DependencyOverridePatchV1>,
    trait_intents: Vec<DependencyTraitIntent>,
    caller_context: DependencyPlanningCallerContext,
) -> Result<DependencyPlanningRequest, DependencyPlanningContractError> {
    let request = DependencyPlanningRequest {
        model_ref,
        task_id,
        task_type: None,
        expected_artifact_kind: None,
        scheduler_intent,
        platform_context: Some(DependencyPlanningPlatformContext::from_os_arch(
            std::env::consts::OS,
            std::env::consts::ARCH,
        )?),
        selected_binding_ids,
        dependency_override_patches,
        trait_intents,
        caller_context,
    };
    request.validate()?;
    Ok(request)
}
