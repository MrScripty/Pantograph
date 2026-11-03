use super::*;
use crate::workflow::*;
use pantograph_scheduler::SchedulerTaskExecutionIntent;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Weak,
};

struct Fixture {
    store: WorkflowExecutionSessionStore,
    session: String,
    run: String,
    first: String,
}
impl Fixture {
    fn snapshot(&self) -> Result<WorkflowFrozenCohortSnapshot, WorkflowCohortRefusal> {
        self.store
            .frozen_cohort_snapshot(&self.session, &self.run, &self.first)
    }
    fn edit(
        &mut self,
        edit: impl FnOnce(&mut WorkflowSchedulerTaskGraph, &mut Vec<SchedulerTaskStateRecord>),
    ) {
        let (mut graph, mut records) = self
            .store
            .active_run_scheduler_task_state(&self.session, &self.run)
            .unwrap()
            .unwrap();
        edit(&mut graph, &mut records);
        self.store
            .set_active_run_scheduler_task_state(&self.session, &self.run, graph, records)
            .unwrap();
    }
}
fn fixture(count: usize, edges: &[(usize, usize)]) -> Fixture {
    let request = dispatch_selection_request_fixture();
    let mut proof = request.readiness_proof;
    proof.execution_context.dependency_override_fingerprint =
        Some(DependencyOverrideFingerprint::parse("override.fixture").unwrap());
    let mut intent = request.task_intent;
    intent.constraints = SchedulerRuntimeDeviceConstraints::default();
    let ctx = &proof.execution_context;
    let template = WorkflowSchedulerTaskIntentTemplate {
        task_type: intent.task_type.clone(),
        constraints: intent.constraints.clone(),
        trait_settings: vec![],
        dependency_override_patches: vec![],
        estimate_hints: vec![],
        dependency_readiness_source: WorkflowSchedulerDependencyReadinessSource {
            graph_revision: ctx.graph_revision.clone(),
            validation_session_id: ctx.validation_session_id.clone(),
            validation_snapshot_id: ctx.validation_snapshot_id.clone(),
            descriptor_fingerprint: ctx.descriptor_fingerprint.clone(),
            dependency_requirements_id: ctx.dependency_requirements_id.clone(),
            selected_binding_ids: ctx.selected_binding_ids.clone(),
            dependency_override_fingerprint: ctx.dependency_override_fingerprint.clone().unwrap(),
        },
    };
    let first = intent.task_id.to_string();
    let mut tasks = Vec::new();
    for i in 0..count {
        let mut intent = intent.clone();
        if i != 0 {
            intent.task_id = SchedulerTaskId::parse(format!("task.cohort.{i}")).unwrap();
            intent.node_id = SchedulerNodeId::parse(format!("node.cohort.{i}")).unwrap();
        }
        let mut task = task_from_intent(intent);
        task.schedulable_intent_template = Some(template.clone());
        tasks.push(task);
    }
    for &(source, target) in edges {
        let source_task = tasks[source].clone();
        tasks[target]
            .dependency_task_ids
            .push(source_task.task_id.clone());
        tasks[target]
            .input_bindings
            .push(WorkflowSchedulerTaskInputBinding {
                source_node_id: source_task.node_id,
                source_task_id: source_task.task_id,
                source_port_id: "text".into(),
                target_port_id: format!("input.{source}"),
            });
    }
    let graph = task_graph(tasks);
    let run = graph.workflow_run_id.to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session = begin_active_run_for_task_graph(&mut store, &graph);
    let orchestrator = orchestrator_without_runtime_host_response();
    orchestrator
        .initialize_active_run_task_state(&mut store, &session, &run, graph)
        .unwrap();
    orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session,
            &run,
            &first,
            DependencyReadinessPolicy::CheckOnly,
            Some(proof),
        )
        .unwrap();
    Fixture {
        store,
        session,
        run,
        first,
    }
}

