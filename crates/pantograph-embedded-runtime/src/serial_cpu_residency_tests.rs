use super::*;
use pantograph_workflow_service::graph::{
    authored_snapshot_from_descriptor, NodeRegistry, WorkflowGraph,
};
use serde_json::json;

const MODEL: &str = "embedding/qualification/synthetic-bert-8";

pub(crate) fn save_model_free_evidence(case: &str, observations: serde_json::Value) {
    let evidence = json!({"case":case,
        "host_dependency_inventory":cfg!(feature = "host-dependency-inventory"),
        "observations":observations});
    if let Some(root) = std::env::var_os("PANTOGRAPH_M4_C5_MODEL_FREE_EVIDENCE_ROOT") {
        std::fs::create_dir_all(&root).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(PathBuf::from(root).join(format!("{case}.json")))
            .unwrap();
        serde_json::to_writer_pretty(&mut file, &evidence).unwrap();
        file.sync_all().unwrap();
    }
    println!(
        "model-free case {case}; host inventory={}",
        cfg!(feature = "host-dependency-inventory")
    );
}

pub(crate) fn config(root: &std::path::Path) -> EmbeddedSerialCpuResidencyConfig {
    EmbeddedSerialCpuResidencyConfig {
        runtime: EmbeddedRuntimeConfig::new(root.join("app-data"), root.to_path_buf()),
        pumas_library_root: root.to_path_buf(),
        models: vec![EmbeddedSerialCpuModelDeclaration {
            model_ref: serde_json::from_value(
                json!({"model_id":MODEL,"selected_artifact_id":"main"}),
            )
            .unwrap(),
            resident_ram_bytes: 32 * 1024 * 1024,
            load_peak_ram_bytes: 600 * 1024 * 1024,
            load_peak_vram_bytes: 0,
        }],
        ram_capacity_bytes: 2 * 1024 * 1024 * 1024,
        vram_capacity_bytes: 1024 * 1024 * 1024,
        ram_safety_margin_bytes: 0,
        vram_safety_margin_bytes: 0,
        cpu_threads: 1,
        readiness_poll_interval: Duration::from_millis(20),
    }
}

