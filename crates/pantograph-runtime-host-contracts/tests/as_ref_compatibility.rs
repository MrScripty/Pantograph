//! Compile coverage of public accessor call forms after migration to AsRef.
use pantograph_runtime_host_contracts::*;

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
    validated_reservation_lifecycle_event,
    ValidatedReservationLifecycleEvent,
    ReservationLifecycleEvent
);

accessor_call_forms!(
    validated_reservation_lifecycle_application,
    ValidatedReservationLifecycleApplication,
    ReservationLifecycleApplication
);

accessor_call_forms!(
    validated_runtime_host_execution_request,
    ValidatedRuntimeHostExecutionRequest,
    RuntimeHostExecutionRequest
);

accessor_call_forms!(
    validated_runtime_host_execution_response,
    ValidatedRuntimeHostExecutionResponse,
    RuntimeHostExecutionResponse
);

accessor_call_forms!(
    validated_runtime_host_batch_execution_request,
    ValidatedRuntimeHostBatchExecutionRequest,
    RuntimeHostBatchExecutionRequest
);

accessor_call_forms!(
    validated_runtime_host_batch_execution_response,
    ValidatedRuntimeHostBatchExecutionResponse,
    RuntimeHostBatchExecutionResponse
);

accessor_call_forms!(
    validated_workflow_session_runtime_load_proof,
    ValidatedWorkflowSessionRuntimeLoadProof,
    WorkflowSessionRuntimeLoadProof
);
