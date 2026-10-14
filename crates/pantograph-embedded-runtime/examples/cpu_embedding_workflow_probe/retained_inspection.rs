//! Reopen existing owner stores without constructing a model or execution runtime.
use super::*;
use pantograph_workflow_service::*;
use pumas_library::models::{
    PumasArtifactConsumer, PumasArtifactLoadTargetResolutionMode, PumasModelRef,
    ResolveModelArtifactLoadTargetRequest,
};

pub(super) async fn observe_model(api: &pumas_library::PumasApi) -> ProbeResult<Value> {
    let facts = api.resolve_model_package_facts(MODEL_ID).await?;
    let target = api
        .resolve_model_artifact_load_target(ResolveModelArtifactLoadTargetRequest {
            model_ref: PumasModelRef {
                model_id: MODEL_ID.into(),
                selected_artifact_id: facts.model_ref.selected_artifact_id.clone(),
                ..Default::default()
            },
            expected_artifact_kind: None,
            caller_observed_entry_path: None,
            caller_observed_package_facts_contract_version: None,
            resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
            consumer: PumasArtifactConsumer {
                consumer_name: "pantograph-embedded-runtime".into(),
                task_kind: Some("embedding".into()),
                runtime_family: Some("candle.cpu".into()),
            },
        })
        .await?;
    Ok(json!({
        "indexed_model": api.get_model(MODEL_ID).await?,
        "execution_descriptor": api.resolve_model_execution_descriptor(MODEL_ID).await?,
        "package_facts": facts, "load_target": target,
    }))
}

pub(super) fn inspect(root: &Path) -> ProbeResult<Value> {
    // Opening absent stores would create empty state and obscure the missing evidence.
    for file in [
        "result.json",
        "diagnostics.db",
        "attribution.db",
        "artifacts/manifest.json",
    ] {
        if !root.join(file).is_file() {
            return Err(format!("missing retained evidence: {file}").into());
        }
    }
    let result: Value = serde_json::from_slice(&std::fs::read(root.join("result.json"))?)?;
    if result["qualification"]["status"] != "passed" || result["shutdown_ok"] != true {
        return Err("retained probe did not qualify or shut down".into());
    }
    let store = ArtifactStore::open_with_default_policy(
        root.join("artifacts"),
        serde_json::from_value(json!({
            "policy_id":"unused-inspection-default", "policy_version":1,
            "delete_on_consume":false,
        }))?,
    )?;
    if store.policy().delete_on_consume {
        return Err("retained probe requires nondestructive artifact reads".into());
    }
    let service = WorkflowService::new()
        .with_attribution_store(
            pantograph_runtime_attribution::SqliteAttributionStore::open(
                root.join("attribution.db"),
            )?,
        )
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(root.join("diagnostics.db"))?)
        .with_artifact_store(store);
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
    let list = service.workflow_run_list_query(Default::default())?;
    if list.runs.len() != 1 || list.runs[0].workflow_id.as_str() != WORKFLOW_ID {
        return Err("retained probe must contain exactly its one scheduler run".into());
    }
    let run_id = list.runs[0].workflow_run_id.as_str();
    let request = json!({"workflow_run_id":run_id});
    let inspection =
        service.workflow_run_inspection_query(serde_json::from_value(request.clone())?)?;
    let mut bodies = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for artifact in &inspection.io_artifacts {
        if !seen.insert(artifact.payload_artifact_id.clone()) {
            continue;
        }
        let body = service.read_artifact_body(ArtifactReadRequest {
            artifact_id: artifact.payload_artifact_id.clone(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })?;
        let descriptor = service.artifact_descriptor(ArtifactDescriptorQueryRequest {
            artifact_id: artifact.payload_artifact_id.clone(),
        })?;
        if descriptor
            .artifact
            .as_ref()
            .map(|item| item.attribution.workflow_run_id.as_str())
            != Some(run_id)
        {
            return Err("retained artifact belongs to another scheduler run".into());
        }
        let value = match body.response.media_type.as_str() {
            "application/json" => serde_json::from_slice::<Value>(&body.body)?,
            "text/plain" => Value::String(std::str::from_utf8(&body.body)?.into()),
            _ => return Err("unsupported retained fixture artifact media type".into()),
        };
        bodies.push(
            json!({"descriptor":descriptor.artifact, "read":body.response,
            "body_blake3":blake3::hash(&body.body).to_hex().to_string(),
            "value":value}),
        );
    }
    if bodies.is_empty() {
        return Err("no retained scheduler artifact bodies".into());
    }
    Ok(
        json!({"contract_version":1, "scope":"fresh_process_retained_owner_stores",
        "run_id":run_id, "run_list":list, "inspection":inspection, "artifact_bodies":bodies,
        "scheduler_timeline":service.workflow_scheduler_timeline_query(serde_json::from_value(request)?)?,
        "library_usage":service.workflow_library_usage_query(Default::default())?}),
    )
}