pub(crate) fn seed(root: &std::path::Path) {
    fn copy(source: &std::path::Path, target: &std::path::Path) {
        std::fs::create_dir_all(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let target = target.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let target = root.join("shared-resources/models").join(MODEL);
    copy(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../inference/tests/fixtures/candle_bert/bert-8"),
        &target,
    );
    std::fs::write(target.join("metadata.json"),json!({
        "schema_version":2,"model_id":MODEL,"family":"qualification","model_type":"embedding",
        "official_name":"synthetic-bert-8","cleaned_name":"synthetic-bert-8","source_path":target,"entry_path":target,
        "storage_kind":"library_owned","selected_artifact_id":"main","selected_artifact_files":["model.safetensors"],
        "import_state":"ready","validation_state":"valid","pipeline_tag":"feature-extraction",
        "task_type_primary":"embedding","input_modalities":["text"],"output_modalities":["embedding"],
        "recommended_backend":"candle","runtime_engine_hints":["candle"]
    }).to_string()).unwrap();
}

#[tokio::test]
async fn invalid_facade_configuration_refuses_before_owner_or_filesystem_startup() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("not-created");
    let mut configuration = config(&root);
    configuration.ram_capacity_bytes = 0;
    assert!(matches!(
        EmbeddedSerialCpuResidency::new(configuration).await,
        Err(EmbeddedRuntimeError::Config { .. })
    ));
    assert!(!root.exists());
}

#[tokio::test]
async fn projection_refresh_publishes_existing_raw_evidence_without_native_work() {
    use pantograph_diagnostics_ledger::{
        DiagnosticEventAppendRequest, DiagnosticsLedgerRepository, NodeExecutionProjectionStatus,
        RunListProjectionStatus,
    };

    let directory = tempfile::tempdir().unwrap();
    seed(directory.path());
    let facade = EmbeddedSerialCpuResidency::new(config(directory.path()))
        .await
        .unwrap();
    let refresh = |batch_size| WorkflowDiagnosticsProjectionRefreshRequest {
        projections: vec![
            WorkflowDiagnosticsProjectionKind::RunDetail,
            WorkflowDiagnosticsProjectionKind::NodeStatus,
            WorkflowDiagnosticsProjectionKind::IoArtifact,
        ],
        workflow_run_id: Some("run-projection-fixture".into()),
        workflow_id: Some("workflow-projection-fixture".into()),
        reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
        batch_size,
    };
    let inspection = || WorkflowRunInspectionQueryRequest {
        workflow_run_id: "run-projection-fixture".into(),
        artifact_limit: Some(10),
        projection_batch_size: Some(10),
    };
    let result = async {
        let service = &facade.runtime.workflow_service;
        let body = b"retained projection fixture body";
        let artifact = service.write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact-projection-fixture".into()),
            payload_kind: ArtifactPayloadKind::Text,
            media_type: "text/plain".into(),
            format: None,
            attribution: ArtifactAttribution {
                workflow_run_id: "run-projection-fixture".into(),
                workflow_id: Some("workflow-projection-fixture".into()),
                workflow_version_id: None,
                node_id: Some("fixture-output".into()),
                port_id: Some("text".into()),
                model_id: None,
                runtime_id: None,
            },
            artifact_role: Some("node_output".into()),
            parent_artifact_id: None,
            revision_index: None,
            body: body.to_vec(),
        })?;
        // These are diagnostic fixtures, not fabricated native/run evidence.
        // The regression exercises projection visibility with no workflow run.
        for (index, payload) in [
            json!({"payload_type":"run_snapshot_accepted",
                "workflow_run_snapshot_id":"snapshot-projection-fixture",
                "workflow_presentation_revision_id":"presentation-projection-fixture",
                "workflow_execution_session_id":"session-projection-fixture",
                "node_versions":[]}),
            json!({"payload_type":"run_terminal","status":"completed","duration_ms":2}),
            json!({"payload_type":"node_execution_status","status":"completed",
                "started_at_ms":1,"completed_at_ms":3,"duration_ms":2}),
            json!({"payload_type":"io_artifact_observed","artifact_id":artifact.artifact_id,
                "artifact_role":"node_output","producer_node_id":"fixture-output",
                "producer_port_id":"text","media_type":"text/plain",
                "size_bytes":body.len(),"retention_state":"retained"}),
        ]
        .into_iter()
        .enumerate()
        {
            let event: DiagnosticEventAppendRequest = serde_json::from_value(json!({
                "source_component":if index == 2 { "node_execution" } else { "workflow_service" },
                "occurred_at_ms":index + 1,
                "workflow_run_id":"run-projection-fixture",
                "workflow_id":"workflow-projection-fixture","node_id":"fixture-output",
                "privacy_class":"system_metadata","retention_class":"audit_metadata",
                "payload":payload
            }))?;
            service.workflow_diagnostic_event_record(event)?;
        }
        let ledger = SqliteDiagnosticsLedger::open(
            directory.path().join("app-data/serial-cpu-diagnostics.db"),
        )?;
        let raw_before = ledger.diagnostic_events_after(0, 10)?;
        assert_eq!(raw_before.len(), 4);
        let stale = facade.workflow_run_inspection_query(inspection())?;
        assert!(stale.run.is_none());
        assert!(stale.node_statuses.is_empty());
        assert!(stale.io_artifacts.is_empty());
        assert_eq!(stale.run_projection_state.last_applied_event_seq, 0);
        for invalid in [0, 501] {
            assert!(matches!(
                facade.workflow_diagnostics_projection_refresh(refresh(invalid)),
                Err(WorkflowServiceError::InvalidRequest(_))
            ));
        }
        // One page cannot silently catch up the run projection past its bound.
        let first = facade.workflow_diagnostics_projection_refresh(refresh(1))?;
        assert!(first.failed.is_empty());
        assert_eq!(first.advanced[0].projection_state.last_applied_event_seq, 1);
        let updated = facade.workflow_diagnostics_projection_refresh(refresh(10))?;
        assert!(updated.failed.is_empty());
        assert_eq!(updated.advanced.len(), 3);
        assert_eq!(updated.invalidations.len(), 3);
        for advance in &updated.advanced {
            let expected = match advance.projection_kind {
                WorkflowDiagnosticsProjectionKind::NodeStatus => 3,
                _ => 4,
            };
            assert_eq!(advance.projection_state.last_applied_event_seq, expected);
        }
        let visible = facade.workflow_run_inspection_query(inspection())?;
        assert_eq!(
            visible.run.as_ref().unwrap().status,
            RunListProjectionStatus::Completed
        );
        assert_eq!(visible.node_statuses.len(), 1);
        assert_eq!(
            visible.node_statuses[0].status,
            NodeExecutionProjectionStatus::Completed
        );
        assert_eq!(visible.io_artifacts.len(), 1);
        assert_eq!(visible.io_artifacts[0].artifact_id, artifact.artifact_id);
        assert_eq!(
            facade
                .workflow_read_artifact_body(ArtifactReadRequest {
                    artifact_id: artifact.artifact_id,
                    byte_range_start: None,
                    byte_range_end_exclusive: None,
                })?
                .body,
            body
        );
        let raw_after = ledger.diagnostic_events_after(0, 10)?;
        assert_eq!(raw_before, raw_after);
        assert!(facade.registry_snapshot().reservations.is_empty());
        let diagnostics = facade.residency_diagnostics().unwrap();
        assert_eq!(
            (diagnostics.loads, diagnostics.retains, diagnostics.stops),
            (0, 0, 0)
        );
        assert!(!diagnostics.has_idle_owner);
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    let shutdown = facade.shutdown().await;
    result.unwrap();
    shutdown.unwrap();
    assert!(matches!(
        facade.workflow_diagnostics_projection_refresh(refresh(10)),
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
    save_model_free_evidence(
        "projection_refresh_publishes_existing_raw_evidence_without_native_work",
        json!({"native":facade.residency_diagnostics(),"registry":format!("{:?}",facade.registry_snapshot()),"shutdown":"ok"}),
    );
}

#[tokio::test]
async fn metadata_sessions_never_load_reserve_or_mint_physical_load_proof() {
    use pantograph_workflow_service::{
        WorkflowExecutionSessionRetentionHint as Retention, WorkflowHost,
    };
    let directory = tempfile::tempdir().unwrap();
    seed(directory.path());
    let facade = EmbeddedSerialCpuResidency::new(config(directory.path()))
        .await
        .unwrap();
    let result = async {
        let host = facade.runtime.host();
        assert!(
            host.can_load_session_runtime(
                "metadata",
                "missing-workflow",
                None,
                Retention::Ephemeral
            )
            .await?
        );
        host.load_session_runtime("metadata", "missing-workflow", None, Retention::Ephemeral)
            .await?;
        assert!(host
            .session_runtime_load_proof("metadata", "missing-workflow")
            .await?
            .is_none());
        assert!(host
            .load_session_runtime("metadata", "missing-workflow", None, Retention::KeepAlive)
            .await
            .is_err());
        assert!(facade.registry_snapshot().reservations.is_empty());
        let diagnostic = facade.residency_diagnostics().unwrap();
        assert_eq!(
            (diagnostic.loads, diagnostic.stops, diagnostic.retains),
            (0, 0, 0)
        );
        assert!(!diagnostic.has_idle_owner);
        Ok::<_, WorkflowServiceError>(())
    }
    .await;
    let shutdown = facade.shutdown().await;
    result.unwrap();
    shutdown.unwrap();
    facade.shutdown().await.unwrap();
    assert!(facade
        .register_attribution_client(ClientRegistrationRequest {
            display_name: None,
            metadata_json: None
        })
        .is_err());
    save_model_free_evidence(
        "metadata_sessions_never_load_reserve_or_mint_physical_load_proof",
        json!({"native":facade.residency_diagnostics(),"registry":format!("{:?}",facade.registry_snapshot()),"shutdown":"ok"}),
    );
}

#[cfg(not(feature = "host-dependency-inventory"))]
async fn default_execution_fixture() -> (
    tempfile::TempDir,
    EmbeddedSerialCpuResidency,
    String,
    String,
) {
    let directory = tempfile::tempdir().unwrap();
    seed(directory.path());
    let facade = EmbeddedSerialCpuResidency::new(config(directory.path()))
        .await
        .unwrap();
    let edit = author(&facade).await.unwrap();
    let registered = facade
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("model-free inventory prerequisite".into()),
            metadata_json: None,
        })
        .unwrap();
    let client = facade
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: None,
        })
        .unwrap();
    let session = facade
        .create_attributed_workflow_execution_session(
            WorkflowExecutionSessionAttributedCreateRequest {
                workflow_id: "serial-residency-test".into(),
                usage_profile: None,
                keep_alive: false,
                attribution: WorkflowExecutionSessionAttributionRequest {
                    credential: registered.credential_proof_request(),
                    client_session_id: client.session.client_session_id.as_str().into(),
                    bucket_selection: pantograph_runtime_attribution::BucketSelection::Default,
                },
            },
        )
        .await
        .unwrap();
    (directory, facade, session.session_id, edit)
}

