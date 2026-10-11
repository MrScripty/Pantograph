//! Real deterministic execution through scheduler sessions; no model or mock host output.
use std::{
    io::{BufRead, Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use pantograph_diagnostics_ledger::{DiagnosticsLedgerRepository, SqliteDiagnosticsLedger};
use pantograph_workflow_service::*;
use serde_json::json;

struct DemoHost(PathBuf);

#[async_trait]
impl WorkflowHost for DemoHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        vec![self.0.clone()]
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _targets: Option<&[WorkflowOutputTarget]>,
        _options: WorkflowRunOptions,
        _handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Err(WorkflowServiceError::Internal(
            "The demo must execute via scheduler-owned non-runtime tasks".into(),
        ))
    }
}

type DemoResult<T> = Result<T, Box<dyn std::error::Error>>;

struct DemoRuntime {
    _directory: tempfile::TempDir,
    host: DemoHost,
    graph: WorkflowGraph,
    service: WorkflowService,
    ledger_path: PathBuf,
    event_cursor: AtomicI64,
    library: Option<Arc<pumas_library::PumasApi>>,
}

impl DemoRuntime {
    async fn new(serve: bool) -> DemoResult<Self> {
        let directory = tempfile::tempdir()?;
        let host = DemoHost(directory.path().to_owned());
        let mut graph: WorkflowGraph =
            serde_json::from_str(include_str!("../../../examples/scheduler-demo/graph.json"))?;
        let registry = NodeRegistry::new();
        for node in &mut graph.nodes {
            node.data["definition"] = serde_json::to_value(
                registry
                    .get_definition(&node.node_type)
                    .ok_or("missing built-in node")?,
            )?;
        }
        std::fs::write(
            directory.path().join("scheduler-demo.json"),
            serde_json::to_vec(&json!({
                "metadata": {"name": "Scheduler JSON extraction"}, "graph": graph,
            }))?,
        )?;
        FileSystemWorkflowGraphStore::new(directory.path())
            .save_workflow("scheduler-demo".into(), graph.clone())?;
        let ledger_path = directory.path().join("diagnostics.sqlite");
        let service = WorkflowService::with_ephemeral_attribution_store()?
            .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(&ledger_path)?)
            .with_artifact_store(ArtifactStore::open(
                directory.path().join("artifacts"),
                ArtifactPolicy {
                    policy_id: "loopback-demo-small-scalar-bodies".into(),
                    policy_version: 1,
                    ttl_seconds: Some(3600),
                    max_disk_bytes: Some(8 * 1024 * 1024),
                    max_memory_bytes: Some(1024 * 1024),
                    max_single_artifact_bytes: Some(8192),
                    spill_threshold_bytes: Some(4096),
                    delete_on_consume: false,
                },
            )?);
        let edit = service
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
                workflow_id: Some("scheduler-demo".into()),
            })
            .await?;
        let validation = service
            .workflow_graph_refresh_current_validation_summary(
                WorkflowGraphCurrentValidationRefreshRequest {
                    graph_session_id: edit.session_id.clone(),
                    graph_revision: edit.graph_revision.parse()?,
                },
            )
            .await?;
        if !validation.summary.submit_gate.allowed {
            return Err("demo graph failed backend validation".into());
        }
        service
            .publish_graph_session_executable_validation_snapshot(
                WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                    workflow_id: "scheduler-demo".into(),
                    workflow_semantic_version: "1.0.0".into(),
                    graph_session_id: edit.session_id.clone(),
                    validation_session_id: validation.summary.validation_session_id,
                    validation_snapshot_id: None,
                },
            )
            .await?;
        service
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: edit.session_id,
            })
            .await?;
        let library = if serve {
            if std::env::var_os("PUMAS_REGISTRY_DB_PATH").is_none() {
                return Err(
                    "Start the browser host through serve.py to isolate its Pumas registry".into(),
                );
            }
            std::fs::create_dir_all(directory.path().join("model-library"))?;
            Some(Arc::new(
                pumas_library::PumasApi::builder(directory.path().join("model-library"))
                    .with_hf_client(false)
                    .with_process_manager(false)
                    .build()
                    .await?,
            ))
        } else {
            None
        };
        Ok(Self {
            _directory: directory,
            host,
            graph,
            service,
            ledger_path,
            library,
            event_cursor: AtomicI64::new(0),
        })
    }

    async fn run(&self, input: serde_json::Value) -> DemoResult<serde_json::Value> {
        let service = &self.service;
        let host = &self.host;
        let graph = &self.graph;
        let ledger = SqliteDiagnosticsLedger::open(&self.ledger_path)?;
        let prior = self.event_cursor.load(Ordering::Relaxed);
        let session = service
            .create_workflow_execution_session(
                host,
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: "scheduler-demo".into(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await?;
        let result = service
            .run_workflow_execution_session(
                host,
                WorkflowExecutionSessionRunRequest {
                    session_id: session.session_id.clone(),
                    workflow_semantic_version: "1.0.0".into(),
                    inputs: vec![WorkflowPortBinding {
                        node_id: "source".into(),
                        port_id: "value".into(),
                        value: input.clone(),
                    }],
                    output_targets: Some(vec![WorkflowOutputTarget {
                        node_id: "out".into(),
                        port_id: "text".into(),
                    }]),
                    override_selection: None,
                    timeout_ms: Some(10_000),
                    priority: None,
                },
            )
            .await;
        service
            .close_workflow_execution_session(
                host,
                WorkflowExecutionSessionCloseRequest {
                    session_id: session.session_id.clone(),
                },
            )
            .await?;
        let events = ledger.diagnostic_events_after(prior, 500)?;
        if let Some(last) = events.last() {
            self.event_cursor.store(last.event_seq, Ordering::Relaxed);
        }
        // Preserve real backend projections for browser-only inspection captures.
        // No display data, completion states, or model records are fabricated here.
        let run_id = events
            .iter()
            .find_map(|event| event.workflow_run_id.clone())
            .ok_or("demo did not record a workflow run")?;
        for projection in [
            "run_list",
            "run_detail",
            "node_status",
            "scheduler_timeline",
            "io_artifact",
            "library_usage",
        ] {
            service.workflow_projection_rebuild(WorkflowProjectionRebuildRequest {
                projection_name: projection.into(),
                batch_size: Some(500),
            })?;
        }
        let request = json!({"workflow_run_id": run_id});
        let projections = json!({
            "queryRunDetail": service.workflow_run_detail_query(serde_json::from_value(request.clone())?)?,
            "queryRunInspection": service.workflow_run_inspection_query(serde_json::from_value(request.clone())?)?,
            "queryRunList": service.workflow_run_list_query(Default::default())?,
            "queryNodeStatus": service.workflow_node_status_query(serde_json::from_value(request.clone())?)?,
            "querySchedulerTimeline": service.workflow_scheduler_timeline_query(serde_json::from_value(request.clone())?)?,
            "queryIoArtifacts": service.workflow_io_artifact_query(serde_json::from_value(request)?)?,
            "queryLibraryUsage": service.workflow_library_usage_query(Default::default())?,
        });
        let report = match result {
            Ok(response) => json!({"ok": true, "result": response}),
            Err(error) => json!({"ok": false, "error": error.to_string()}),
        };
        Ok(json!({
            "scope": "scheduler-owned deterministic workflow; no model inference",
            "session_id": session.session_id,
            "graph": graph,
            "input": input,
            "execution": report,
            "diagnostics": events,
            "projections": projections,
        }))
    }
}

#[path = "scheduler_demo/commands.rs"]
mod commands;

#[tokio::main]
async fn main() -> DemoResult<()> {
    let serve = std::env::args().any(|arg| arg == "--serve-stdio");
    let runtime = DemoRuntime::new(serve).await?;
    if serve {
        for line in std::io::stdin().lock().lines() {
            let response = match serde_json::from_str::<serde_json::Value>(&line?) {
                Ok(request) => runtime.dispatch(request).await,
                Err(error) => {
                    json!({"ok": false, "error": {"code": "invalid_request", "message": error.to_string()}})
                }
            };
            println!("{}", serde_json::to_string(&response)?);
            std::io::stdout().flush()?;
        }
    } else {
        let mut raw = String::new();
        std::io::stdin().read_to_string(&mut raw)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&runtime.run(serde_json::from_str(&raw)?).await?)?
        );
    }
    Ok(())
}