#[test]
fn cohort_capture_complete_deterministic_independent_chain_fanout_and_join() {
    for count in 1..=4 {
        let chain: Vec<_> = (1..count).map(|i| (i - 1, i)).collect();
        let fanout: Vec<_> = (1..count).map(|i| (0, i)).collect();
        let join = if count == 4 {
            vec![(0, 1), (0, 2), (1, 3), (2, 3)]
        } else {
            chain.clone()
        };
        for edges in [vec![], chain, fanout, join] {
            let f = fixture(count, &edges);
            let before = format!("{:?}", f.store);
            let snapshot = f.snapshot().unwrap();
            assert_eq!(snapshot.members().len(), count);
            assert_eq!(snapshot.first_task_id().as_str(), f.first);
            assert_eq!(snapshot, f.snapshot().unwrap());
            assert_eq!(before, format!("{:?}", f.store));
            assert_eq!(
                snapshot
                    .members()
                    .iter()
                    .map(|m| m.symbolic_inputs.len())
                    .sum::<usize>(),
                edges.len()
            );
            assert!(snapshot
                .members()
                .windows(2)
                .all(|w| w[0].task.task_id < w[1].task.task_id));
            assert!(
                serde_json::to_vec(&snapshot).unwrap().len() <= WORKFLOW_COHORT_MAX_SNAPSHOT_BYTES
            );
        }
    }
}
#[test]
fn cohort_capture_refuses_fifth_missing_foreign_running_and_external_prerequisite() {
    assert_eq!(
        fixture(5, &[]).snapshot(),
        Err(WorkflowCohortRefusal::PopulationLimit)
    );
    let mut f = fixture(3, &[]);
    f.edit(|_, records| {
        records.pop();
    });
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::UnknownRecord));
    let mut f = fixture(3, &[]);
    f.edit(|graph, _| {
        graph.tasks[1].workflow_id = SchedulerWorkflowId::parse("foreign.workflow").unwrap();
    });
    assert!(f.snapshot().is_err());
    let mut f = fixture(3, &[]);
    f.edit(|graph, records| {
        let task = &graph.tasks[1];
        records
            .iter_mut()
            .find(|r| r.task_id == task.task_id)
            .unwrap()
            .state = SchedulerTaskState::Running {
            execution_intent: SchedulerTaskExecutionIntent::runtime(
                task.schedulable_intent.clone().unwrap(),
            ),
        };
    });
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::UnsupportedState));
    let mut f = fixture(3, &[]);
    f.edit(|graph, records| {
        let mut external = text_input_task("task.external", "unused");
        external.workflow_id = graph.workflow_id.clone();
        external.workflow_run_id = graph.workflow_run_id.clone();
        let mut record = records[0].clone();
        record.task_id = external.task_id.clone();
        record.node_id = external.node_id.clone();
        record.state = SchedulerTaskState::AwaitingInputs {
            diagnostics: vec![],
        };
        graph.tasks[1]
            .dependency_task_ids
            .push(external.task_id.clone());
        graph.tasks.push(external);
        records.push(record);
    });
    assert_eq!(
        f.snapshot(),
        Err(WorkflowCohortRefusal::ExternalUnfinishedPrerequisite)
    );
}
#[test]
fn cohort_capture_rejects_cycles_and_duplicate_edges() {
    for cycle in [false, true] {
        let mut f = fixture(3, &[(0, 1), (1, 2)]);
        f.edit(|graph, _| {
            let dependency = graph.tasks[if cycle { 2 } else { 0 }].task_id.clone();
            graph.tasks[1].dependency_task_ids.push(dependency);
        });
        assert_eq!(
            f.snapshot(),
            Err(WorkflowCohortRefusal::InvalidDependencies)
        );
    }
}
#[test]
fn cohort_capture_invalidates_every_member_descriptor_version_dependency_and_binding() {
    for index in 0..4 {
        for field in 0..4 {
            let mut f = fixture(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
            let snapshot = f.snapshot().unwrap();
            f.edit(|graph, records| {
                let task = &mut graph.tasks[index];
                match field {
                    0 => {
                        records
                            .iter_mut()
                            .find(|r| r.task_id == task.task_id)
                            .unwrap()
                            .state_version += 1
                    }
                    1 => task.node_type.push_str(".changed"),
                    2 => {
                        task.schedulable_intent_template
                            .as_mut()
                            .unwrap()
                            .dependency_readiness_source
                            .descriptor_fingerprint =
                            DependencyReadinessDescriptorFingerprint::parse("descriptor.changed")
                                .unwrap()
                    }
                    _ => {
                        if let Some(binding) = task.input_bindings.first_mut() {
                            binding.source_port_id.push_str(".changed");
                        } else {
                            task.dependency_task_ids
                                .push(SchedulerTaskId::parse("unknown.task").unwrap());
                        }
                    }
                }
            });
            assert!(
                f.store.validate_frozen_cohort_snapshot(&snapshot).is_err(),
                "member {index} field {field}"
            );
        }
    }
}
fn add_known_input(
    f: &mut Fixture,
    value: WorkflowSchedulerTaskResultValue,
) -> WorkflowSchedulerTaskResult {
    let mut result = None;
    f.edit(|graph, records| {
        let mut source = graph.tasks[0].clone();
        source.task_id = SchedulerTaskId::parse("task.source").unwrap();
        source.node_id = SchedulerNodeId::parse("node.source").unwrap();
        let intent = source.schedulable_intent.as_mut().unwrap();
        intent.task_id = source.task_id.clone();
        intent.node_id = source.node_id.clone();
        let mut record = records
            .iter()
            .find(|r| r.task_id == graph.tasks[0].task_id)
            .unwrap()
            .clone();
        record.task_id = source.task_id.clone();
        record.node_id = source.node_id.clone();
        record.state = SchedulerTaskState::Completed {
            execution_intent: SchedulerTaskExecutionIntent::runtime(intent.clone()),
        };
        let mut output = runtime_task_result_fixture(&source);
        output.outputs = vec![WorkflowSchedulerTaskResultOutput {
            port_id: "text".into(),
            value: value.clone(),
        }];
        result = Some(output);
        for task in &mut graph.tasks {
            task.dependency_task_ids.push(source.task_id.clone());
            task.input_bindings.push(WorkflowSchedulerTaskInputBinding {
                source_task_id: source.task_id.clone(),
                source_node_id: source.node_id.clone(),
                source_port_id: "text".into(),
                target_port_id: if matches!(
                    value,
                    WorkflowSchedulerTaskResultValue::PumasModelRef(_)
                ) {
                    "pumas_model_ref".into()
                } else {
                    "known.text".into()
                },
            });
        }
        graph.tasks.push(source);
        records.push(record);
    });
    let result = result.unwrap();
    f.store
        .set_active_run_scheduler_task_results(&f.session, &f.run, vec![result.clone()])
        .unwrap();
    result
}
#[test]
fn cohort_capture_known_inputs_include_values_and_ignored_model_binding_identities() {
    for model in [false, true] {
        let mut f = fixture(4, &[(0, 1), (1, 2), (2, 3)]);
        let value = if model {
            WorkflowSchedulerTaskResultValue::PumasModelRef(
                dispatch_selection_request_fixture().task_intent.model_ref,
            )
        } else {
            WorkflowSchedulerTaskResultValue::String("known text".into())
        };
        let mut result = add_known_input(&mut f, value);
        let snapshot = f.snapshot().unwrap();
        for member in snapshot.members() {
            assert_eq!(member.known_input_identities.len(), 1);
            assert_eq!(member.known_inputs.len(), usize::from(!model));
        }
        if let WorkflowSchedulerTaskResultValue::PumasModelRef(model) = &mut result.outputs[0].value
        {
            model.model_id.push_str(".changed");
        } else {
            result.outputs[0].value =
                WorkflowSchedulerTaskResultValue::String("changed text".into());
        }
        f.store
            .set_active_run_scheduler_task_results(&f.session, &f.run, vec![result])
            .unwrap();
        assert!(f.store.validate_frozen_cohort_snapshot(&snapshot).is_err());
    }
}
#[test]
fn cohort_capture_bounds_graph_records_inputs_and_total_bytes() {
    let mut f = fixture(4, &[]);
    f.edit(|graph, records| {
        for task in graph.tasks.iter_mut().skip(1) {
            let settings: Vec<_> = (0..20)
                .map(|i| pantograph_scheduler::SchedulerTraitSetting {
                    trait_id: format!("setting.{i}").parse().unwrap(),
                    value: pantograph_scheduler::SchedulerTraitValue::String("x".repeat(1024)),
                })
                .collect();
            task.schedulable_intent.as_mut().unwrap().trait_settings = settings.clone();
            task.schedulable_intent_template
                .as_mut()
                .unwrap()
                .trait_settings = settings;
            records
                .iter_mut()
                .find(|r| r.task_id == task.task_id)
                .unwrap()
                .state = SchedulerTaskState::WaitingDependencyReadiness {
                execution_intent: SchedulerTaskExecutionIntent::runtime(
                    task.schedulable_intent.clone().unwrap(),
                ),
            };
        }
    });
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::SnapshotByteLimit));
    let mut f = fixture(1, &[]);
    let mut result = add_known_input(
        &mut f,
        WorkflowSchedulerTaskResultValue::String("known".into()),
    );
    result
        .outputs
        .extend((0..32).map(|i| WorkflowSchedulerTaskResultOutput {
            port_id: format!("other.{i}"),
            value: WorkflowSchedulerTaskResultValue::Bool(true),
        }));
    f.store
        .set_active_run_scheduler_task_results(&f.session, &f.run, vec![result])
        .unwrap();
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::UnknownInput));
    let mut f = fixture(1, &[]);
    add_known_input(
        &mut f,
        WorkflowSchedulerTaskResultValue::Json(serde_json::json!([1, 2, 3])),
    );
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::UnknownInput));
    let mut f = fixture(2, &[]);
    f.edit(|graph, _| {
        graph.tasks[1].node_type = "x".repeat(129);
    });
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::UnsupportedTask));
    let mut f = fixture(2, &[(0, 1)]);
    f.edit(|graph, records| {
        records
            .iter_mut()
            .find(|r| r.task_id == graph.tasks[1].task_id)
            .unwrap()
            .state = SchedulerTaskState::AwaitingInputs {
            diagnostics: vec![
                pantograph_scheduler::SchedulerTaskStateDiagnostic {
                    severity: SchedulerTaskStateDiagnosticSeverity::Info,
                    code: SchedulerTaskStateDiagnosticCode::AwaitingInputs,
                    message: "valid diagnostic".into(),
                    hint: None,
                };
                33
            ],
        };
    });
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::UnsupportedState));
}