#[cfg(not(feature = "host-dependency-inventory"))]
fn execution_request(session_id: &str) -> WorkflowExecutionSessionRunRequest {
    WorkflowExecutionSessionRunRequest {
        session_id: session_id.into(),
        workflow_semantic_version: "1.0.0".into(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".into(),
            port_id: "text".into(),
            value: json!("hello world"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "vectors".into(),
            port_id: "vector".into(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: Some(0),
    }
}

#[cfg(not(feature = "host-dependency-inventory"))]
async fn unchanged_execution_facts(
    directory: &std::path::Path,
    facade: &EmbeddedSerialCpuResidency,
    session_id: &str,
) -> serde_json::Value {
    use pantograph_diagnostics_ledger::DiagnosticsLedgerRepository;
    let snapshot = facade
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: session_id.into(),
        })
        .await
        .unwrap();
    let ledger =
        SqliteDiagnosticsLedger::open(directory.join("app-data/serial-cpu-diagnostics.db"))
            .unwrap();
    let events = ledger.diagnostic_events_after(0, 129).unwrap();
    assert!(events.len() <= 128);
    let native = facade.residency_diagnostics().unwrap();
    assert_eq!(
        (
            native.loads,
            native.forward_drains,
            native.stops,
            native.retains
        ),
        (0, 0, 0, 0)
    );
    let registry = facade.registry_snapshot();
    assert!(registry.reservations.is_empty());
    json!({"session":snapshot.session,"workflow_run_id":snapshot.workflow_run_id,"queue":snapshot.items,
        "ledger":events,"native":native,"registry":registry})
}

#[cfg(not(feature = "host-dependency-inventory"))]
async fn inventory_rejection_case(resume: bool) {
    let (directory, facade, session_id, edit) = default_execution_fixture().await;
    let load_entered = Arc::new(tokio::sync::Notify::new());
    let drain_entered = Arc::new(tokio::sync::Notify::new());
    facade
        .set_residency_after_load_gate(load_entered.clone(), Arc::new(tokio::sync::Notify::new()));
    facade.set_residency_after_drain_gate(
        drain_entered.clone(),
        Arc::new(tokio::sync::Notify::new()),
    );
    let before = unchanged_execution_facts(directory.path(), &facade, &session_id).await;
    let error = if resume {
        // An actual authored/attributed session and a valid run identifier; no
        // otherwise-resumable paused run is invented or submitted by this test.
        facade
            .resume_workflow_execution_session_runtime_dependency_readiness(
                WorkflowExecutionSessionResumeRequest {
                    session_id: session_id.clone(),
                    workflow_run_id: "run.model-free-inventory-prerequisite".into(),
                },
            )
            .await
            .unwrap_err()
    } else {
        facade
            .run_workflow_execution_session(execution_request(&session_id))
            .await
            .unwrap_err()
    };
    assert!(
        matches!(&error,WorkflowServiceError::InvalidRequest(message)
        if message == "serial CPU residency execution requires host-dependency-inventory")
    );
    let after = unchanged_execution_facts(directory.path(), &facade, &session_id).await;
    // Collection time changes on every registry query. Keep the actual raw
    // snapshots in evidence, and compare every state field, including runtime
    // transition timestamps, without this observation-only timestamp.
    let mut before_state = before.clone();
    let mut after_state = after.clone();
    for state in [&mut before_state, &mut after_state] {
        state["registry"]
            .as_object_mut()
            .unwrap()
            .remove("generated_at_ms")
            .unwrap();
    }
    assert_eq!(before_state, after_state);
    // The rejected request cannot reach either genuine native boundary.
    assert!(futures_util::poll!(Box::pin(load_entered.notified())).is_pending());
    assert!(futures_util::poll!(Box::pin(drain_entered.notified())).is_pending());
    facade
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest { session_id })
        .await
        .unwrap();
    facade
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: edit,
        })
        .await
        .unwrap();
    facade.shutdown().await.unwrap();
    save_model_free_evidence(
        if resume {
            "default_resume_requires_inventory_before_enqueue"
        } else {
            "default_run_requires_inventory_before_enqueue"
        },
        json!({"before":before,"after":after,"error":error.to_string(),"shutdown":"ok",
            "otherwise_resumable_paused_run_claimed":false}),
    );
}

