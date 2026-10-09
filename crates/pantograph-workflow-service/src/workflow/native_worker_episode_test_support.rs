//! Controlled graph/readiness/candidate declarations, actual queue/Worker/attempt/result lifecycle.
//! This does not install a protected full-envelope provider or qualify declared RAM.
use super::*;
use crate::scheduler::{
    WorkflowSchedulerLifecycleComponentRegistryHandle, WorkflowSchedulerLifecycleOwnerId,
};
use crate::workflow::runtime_branch_task_event::*;
use crate::workflow::task_execution_worker::*;
use pantograph_runtime_host_contracts::*;
use pantograph_scheduler::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Candidate(WorkflowRuntimeDispatchCandidateFact);
impl WorkflowRuntimeDispatchCandidateProvider for Candidate {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        _: &SchedulerTaskStateRecord,
        _: &pantograph_dependency_planning::DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        if task
            .schedulable_intent
            .as_ref()
            .is_none_or(|i| i.task_id != self.0.resource_fit_assessment.task_id)
        {
            return Err(WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: "controlled candidate task changed".into(),
            });
        }
        let bundle = WorkflowRuntimeDispatchCandidateFactBundle {
            contract_version: super::runtime_dispatch_selection::WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
            facts: vec![self.0.clone()],
            diagnostics: vec![],
        };
        Ok(
            WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(
                bundle.try_into().map_err(|e| {
                    WorkflowRuntimeDispatchCandidateProviderError::Failed {
                        message: format!("{e}"),
                    }
                })?,
            ),
        )
    }
}
struct Host {
    node: String,
}
#[async_trait::async_trait]
impl WorkflowHost for Host {
    async fn workflow_io(&self, _: &str) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: vec![],
            outputs: vec![WorkflowIoNode {
                node_id: self.node.clone(),
                node_type: "embedding-inference".into(),
                name: None,
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "embedding".into(),
                    name: None,
                    description: None,
                    data_type: Some("vector".into()),
                    required: Some(true),
                    multiple: Some(false),
                }],
            }],
        })
    }
    async fn run_workflow(
        &self,
        _: &str,
        _: &[WorkflowPortBinding],
        _: Option<&[WorkflowOutputTarget]>,
        _: WorkflowRunOptions,
        _: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Err(WorkflowServiceError::Internal(
            "controlled episode must execute through TaskWorker".into(),
        ))
    }
}
/// Test-support driver only. Native production/drain/cleanup are supplied by the
/// existing serial port; task metadata and candidate/readiness remain controlled.
pub struct WorkflowControlledNativeWorkerEpisode {
    service: Arc<WorkflowService>,
    session: String,
    intent: SchedulableTaskIntent,
    episode_origin: Instant,
}
impl WorkflowControlledNativeWorkerEpisode {
    pub fn new(
        request: &RuntimeHostBatchExecutionRequest,
        port: Arc<dyn SerialRuntimeHostBatchExecutionPort>,
        fact: WorkflowRuntimeDispatchCandidateFact,
    ) -> Result<Self, WorkflowServiceError> {
        let episode_origin = Instant::now();
        let invalid = |s: &str| WorkflowServiceError::InvalidRequest(s.into());
        if request.members.len() != 1
            || !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(request)
        {
            return Err(invalid("controlled episode metadata bounds"));
        }
        request.validate().map_err(|e| invalid(&format!("{e:?}")))?;
        let m = &request.members[0];
        let intent = m.handoff.task_intent.clone();
        let service = Arc::new(
            WorkflowService::new_serial_ready_cpu(port, WorkflowSerialReadyConfig::default())
                .with_runtime_dispatch_candidate_provider(Arc::new(Candidate(fact))),
        );
        let mut store = service.session_store_guard()?;
        let session = store.create_session(
            intent.workflow_id.to_string(),
            None,
            None,
            vec!["candle".into()],
            vec![intent.model_ref.model_id.clone()],
            true,
        )?;
        let targets = vec![WorkflowOutputTarget {
            node_id: intent.node_id.to_string(),
            port_id: "embedding".into(),
        }];
        store.enqueue_run_with_id(
            &session,
            &WorkflowExecutionSessionRunRequest {
                session_id: session.clone(),
                workflow_semantic_version: "controlled.worker.v1".into(),
                inputs: vec![],
                output_targets: Some(targets.clone()),
                override_selection: None,
                timeout_ms: Some(10_000),
                priority: None,
            },
            intent.workflow_run_id.to_string(),
        )?;
        store.mark_runtime_loaded(&session, true)?;
        store
            .begin_queued_run(&session, intent.workflow_run_id.as_str())?
            .ok_or_else(|| invalid("controlled episode queue did not admit"))?;
        let context = crate::graph::WorkflowRuntimeSourceContext {
            operation_type: "embedding".into(),
            context_shape_key: "controlled.singleton.text".into(),
            cancellation_mode: "per-run-fanout".into(),
        };
        let source_task = SchedulerTaskId::parse("controlled.source").unwrap();
        let source_node = SchedulerNodeId::parse("controlled.source").unwrap();
        let mut task = WorkflowSchedulerTask {
            workflow_id: intent.workflow_id.clone(),
            workflow_run_id: intent.workflow_run_id.clone(),
            node_id: intent.node_id.clone(),
            task_id: intent.task_id.clone(),
            node_type: "embedding-inference".into(),
            execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            dependency_task_ids: vec![source_task.clone()],
            input_bindings: vec![WorkflowSchedulerTaskInputBinding {
                source_task_id: source_task.clone(),
                source_node_id: source_node.clone(),
                source_port_id: "text".into(),
                target_port_id: "text".into(),
            }],
            schedulable_intent: Some(intent.clone()),
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            runtime_source_context: Some(context.clone()),
            diagnostics: vec![],
        };
        let ready = SchedulerTaskStateRecord {
            contract_version: 1,
            workflow_id: intent.workflow_id.clone(),
            workflow_run_id: intent.workflow_run_id.clone(),
            node_id: intent.node_id.clone(),
            task_id: intent.task_id.clone(),
            state: SchedulerTaskState::Ready {
                execution_intent: SchedulerTaskExecutionIntent::runtime(intent.clone()),
            },
            state_version: 1,
            last_transition_id: "controlled.ready".parse().unwrap(),
        };
        let mut tasks = vec![task.clone()];
        task.node_id = source_node.clone();
        task.task_id = source_task.clone();
        task.execution_class = WorkflowSchedulerTaskExecutionClass::SourceInput;
        task.schedulable_intent = None;
        task.runtime_source_context = None;
        task.dependency_task_ids.clear();
        task.input_bindings.clear();
        task.source_input_task_template = Some(WorkflowSchedulerSourceInputTemplate::Text {
            port_id: "text".into(),
        });
        tasks.push(task);
        let source = SchedulerTaskStateRecord {
            contract_version: 1,
            workflow_id: intent.workflow_id.clone(),
            workflow_run_id: intent.workflow_run_id.clone(),
            node_id: source_node.clone(),
            task_id: source_task.clone(),
            state: SchedulerTaskState::Completed {
                execution_intent: SchedulerTaskExecutionIntent::SourceInput {
                    task_intent: SchedulerSourceInputTaskIntent {
                        contract_version: 1,
                        workflow_id: intent.workflow_id.clone(),
                        workflow_run_id: intent.workflow_run_id.clone(),
                        node_id: source_node.clone(),
                        task_id: source_task.clone(),
                        task_kind: "text-input".parse().unwrap(),
                    },
                },
            },
            state_version: 1,
            last_transition_id: "controlled.source-ready".parse().unwrap(),
        };
        let [input] = m.materialized_inputs.as_slice() else {
            return Err(invalid("controlled singleton input"));
        };
        let RuntimeHostExecutionInputValue::String(text) = &input.value else {
            return Err(invalid("controlled text input"));
        };
        store.set_active_run_scheduler_task_state(
            &session,
            intent.workflow_run_id.as_str(),
            WorkflowSchedulerTaskGraph {
                schema_version: 1,
                workflow_id: intent.workflow_id.clone(),
                workflow_run_id: intent.workflow_run_id.clone(),
                tasks,
            },
            vec![ready, source],
        )?;
        store.set_active_run_scheduler_task_results(
            &session,
            intent.workflow_run_id.as_str(),
            vec![WorkflowSchedulerTaskResult {
                schema_version: 1,
                workflow_id: intent.workflow_id.to_string(),
                workflow_run_id: intent.workflow_run_id.to_string(),
                node_id: source_node.to_string(),
                task_id: source_task.to_string(),
                status: WorkflowSchedulerTaskResultStatus::Completed,
                outputs: vec![WorkflowSchedulerTaskResultOutput {
                    port_id: "text".into(),
                    value: WorkflowSchedulerTaskResultValue::String(text.clone()),
                }],
                diagnostics: vec![],
                terminal_metadata: None,
            }],
        )?;
        store.record_active_run_runtime_dispatch_readiness_proof(
            &session,
            intent.workflow_run_id.as_str(),
            intent.task_id.as_str(),
            m.handoff.readiness_proof.clone(),
        )?;
        drop(store);
        let event =
            WorkflowRuntimeBranchTaskEventRecord::ready(WorkflowRuntimeBranchTaskEventRequest {
                event_id: WorkflowRuntimeBranchTaskEventId::parse("controlled.worker.event")
                    .unwrap(),
                session_id: session.clone(),
                workflow_id: intent.workflow_id.to_string(),
                workflow_run_id: intent.workflow_run_id.to_string(),
                scheduler_task_id: intent.task_id.to_string(),
                scheduler_task_attempt_id: None,
                attempt_generation: 1,
                queued_input_keys: vec![],
                output_targets: Some(targets),
                timeout_ms: Some(10_000),
                batching_key: None,
                runtime_source_context: context,
                batch_eligibility: None,
                ready_at_ms: crate::scheduler::unix_timestamp_ms().saturating_sub(1),
            })
            .map_err(|e| invalid(&format!("{e:?}")))?;
        service
            .runtime_branch_task_event_repository
            .lock()
            .unwrap()
            .enqueue(event)
            .map_err(|e| invalid(&format!("{e:?}")))?;
        Ok(Self {
            service,
            session,
            intent,
            episode_origin,
        })
    }
    pub fn timing_origin(&self) -> Instant {
        self.episode_origin
    }
    pub fn accepted_vector(&self) -> Option<serde_json::Value> {
        let mut store = self.service.session_store_guard().ok()?;
        store
            .active_run_scheduler_task_results(&self.session, self.intent.workflow_run_id.as_str())
            .ok()?
            .into_iter()
            .find(|r| {
                r.task_id == self.intent.task_id.as_str()
                    && r.status == WorkflowSchedulerTaskResultStatus::Completed
            })?
            .outputs
            .into_iter()
            .find_map(|o| match o.value {
                WorkflowSchedulerTaskResultValue::Json(v) if o.port_id == "embedding" => Some(v),
                _ => None,
            })
    }
    pub fn cancel(&self) -> Result<(), WorkflowServiceError> {
        let store = self.service.session_store_guard()?;
        let attempt = store.active_run_scheduler_task_attempt_id(
            &self.session,
            self.intent.workflow_run_id.as_str(),
            self.intent.task_id.as_str(),
        )?;
        self.service
            .scheduler_task_orchestrator
            .request_started_runtime_task_cancellation(
                &self.intent.task_id,
                &attempt,
                "controlled episode cancellation",
            )
            .map_err(|e| WorkflowServiceError::Internal(e.to_string()))
    }
    pub async fn run(&self) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let registry = WorkflowSchedulerLifecycleComponentRegistryHandle::new(
            WorkflowSchedulerLifecycleOwnerId::parse("controlled.native.worker").unwrap(),
        );
        let worker = WorkflowTaskExecutionWorker::spawn(
            registry,
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(
                self.service.clone(),
                Arc::new(Host {
                    node: self.intent.node_id.to_string(),
                }),
            ),
        )?;
        let command = WorkflowTaskExecutionWorkerRuntimeBranchCommand {
            session_id: self.session.clone(),
            workflow_run_id: self.intent.workflow_run_id.to_string(),
            workflow_id: self.intent.workflow_id.to_string(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: self.intent.node_id.to_string(),
                port_id: "embedding".into(),
            }]),
            timeout_ms: Some(10_000),
            start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started,
        };
        let (responder, completion) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let started = Instant::now();
        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                command, responder,
            ))
            .map_err(|e| WorkflowServiceError::Internal(format!("{e:?}")))?;
        let outcome = tokio::time::timeout(Duration::from_secs(15), completion)
            .await
            .map_err(|_| {
                WorkflowServiceError::RuntimeTimeout("controlled Worker episode timeout".into())
            })?
            .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;
        worker.shutdown().await?;
        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCompleted(outcome) = outcome else {
            return Err(WorkflowServiceError::Internal(format!(
                "controlled episode {outcome:?}"
            )));
        };
        eprintln!(
            "CONTROLLED_WORKER_EPISODE_NS {}",
            started.elapsed().as_nanos()
        );
        Ok(outcome.response)
    }
}
