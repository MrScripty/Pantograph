use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerRepository, ExecutionGuaranteeLevel, LicenseSnapshot, ModelIdentity,
    ModelLicenseUsageEvent, ModelOutputMeasurement, OutputModality, RetentionClass,
    UsageEventStatus, UsageLineage,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId,
};

use super::*;

#[test]
fn workflow_diagnostics_usage_query_delegates_to_ledger_and_summarizes_events() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .record_usage_event(sample_event("usage-a", "model-a", Some("mit")))
        .expect("usage a");
    ledger
        .record_usage_event(sample_event("usage-b", "model-a", Some("mit")))
        .expect("usage b");
    ledger
        .record_usage_event(sample_event("usage-c", "model-b", Some("apache-2.0")))
        .expect("usage c");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            model_id: Some("model-a".to_string()),
            page_size: Some(10),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        })
        .expect("diagnostics query");

    assert_eq!(response.events.len(), 2);
    assert_eq!(response.summaries.len(), 1);
    assert_eq!(response.summaries[0].model_id, "model-a");
    assert_eq!(response.summaries[0].license_value.as_deref(), Some("mit"));
    assert_eq!(response.summaries[0].event_count, 2);
    assert_eq!(response.page_size, 10);
    assert_eq!(response.retention_policy.retention_days, 365);
}

#[test]
fn workflow_diagnostics_usage_query_validates_ids_and_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let invalid_id =
        service.workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            client_id: Some("bad\nid".to_string()),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        });
    assert!(matches!(
        invalid_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_page =
        service.workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            page_size: Some(501),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        });
    assert!(matches!(
        oversized_page,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

fn sample_event(
    usage_id: &str,
    model_id: &str,
    license_value: Option<&str>,
) -> ModelLicenseUsageEvent {
    ModelLicenseUsageEvent {
        usage_event_id: UsageEventId::try_from(usage_id.to_string()).unwrap(),
        client_id: ClientId::try_from("client-a".to_string()).unwrap(),
        client_session_id: ClientSessionId::try_from("session-a".to_string()).unwrap(),
        bucket_id: BucketId::try_from("bucket-a".to_string()).unwrap(),
        workflow_run_id: WorkflowRunId::try_from("run-a".to_string()).unwrap(),
        workflow_id: WorkflowId::try_from("workflow-a".to_string()).unwrap(),
        model: ModelIdentity {
            model_id: model_id.to_string(),
            model_revision: Some("rev-1".to_string()),
            model_hash: None,
            model_modality: Some("text".to_string()),
            runtime_backend: Some("pytorch".to_string()),
        },
        lineage: UsageLineage {
            node_id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            port_ids: vec!["text".to_string()],
            composed_parent_chain: Vec::new(),
            effective_contract_version: Some("v1".to_string()),
            effective_contract_digest: Some("digest-a".to_string()),
            metadata_json: None,
        },
        license_snapshot: LicenseSnapshot {
            license_value: license_value.map(str::to_string),
            source_metadata_json: Some(r#"{"source":"pumas"}"#.to_string()),
            model_metadata_snapshot_json: Some(r#"{"model":"snapshot"}"#.to_string()),
            unavailable_reason: None,
        },
        output_measurement: ModelOutputMeasurement {
            modality: OutputModality::Text,
            item_count: Some(1),
            character_count: Some(10),
            byte_size: Some(10),
            token_count: None,
            width: None,
            height: None,
            pixel_count: None,
            duration_ms: None,
            sample_rate_hz: None,
            channels: None,
            frame_count: None,
            vector_count: None,
            dimensions: None,
            numeric_representation: None,
            top_level_shape: None,
            schema_id: None,
            schema_digest: None,
            unavailable_reasons: Vec::new(),
        },
        guarantee_level: ExecutionGuaranteeLevel::ManagedFull,
        status: UsageEventStatus::Completed,
        retention_class: RetentionClass::Standard,
        started_at_ms: 10,
        completed_at_ms: Some(20),
        correlation_id: None,
    }
}