#[cfg(not(feature = "host-dependency-inventory"))]
#[tokio::test]
async fn default_run_requires_inventory_before_enqueue() {
    inventory_rejection_case(false).await;
}

#[cfg(not(feature = "host-dependency-inventory"))]
#[tokio::test]
async fn default_resume_requires_inventory_before_enqueue() {
    inventory_rejection_case(true).await;
}

#[cfg(not(feature = "host-dependency-inventory"))]
#[tokio::test]
async fn closed_admission_precedes_missing_inventory() {
    let (_directory, facade, session_id, _edit) = default_execution_fixture().await;
    // Hold the existing operation read guard so shutdown is pending after it
    // closes admission; no native load/forward is started.
    let operation = facade.operations.read().await;
    let mut shutdown = Box::pin(facade.shutdown());
    assert!(futures_util::poll!(shutdown.as_mut()).is_pending());
    let run = facade
        .run_workflow_execution_session(execution_request(&session_id))
        .await
        .unwrap_err();
    let resume = facade
        .resume_workflow_execution_session_runtime_dependency_readiness(
            WorkflowExecutionSessionResumeRequest {
                session_id,
                workflow_run_id: "run.model-free-inventory-prerequisite".into(),
            },
        )
        .await
        .unwrap_err();
    for error in [&run, &resume] {
        assert!(
            matches!(error,WorkflowServiceError::InvalidRequest(message) if message == "serial CPU residency admission is closed")
        );
    }
    drop(operation);
    shutdown.await.unwrap();
    save_model_free_evidence(
        "closed_admission_precedes_missing_inventory",
        json!({"run":run.to_string(),"resume":resume.to_string(),"native":facade.residency_diagnostics(),"shutdown":"ok"}),
    );
}

