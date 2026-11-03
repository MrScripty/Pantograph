use super::*;
use crate::workflow::{
    WorkflowRuntimeDispatchCandidateProvider, WorkflowRuntimeDispatchCandidateProviderError,
    WorkflowRuntimeDispatchCandidateSet, WorkflowSchedulerTask, WorkflowSerialReadyConfig,
};
use async_trait::async_trait;
use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
use pantograph_runtime_host_contracts::*;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerSerialBoundDispatch, SchedulerSerialDrainedDispatch,
    SchedulerSerialOwnerSnapshot, SchedulerTaskStateRecord,
};
use std::sync::Arc;

struct UnqueriedPort;

#[async_trait]
impl RuntimeHostBatchExecutionPort for UnqueriedPort {
    async fn execute_runtime_host_batch_request(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        panic!("configuration must not execute a batch")
    }
}

#[async_trait]
impl ReservationLifecyclePort for UnqueriedPort {
    async fn apply_reservation_lifecycle(
        &self,
        _: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        panic!("configuration must not apply reservation lifecycle")
    }
}

#[async_trait]
impl SerialRuntimeHostBatchExecutionPort for UnqueriedPort {
    fn resident_serial_cpu_owner(
        &self,
        _: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
        panic!("configuration must not query a loaded CPU owner")
    }

    async fn prepare_serial_cpu(
        &self,
        _: SerialRuntimeHostPreparationRequest,
    ) -> Result<SerialRuntimeHostCpuOwnerEvidence, RuntimeHostExecutionPortError> {
        panic!("configuration must not prepare native work")
    }

    async fn execute_serial_singleton(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
        _: SchedulerSerialBoundDispatch,
    ) -> Result<
        (
            RuntimeHostBatchExecutionResponse,
            SchedulerSerialDrainedDispatch,
            Option<SerialRuntimeHostCpuOwnerEvidence>,
        ),
        RuntimeHostExecutionPortError,
    > {
        panic!("configuration must not execute native work")
    }

    async fn apply_serial_cleanup(
        &self,
        _: ReservationLifecycleEvent,
        _: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        panic!("configuration must not clean up native work")
    }
}

struct UnqueriedCandidates;

#[test]
fn fallible_grouping_observation_refuses_absent_mode_and_poisoned_cache() {
    assert!(matches!(
        WorkflowService::new().try_serial_grouping_work_for_test(),
        Err(crate::workflow::WorkflowServiceError::InvalidRequest(_))
    ));
    let service = WorkflowService::new_serial_ready_cpu(
        Arc::new(UnqueriedPort),
        WorkflowSerialReadyConfig::default()
            .with_fair_service_opportunities()
            .with_bounded_grouping()
            .unwrap(),
    );
    assert_eq!(
        service.try_serial_grouping_work_for_test().unwrap(),
        (0, 0, 0)
    );
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = service
            .serial_ready_mode
            .as_ref()
            .unwrap()
            .grouping
            .lock()
            .unwrap();
        panic!("controlled test-support cache poison");
    }));
    assert!(poisoned.is_err());
    assert!(matches!(
        service.try_serial_grouping_work_for_test(),
        Err(crate::workflow::WorkflowServiceError::Internal(_))
    ));
}

impl WorkflowRuntimeDispatchCandidateProvider for UnqueriedCandidates {
    fn runtime_dispatch_candidates(
        &self,
        _: &WorkflowSchedulerTask,
        _: &SchedulerTaskStateRecord,
        _: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        panic!("configuration must not query dispatch candidates")
    }

    fn requires_materialized_inputs(&self) -> bool {
        panic!("configuration must not query provider capabilities")
    }

    fn requires_dependency_lookahead(&self) -> bool {
        panic!("configuration must not query provider capabilities")
    }
}

fn serial(config: WorkflowSerialReadyConfig) -> WorkflowService {
    WorkflowService::new_serial_ready_cpu(Arc::new(UnqueriedPort), config)
        .with_runtime_dispatch_candidate_provider(Arc::new(UnqueriedCandidates))
}