struct Provider {
    service: Mutex<Weak<WorkflowService>>,
    epoch: Arc<AtomicU64>,
    calls: AtomicUsize,
    mode: usize,
}
impl WorkflowCohortCoverageProvider for Provider {
    fn supports_cohort_coverage(&self) -> bool {
        if let Some(service) = self.service.lock().unwrap().upgrade() {
            assert!(service.cohort_store_is_unlocked_for_test());
        }
        true
    }
    fn cohort_placements(
        &self,
        snapshot: &WorkflowFrozenCohortSnapshot,
    ) -> Result<Vec<WorkflowCohortTaskPlacements>, WorkflowCohortRefusal> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let service = self.service.lock().unwrap().upgrade().unwrap();
        assert!(
            service.cohort_store_is_unlocked_for_test(),
            "provider callback under store lock"
        );
        let mut groups: Vec<_> = snapshot
            .members()
            .iter()
            .map(|m| WorkflowCohortTaskPlacements {
                task_id: m.task.task_id.clone(),
                complete: true,
                placements: (0..2)
                    .map(|i| WorkflowCohortPlacement {
                        descriptor: WorkflowCohortPlacementDescriptor {
                            placement_id: format!("placement.{i}"),
                            runtime_id: format!("runtime.{i}"),
                            runtime_variant_id: None,
                            device_id: "cpu".into(),
                            artifact_fingerprint: "artifact.immutable".into(),
                            runtime_instance_id: None,
                        },
                        owner_epoch: WorkflowCohortOwnerEpoch::capture(
                            "owner.test".into(),
                            self.epoch.clone(),
                        )
                        .unwrap(),
                    })
                    .collect(),
            })
            .collect();
        match self.mode {
            1 => {
                let p = groups[0].placements[0].clone();
                groups[0].placements.push(p);
            }
            2 => self.epoch.store(2, Ordering::Release),
            3 => {
                let mut store = service.session_store_guard().unwrap();
                let (mut graph, records) = store
                    .active_run_scheduler_task_state(
                        &snapshot.session_id,
                        &snapshot.workflow_run_id,
                    )
                    .unwrap()
                    .unwrap();
                graph
                    .tasks
                    .last_mut()
                    .unwrap()
                    .node_type
                    .push_str(".changed");
                store
                    .set_active_run_scheduler_task_state(
                        &snapshot.session_id,
                        &snapshot.workflow_run_id,
                        graph,
                        records,
                    )
                    .unwrap();
            }
            4 => groups[0].complete = false,
            5 => groups[0].placements[0].descriptor.runtime_id = String::new(),
            6 => {
                groups.pop();
            }
            7 => {
                let generation = Arc::new(AtomicU64::new(1));
                groups
                    .last_mut()
                    .unwrap()
                    .placements
                    .last_mut()
                    .unwrap()
                    .owner_epoch =
                    WorkflowCohortOwnerEpoch::capture("last.owner".into(), generation.clone())
                        .unwrap();
                generation.store(2, Ordering::Release);
            }
            8 => groups[0].placements[1] = groups[0].placements[0].clone(),
            9 => groups[0].task_id = groups[1].task_id.clone(),
            10 => groups[0].task_id = "task.foreign".parse().unwrap(),
            11 => {
                let mut store = service.session_store_guard().unwrap();
                let mut results = store
                    .active_run_scheduler_task_results(
                        &snapshot.session_id,
                        &snapshot.workflow_run_id,
                    )
                    .unwrap();
                results[0].outputs[0].value =
                    WorkflowSchedulerTaskResultValue::String("changed input".into());
                store
                    .set_active_run_scheduler_task_results(
                        &snapshot.session_id,
                        &snapshot.workflow_run_id,
                        results,
                    )
                    .unwrap();
            }
            12 => {
                let mut proof = snapshot.first_readiness_proof().clone();
                proof.readiness_proof_version =
                    pantograph_dependency_planning::DependencyReadinessProofVersion::parse(
                        proof.readiness_proof_version.get() + 1,
                    )
                    .unwrap();
                service
                    .session_store_guard()
                    .unwrap()
                    .record_active_run_runtime_dispatch_readiness_proof(
                        &snapshot.session_id,
                        &snapshot.workflow_run_id,
                        snapshot.first_task_id().as_str(),
                        proof,
                    )
                    .unwrap();
            }
            13 => {
                let mut store = service.session_store_guard().unwrap();
                let (mut graph, mut records) = store
                    .active_run_scheduler_task_state(
                        &snapshot.session_id,
                        &snapshot.workflow_run_id,
                    )
                    .unwrap()
                    .unwrap();
                let removed = graph.tasks.remove(1).task_id;
                records.retain(|r| r.task_id != removed);
                store
                    .set_active_run_scheduler_task_state(
                        &snapshot.session_id,
                        &snapshot.workflow_run_id,
                        graph,
                        records,
                    )
                    .unwrap();
            }
            _ => {}
        }
        groups.reverse();
        Ok(groups)
    }
}
fn service(
    f: Fixture,
    mode: usize,
) -> (Arc<WorkflowService>, Arc<Provider>, String, String, String) {
    let provider = Arc::new(Provider {
        service: Mutex::new(Weak::new()),
        epoch: Arc::new(AtomicU64::new(1)),
        calls: AtomicUsize::new(0),
        mode,
    });
    struct DispatchMustNotBeCalled;
    impl WorkflowRuntimeDispatchCandidateProvider for DispatchMustNotBeCalled {
        fn runtime_dispatch_candidates(
            &self,
            _: &WorkflowSchedulerTask,
            _: &SchedulerTaskStateRecord,
            _: &DependencyReadinessProofEnvelope,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            panic!("coverage must not enter dispatch collection or reserve resources")
        }
    }
    let service = Arc::new(
        WorkflowService::new_with_cohort_coverage(provider.clone())
            .with_runtime_dispatch_candidate_provider(Arc::new(DispatchMustNotBeCalled)),
    );
    *provider.service.lock().unwrap() = Arc::downgrade(&service);
    *service.session_store_guard().unwrap() = f.store;
    (service, provider, f.session, f.run, f.first)
}
#[test]
fn cohort_capture_query_is_opt_in_read_only_bounded_and_always_honest_about_missing_evidence() {
    let default = WorkflowService::new();
    assert!(!default.cohort_coverage_enabled());
    assert!(matches!(
        default
            .workflow_cohort_evidence_coverage("missing", "missing", "missing")
            .unwrap(),
        WorkflowCohortCoverageResult::Disabled
    ));
    struct Disabled;
    impl WorkflowCohortCoverageProvider for Disabled {}
    assert!(
        !WorkflowService::new_with_cohort_coverage(Arc::new(Disabled)).cohort_coverage_enabled()
    );
    let (service, provider, session, run, first) =
        service(fixture(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]), 0);
    let before = format!("{:?}", service.session_store_guard().unwrap());
    let result = service
        .workflow_cohort_evidence_coverage(&session, &run, &first)
        .unwrap();
    let WorkflowCohortCoverageResult::Incomplete(coverage) = result else {
        panic!("expected honest incomplete coverage")
    };
    assert_eq!(coverage.snapshot.members().len(), 4);
    assert_eq!(coverage.placements.len(), 8);
    assert!(!coverage.native_evidence_complete);
    assert!(coverage.placements.iter().all(|p| p
        .missing
        .contains(&WorkflowCohortMissingQualification::Workload)
        && p.missing
            .contains(&WorkflowCohortMissingQualification::TransferTiming)
        && p.missing
            .contains(&WorkflowCohortMissingQualification::ReleaseReconciliation)
        && p.missing
            .contains(&WorkflowCohortMissingQualification::ConditionalCapacity)));
    assert_eq!(
        before,
        format!("{:?}", service.session_store_guard().unwrap())
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert!(serde_json::to_vec(&coverage).unwrap().len() <= WORKFLOW_COHORT_MAX_SNAPSHOT_BYTES);
}
#[test]
fn cohort_capture_query_revalidates_owner_and_members_and_refuses_incomplete_placements() {
    for (mode, expected) in [
        (1, WorkflowCohortRefusal::PlacementLimit),
        (2, WorkflowCohortRefusal::OwnerEpochChanged),
        (3, WorkflowCohortRefusal::SnapshotChanged),
        (4, WorkflowCohortRefusal::IncompletePlacements),
        (5, WorkflowCohortRefusal::UnsupportedPlacement),
        (6, WorkflowCohortRefusal::IncompletePlacements),
        (7, WorkflowCohortRefusal::OwnerEpochChanged),
        (8, WorkflowCohortRefusal::UnsupportedPlacement),
        (9, WorkflowCohortRefusal::IncompletePlacements),
        (10, WorkflowCohortRefusal::IncompletePlacements),
    ] {
        let (service, _, session, run, first) = service(fixture(3, &[(0, 1), (1, 2)]), mode);
        assert!(
            matches!(service.workflow_cohort_evidence_coverage(&session,&run,&first).unwrap(), WorkflowCohortCoverageResult::Refused(reason) if reason == expected),
            "mode {mode}"
        );
    }
    let (service, provider, session, run, first) = service(fixture(5, &[]), 0);
    assert!(matches!(
        service
            .workflow_cohort_evidence_coverage(&session, &run, &first)
            .unwrap(),
        WorkflowCohortCoverageResult::Refused(WorkflowCohortRefusal::PopulationLimit)
    ));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn cohort_capture_preserves_pair_api_two_task_success_and_third_task_refusal() {
    for count in [2, 3] {
        let f = fixture(count, &[(0, 1)]);
        let snapshot = f.snapshot().unwrap();
        let first = snapshot
            .members()
            .iter()
            .find(|m| m.task.task_id == *snapshot.first_task_id())
            .unwrap();
        let pair = f.store.completion_successor_snapshot(
            &f.session,
            &f.run,
            &first.task,
            &first.record,
            snapshot.first_readiness_proof(),
        );
        assert_eq!(pair.is_some(), count == 2);
    }
}

#[test]
fn cohort_capture_first_admission_requires_valid_ready_exact_owner_proof() {
    for field in 0..7 {
        let mut f = fixture(3, &[]);
        let mut proof = f.snapshot().unwrap().first_readiness_proof().clone();
        match field {
            0 => proof.contract_version += 1,
            1 => proof.preflight_result.environment_ref = None,
            2 => {
                proof.preflight_result.readiness_state =
                    DependencyEnvironmentReadinessState::NotImplemented
            }
            3 => proof.execution_context.workflow_id = "workflow.foreign".parse().unwrap(),
            4 => proof.execution_context.workflow_run_id = "run.foreign".parse().unwrap(),
            5 => proof.execution_context.scheduler_task_id = "task.foreign".parse().unwrap(),
            _ => proof.execution_context.node_id = "node.foreign".parse().unwrap(),
        }
        f.store
            .record_active_run_runtime_dispatch_readiness_proof(&f.session, &f.run, &f.first, proof)
            .unwrap();
        assert_eq!(
            f.snapshot(),
            Err(WorkflowCohortRefusal::FirstNotAdmitted),
            "field {field}"
        );
    }
}
#[test]
fn cohort_capture_caps_result_population_before_lookup_and_streams_byte_limits() {
    let mut f = fixture(1, &[]);
    let source = f.snapshot().unwrap().members()[0].task.clone();
    let results: Vec<_> = (0..129)
        .map(|i| {
            let mut result = runtime_task_result_fixture(&source);
            result.task_id = format!("task.unrelated.{i}");
            result
        })
        .collect();
    f.store
        .set_active_run_scheduler_task_results(&f.session, &f.run, results)
        .unwrap();
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::PopulationLimit));
    let value = "text";
    let bytes = serde_json::to_vec(value).unwrap();
    let mut exact = bytes.len();
    assert!(cohort_fingerprint(&value, &mut exact).is_ok());
    assert_eq!(exact, 0);
    let mut short = bytes.len() - 1;
    assert_eq!(
        cohort_fingerprint(&value, &mut short),
        Err(WorkflowCohortRefusal::SnapshotByteLimit)
    );
    assert!(
        WorkflowCohortOwnerEpoch::capture(" ".repeat(129), Arc::new(AtomicU64::new(1))).is_none()
    );
}