pub(crate) async fn author(
    facade: &EmbeddedSerialCpuResidency,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut graph: WorkflowGraph = serde_json::from_value(json!({"nodes":[
        {"id":"deps","node_type":"dependency-environment","position":{"x":0,"y":0},"data":{"mode":"manual"}},
        {"id":"prompt","node_type":"text-input","position":{"x":0,"y":100},"data":{}},
        {"id":"infer","node_type":"llm-inference","position":{"x":300,"y":100},"data":{
            "task_kind":"embedding","runtime":"candle","device":"cpu",
            "runtime_source_context":{"operation_type":"embedding.text","context_shape_key":"embedding.one-text","cancellation_mode":"run_scoped"},
            "pumas_model_ref":{"model_id":MODEL,"selected_artifact_id":"main"}}},
        {"id":"vectors","node_type":"vector-output","position":{"x":600,"y":100},"data":{}}
    ],"edges":[
        {"id":"text","source":"prompt","source_handle":"text","target":"infer","target_handle":"text"},
        {"id":"vector","source":"infer","source_handle":"embedding","target":"vectors","target_handle":"vector"},
        {"id":"dep","source":"deps","source_handle":"dependency_environment_sidecar","target":"infer","target_handle":"dependency_environment_sidecar"}
    ]}))?;
    let registry = NodeRegistry::new();
    for node in &mut graph.nodes {
        if node.node_type != "llm-inference" {
            node.data["definition"] = serde_json::to_value(
                registry
                    .get_definition(&node.node_type)
                    .ok_or("missing node")?,
            )?;
        }
    }
    let preview = facade
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
            workflow_id: None,
        })
        .await?;
    let validation = facade
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: preview.session_id.clone(),
                graph_revision: preview.graph_revision.parse()?,
            },
        )
        .await?;
    let projection = validation
        .node_projections
        .iter()
        .find(|node| node.node_id.as_str() == "infer")
        .ok_or("missing descriptor")?;
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "infer")
        .unwrap()
        .data["inference_interface_snapshot"] =
        serde_json::to_value(authored_snapshot_from_descriptor(&projection.descriptor)?)?;
    facade
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: preview.session_id,
        })
        .await?;
    facade.workflow_graph_save(WorkflowGraphSaveRequest {
        name: "serial-residency-test".into(),
        graph: graph.clone(),
    })?;
    let edit = facade
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph,
            workflow_id: Some("serial-residency-test".into()),
        })
        .await?;
    let validation = facade
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: edit.session_id.clone(),
                graph_revision: edit.graph_revision.parse()?,
            },
        )
        .await?;
    let action = facade
        .workflow_graph_resolve_dependency_environment_action_intent(
            pantograph_inference_interface_contracts::DependencyEnvironmentActionIntent {
                contract_version: 1,
                graph_session_id: edit.session_id.parse()?,
                graph_revision: edit.graph_revision.parse()?,
                validation_session_id: validation.summary.validation_session_id.clone(),
                target_node_id: "deps".parse()?,
                action:
                    pantograph_inference_interface_contracts::DependencyEnvironmentAction::Resolve,
            },
        )
        .await?;
    assert_eq!(action.status,pantograph_inference_interface_contracts::DependencyEnvironmentActionIntentStatus::RequestReady);
    facade
        .publish_graph_session_executable_validation_snapshot(
            WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                workflow_id: "serial-residency-test".into(),
                workflow_semantic_version: "1.0.0".into(),
                graph_session_id: edit.session_id.clone(),
                validation_snapshot_id: None,
                validation_session_id: Some(
                    validation
                        .summary
                        .validation_session_id
                        .ok_or("missing validation")?,
                ),
            },
        )
        .await?;
    Ok(edit.session_id)
}