#[test]
fn ordinary_configuration_has_no_serial_policy_or_sample_bounds() {
    let service = WorkflowService::new()
        .with_runtime_dispatch_candidate_provider(Arc::new(UnqueriedCandidates));
    let snapshot: crate::WorkflowSchedulerConfiguration = service.scheduler_configuration();
    assert_eq!(
        snapshot.execution_owner(),
        crate::WorkflowSchedulerExecutionOwner::Ordinary
    );
    assert!(!snapshot.serial_completion_advisory());
    assert!(!snapshot.serial_fair_opportunities());
    assert!(!snapshot.serial_residency());
    assert!(!snapshot.serial_key_reuse());
    assert_eq!(snapshot.serial_minimum_samples(), None);
    assert_eq!(snapshot.serial_maximum_sample_age(), None);
}

#[test]
fn serial_configuration_reports_each_admitted_option_combination_without_runtime_queries() {
    let mut combinations = 0;
    for advisory in [false, true] {
        for fair in [false, true] {
            for residency in [false, true] {
                for reuse in [false, true] {
                    if !fair && (residency || reuse) {
                        continue;
                    }
                    let mut config = WorkflowSerialReadyConfig::default();
                    if advisory {
                        config = config.with_completion_lookahead();
                    }
                    if fair {
                        config = config.with_fair_service_opportunities();
                    }
                    if residency {
                        config = config.with_serial_residency().unwrap();
                    }
                    if reuse {
                        config = config.with_bounded_grouping().unwrap();
                    }
                    let service = serial(config);
                    let snapshot = service.scheduler_configuration();
                    assert_eq!(
                        snapshot.execution_owner(),
                        WorkflowSchedulerExecutionOwner::SerialReadyCpu
                    );
                    assert_eq!(snapshot.serial_completion_advisory(), advisory);
                    assert_eq!(snapshot.serial_fair_opportunities(), fair);
                    assert_eq!(snapshot.serial_residency(), residency);
                    assert_eq!(snapshot.serial_key_reuse(), reuse);
                    assert_eq!(snapshot.serial_minimum_samples(), Some(3));
                    assert_eq!(
                        snapshot.serial_maximum_sample_age(),
                        Some(Duration::from_secs(60))
                    );
                    combinations += 1;
                }
            }
        }
    }
    assert_eq!(combinations, 10);
}

#[test]
fn configured_duration_keeps_nanosecond_precision_and_inclusive_limits() {
    for samples in [3, 8] {
        for age in [
            Duration::from_nanos(1),
            Duration::new(1, 123),
            Duration::from_secs(3600),
        ] {
            let service = serial(WorkflowSerialReadyConfig::new(samples, age).unwrap());
            let snapshot = service.scheduler_configuration();
            assert_eq!(snapshot.serial_minimum_samples(), Some(samples));
            assert_eq!(snapshot.serial_maximum_sample_age(), Some(age));
        }
    }
    assert!(WorkflowSerialReadyConfig::new(2, Duration::from_secs(1)).is_err());
    assert!(WorkflowSerialReadyConfig::new(9, Duration::from_secs(1)).is_err());
    assert!(WorkflowSerialReadyConfig::new(3, Duration::ZERO).is_err());
    assert!(
        WorkflowSerialReadyConfig::new(3, Duration::from_secs(3600) + Duration::from_nanos(1))
            .is_err()
    );
}

#[test]
fn service_clones_and_capacity_changes_preserve_constructor_configuration() {
    let service = serial(
        WorkflowSerialReadyConfig::default()
            .with_fair_service_opportunities()
            .with_bounded_grouping()
            .unwrap(),
    );
    let snapshot = service.scheduler_configuration();
    let cloned = service.clone();
    cloned.set_loaded_runtime_capacity_limit(Some(1)).unwrap();
    assert_eq!(cloned.scheduler_configuration(), snapshot);
    assert_eq!(service.scheduler_configuration(), snapshot);
    let copied = snapshot;
    drop(service);
    drop(cloned);
    assert_eq!(copied, snapshot);
    assert!(format!("{snapshot:?}").len() < 512);
}