#[test]
fn cohort_capture_rejects_oversized_lookup_keys_before_absent_session_lookup() {
    let f = fixture(1, &[]);
    assert_eq!(
        f.store
            .frozen_cohort_snapshot(&"x".repeat(257), &f.run, &f.first),
        Err(WorkflowCohortRefusal::InvalidIdentity)
    );
    assert_eq!(
        f.store
            .frozen_cohort_snapshot(&f.session, &"x".repeat(129), &f.first),
        Err(WorkflowCohortRefusal::InvalidIdentity)
    );
    assert_eq!(
        f.store
            .frozen_cohort_snapshot(&f.session, &f.run, &"x".repeat(129)),
        Err(WorkflowCohortRefusal::InvalidIdentity)
    );
}

#[test]
fn cohort_capture_revalidates_callback_input_proof_completed_descriptor_and_population_mutations() {
    for mode in [3, 11, 12, 13] {
        let mut f = fixture(3, &[]);
        add_known_input(
            &mut f,
            WorkflowSchedulerTaskResultValue::String("known".into()),
        );
        let (service, _, session, run, first) = service(f, mode);
        assert!(
            matches!(
                service
                    .workflow_cohort_evidence_coverage(&session, &run, &first)
                    .unwrap(),
                WorkflowCohortCoverageResult::Refused(WorkflowCohortRefusal::SnapshotChanged)
            ),
            "mode {mode}"
        );
    }
}
#[test]
fn cohort_capture_refuses_pending_pair_cleanup_and_nonready_first() {
    let mut f = fixture(2, &[(0, 1)]);
    let snapshot = f.snapshot().unwrap();
    let first = snapshot
        .members()
        .iter()
        .find(|m| m.task.task_id == *snapshot.first_task_id())
        .unwrap();
    let pair = f
        .store
        .completion_successor_snapshot(
            &f.session,
            &f.run,
            &first.task,
            &first.record,
            snapshot.first_readiness_proof(),
        )
        .unwrap();
    f.store.install_completion_cleanup_gate(
        &f.session,
        &f.run,
        &pair,
        &super::super::super::WorkflowSchedulerTaskAttemptId::new(),
    );
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::CleanupPending));
    let mut f = fixture(2, &[]);
    let first_id = f.first.clone();
    f.edit(|_, records| {
        records
            .iter_mut()
            .find(|r| r.task_id.as_str() == first_id)
            .unwrap()
            .state = SchedulerTaskState::AwaitingInputs {
            diagnostics: vec![],
        };
    });
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::FirstNotAdmitted));
}