#[cfg(feature = "host-dependency-inventory")]
#[tokio::test]
async fn actual_facade_session_uses_private_load_owner_and_acknowledged_shutdown() {
    let directory = tempfile::tempdir().unwrap();
    seed(directory.path());
    let facade = EmbeddedSerialCpuResidency::new(config(directory.path()))
        .await
        .unwrap();
    let result = async {
        let edit = author(&facade).await?;
        let registered = facade.register_attribution_client(ClientRegistrationRequest {
            display_name: Some("CPU fixture".into()),
            metadata_json: None,
        })?;
        let client = facade.open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: None,
        })?;
        let attribution = WorkflowExecutionSessionAttributionRequest {
            credential: registered.credential_proof_request(),
            client_session_id: client.session.client_session_id.as_str().into(),
            bucket_selection: pantograph_runtime_attribution::BucketSelection::Default,
        };
        assert!(matches!(
            facade
                .create_attributed_workflow_execution_session(
                    WorkflowExecutionSessionAttributedCreateRequest {
                        workflow_id: "serial-residency-test".into(),
                        usage_profile: None,
                        keep_alive: true,
                        attribution: attribution.clone(),
                    }
                )
                .await,
            Err(WorkflowServiceError::InvalidRequest(_))
        ));
        assert!(facade.registry_snapshot().reservations.is_empty());
        let session = facade
            .create_attributed_workflow_execution_session(
                WorkflowExecutionSessionAttributedCreateRequest {
                    workflow_id: "serial-residency-test".into(),
                    usage_profile: None,
                    keep_alive: false,
                    attribution,
                },
            )
            .await?;
        let mut runs = Vec::new();
        for round in 0..2 {
            let execution = async {
                let mut result = facade
                    .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                        session_id: session.session_id.clone(),
                        workflow_semantic_version: "1.0.0".into(),
                        inputs: vec![WorkflowPortBinding {
                            node_id: "prompt".into(),
                            port_id: "text".into(),
                            value: json!("hello world"),
                        }],
                        output_targets: Some(vec![WorkflowOutputTarget {
                            node_id: "vectors".into(),
                            port_id: "vector".into(),
                        }]),
                        override_selection: None,
                        timeout_ms: None,
                        priority: Some(0),
                    })
                    .await;
                while matches!(
                    result,
                    Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
                ) {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    if let Some(request) = facade
                        .workflow_execution_session_runtime_dependency_readiness_resume_candidates(
                        )?
                        .into_iter()
                        .find(|request| request.session_id == session.session_id)
                    {
                        result = facade
                            .resume_workflow_execution_session_runtime_dependency_readiness(request)
                            .await;
                    }
                }
                result
            };
            let response = tokio::time::timeout(Duration::from_secs(30), execution).await??;
            let golden: serde_json::Value = serde_json::from_slice(&std::fs::read(
                directory
                    .path()
                    .join("shared-resources/models")
                    .join(MODEL)
                    .join("golden.json"),
            )?)?;
            let actual = response
                .outputs
                .iter()
                .find(|output| output.node_id == "vectors")
                .ok_or("missing actual vector")?
                .value
                .as_array()
                .ok_or("vector shape")?;
            let expected = golden["single_vectors"][0].as_array().unwrap();
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                assert!((actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-5);
            }
            assert!(!runs.contains(&response.workflow_run_id));
            runs.push(response.workflow_run_id);
            let diagnostic = facade.residency_diagnostics().unwrap();
            assert_eq!(
                (diagnostic.loads, diagnostic.retains, diagnostic.stops),
                (1, round, 0)
            );
            assert!(diagnostic.has_idle_owner);
            assert_eq!(
                facade.registry_snapshot().reservations.len(),
                1,
                "only genuine mode guard remains after task receipt ACK"
            );
        }
        facade
            .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                session_id: session.session_id,
            })
            .await?;
        facade
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: edit,
            })
            .await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    let operations = facade.operations.read().await;
    let mut first = Box::pin(facade.shutdown());
    assert!(futures_util::poll!(first.as_mut()).is_pending());
    drop(first);
    // The retained shutdown supervisor is blocked before native stop by the
    // controlled operation lifetime, even after its original caller disappeared.
    tokio::task::yield_now().await;
    assert!(facade.shutdown_state.lock().await.task.is_some());
    assert_eq!(facade.residency_diagnostics().unwrap().stops, 0);
    drop(operations);
    let shutdown = facade.shutdown().await;
    result.unwrap();
    shutdown.unwrap();
    facade.shutdown().await.unwrap();
    assert!(facade.shutdown_state.lock().await.task.is_none());
    assert!(facade.registry_snapshot().reservations.is_empty());
    assert_eq!(facade.residency_diagnostics().unwrap().stops, 1);
    assert!(!facade.residency_diagnostics().unwrap().has_idle_owner);
    let model: inference::PumasModelRef =
        serde_json::from_value(json!({"model_id":MODEL,"selected_artifact_id":"main"})).unwrap();
    assert!(facade
        .runtime
        .gateway
        .resident_cpu_serial_owner(&model)
        .is_none());
}
