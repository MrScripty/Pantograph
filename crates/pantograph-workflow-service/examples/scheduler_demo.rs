//! Real deterministic execution through scheduler sessions; no model or mock host output.
use std::{io::Read, path::PathBuf};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    let input: serde_json::Value = serde_json::from_str(&raw)?;
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
    let ledger_path = directory.path().join("diagnostics.sqlite");
    let service = WorkflowService::with_ephemeral_attribution_store()?
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(&ledger_path)?);
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
    let session = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "scheduler-demo".into(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await?;
    let result = service
        .run_workflow_execution_session(
            &host,
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
            &host,
            WorkflowExecutionSessionCloseRequest {
                session_id: session.session_id.clone(),
            },
        )
        .await?;
    let ledger = SqliteDiagnosticsLedger::open(&ledger_path)?;
    let events = ledger.diagnostic_events_after(0, 500)?;
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
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "scope": "scheduler-owned deterministic workflow; no model inference",
            "session_id": session.session_id,
            "graph": graph,
            "input": input,
            "execution": report,
            "diagnostics": events,
            "projections": projections,
        }))?
    );
    Ok(())
}