#[test]
fn cohort_capture_caps_graph_and_record_populations_before_collection() {
    for extra_records_only in [false, true] {
        let mut f = fixture(1, &[]);
        f.edit(|graph, records| {
            for i in 0..128 {
                let mut task = text_input_task(&format!("source.{i}"), "unused");
                task.workflow_id = graph.workflow_id.clone();
                task.workflow_run_id = graph.workflow_run_id.clone();
                let mut record = records[0].clone();
                record.task_id = task.task_id.clone();
                record.node_id = task.node_id.clone();
                record.state = SchedulerTaskState::AwaitingInputs {
                    diagnostics: vec![],
                };
                records.push(record);
                if !extra_records_only {
                    graph.tasks.push(task);
                }
            }
        });
        assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::PopulationLimit));
    }
}
#[test]
fn cohort_capture_caps_retained_proof_population_before_first_lookup() {
    let mut f = fixture(1, &[]);
    let proof = f.snapshot().unwrap().first_readiness_proof().clone();
    for i in 0..128 {
        let id = format!("task.old.{i}");
        f.edit(|graph, records| {
            graph.tasks.truncate(1);
            records.retain(|r| r.task_id == graph.tasks[0].task_id);
            let mut task = graph.tasks[0].clone();
            task.task_id = id.parse().unwrap();
            task.node_id = format!("node.old.{i}").parse().unwrap();
            let intent = task.schedulable_intent.as_mut().unwrap();
            intent.task_id = task.task_id.clone();
            intent.node_id = task.node_id.clone();
            let mut record = records[0].clone();
            record.task_id = task.task_id.clone();
            record.node_id = task.node_id.clone();
            record.state = SchedulerTaskState::Ready {
                execution_intent: SchedulerTaskExecutionIntent::runtime(intent.clone()),
            };
            graph.tasks.push(task);
            records.push(record);
        });
        f.store
            .record_active_run_runtime_dispatch_readiness_proof(
                &f.session,
                &f.run,
                &id,
                proof.clone(),
            )
            .unwrap();
    }
    assert_eq!(f.snapshot(), Err(WorkflowCohortRefusal::PopulationLimit));
}
