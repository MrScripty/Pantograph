//! Compile coverage of public accessor call forms after migration to AsRef.
use pantograph_scheduler::*;

macro_rules! accessor_call_forms {
    ($name:ident, $wrapper:ty, $raw:ty) => {
        #[test]
        fn $name() {
            let _: fn(&$wrapper) -> &$raw = |value| value.as_ref();
            let _: fn(&$wrapper) -> &$raw = |value| <$wrapper>::as_ref(value);
            let _: fn(&$wrapper) -> &$raw = <$wrapper>::as_ref;
            let _: fn(&$wrapper) -> &$raw = |value| <$wrapper as AsRef<$raw>>::as_ref(value);
        }
    };
}

accessor_call_forms!(
    validated_scheduler_batch_policy_decision,
    ValidatedSchedulerBatchPolicyDecision,
    SchedulerBatchPolicyDecision
);
accessor_call_forms!(
    validated_scheduler_capability_hint_snapshot,
    ValidatedSchedulerCapabilityHintSnapshot,
    SchedulerCapabilityHintSnapshot
);
accessor_call_forms!(
    validated_scheduler_dispatch_decision,
    ValidatedSchedulerDispatchDecision,
    SchedulerDispatchDecision
);
accessor_call_forms!(
    validated_scheduler_dispatch_selection_request,
    ValidatedSchedulerDispatchSelectionRequest,
    SchedulerDispatchSelectionRequest
);
accessor_call_forms!(
    validated_scheduler_dispatch_selection_decision,
    ValidatedSchedulerDispatchSelectionDecision,
    SchedulerDispatchSelectionDecision
);
accessor_call_forms!(
    validated_scheduler_runtime_handoff,
    ValidatedSchedulerRuntimeHandoff,
    SchedulerRuntimeHandoff
);
accessor_call_forms!(
    validated_schedulable_task_intent,
    ValidatedSchedulableTaskIntent,
    SchedulableTaskIntent
);
accessor_call_forms!(
    validated_scheduler_task_lifecycle_diagnostic_snapshot,
    ValidatedSchedulerTaskLifecycleDiagnosticSnapshot,
    SchedulerTaskLifecycleDiagnosticSnapshot
);
accessor_call_forms!(
    validated_scheduler_task_state_record,
    ValidatedSchedulerTaskStateRecord,
    SchedulerTaskStateRecord
);
accessor_call_forms!(
    validated_scheduler_task_state_transition,
    ValidatedSchedulerTaskStateTransition,
    SchedulerTaskStateTransition
);
accessor_call_forms!(
    validated_scheduler_readiness_admission_request,
    ValidatedSchedulerReadinessAdmissionRequest,
    SchedulerReadinessAdmissionRequest
);
accessor_call_forms!(
    validated_scheduler_readiness_admission_decision,
    ValidatedSchedulerReadinessAdmissionDecision,
    SchedulerReadinessAdmissionDecision
);
accessor_call_forms!(
    validated_scheduler_resource_residency_snapshot,
    ValidatedSchedulerResourceResidencySnapshot,
    SchedulerResourceResidencySnapshot
);
accessor_call_forms!(
    validated_scheduler_lifecycle_owner_snapshot,
    ValidatedSchedulerLifecycleOwnerSnapshot,
    SchedulerLifecycleOwnerSnapshot
);
