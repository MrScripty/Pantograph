use super::*;

#[test]
fn request_roundtrip_uses_snake_case() {
    let req = WorkflowRunRequest {
        workflow_id: "wf-1".to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("hello"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
        override_selection: Some(WorkflowTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama.cpp".to_string()),
        }),
        timeout_ms: None,
    };

    let json = serde_json::to_value(&req).expect("serialize request");
    assert_eq!(json["workflow_id"], "wf-1");
    assert_eq!(json["inputs"][0]["node_id"], "input-1");
    assert_eq!(json["output_targets"][0]["port_id"], "text");
    assert_eq!(json["override_selection"]["model_id"], "model-a");
    assert_eq!(json["override_selection"]["backend_key"], "llama.cpp");
}

#[test]
fn request_rejects_caller_authored_run_id() {
    let payload = serde_json::json!({
        "workflow_id": "wf-1",
        "inputs": [],
        "output_targets": null,
        "override_selection": null,
        "timeout_ms": null,
        "run_id": "caller-run-1"
    });

    let err = serde_json::from_value::<WorkflowRunRequest>(payload)
        .expect_err("old run_id field must be rejected");
    assert!(err.to_string().contains("unknown field `run_id`"));
}

#[test]
fn response_roundtrip_preserves_outputs() {
    let res = WorkflowRunResponse {
        workflow_run_id: "run-1".to_string(),
        outputs: vec![WorkflowPortBinding {
            node_id: "vector-output-1".to_string(),
            port_id: "vector".to_string(),
            value: serde_json::json!([0.1, 0.2, 0.3]),
        }],
        timing_ms: 5,
    };

    let json = serde_json::to_string(&res).expect("serialize response");
    let parsed: WorkflowRunResponse = serde_json::from_str(&json).expect("parse response");
    assert_eq!(parsed.workflow_run_id, "run-1");
    assert_eq!(parsed.outputs[0].node_id, "vector-output-1");
}

#[test]
fn workflow_io_roundtrip_uses_snake_case() {
    let response = WorkflowIoResponse {
        inputs: vec![WorkflowIoNode {
            node_id: "text-input-1".to_string(),
            node_type: "text-input".to_string(),
            name: Some("Prompt".to_string()),
            description: Some("Prompt input".to_string()),
            ports: vec![WorkflowIoPort {
                port_id: "text".to_string(),
                name: Some("Text".to_string()),
                description: None,
                data_type: Some("string".to_string()),
                required: Some(false),
                multiple: Some(false),
            }],
        }],
        outputs: vec![WorkflowIoNode {
            node_id: "text-output-1".to_string(),
            node_type: "text-output".to_string(),
            name: Some("Answer".to_string()),
            description: None,
            ports: vec![WorkflowIoPort {
                port_id: "text".to_string(),
                name: Some("Text".to_string()),
                description: None,
                data_type: Some("string".to_string()),
                required: Some(false),
                multiple: Some(false),
            }],
        }],
    };

    let json = serde_json::to_value(&response).expect("serialize workflow io");
    assert_eq!(json["inputs"][0]["node_id"], "text-input-1");
    assert_eq!(json["outputs"][0]["ports"][0]["port_id"], "text");

    let parsed: WorkflowIoResponse =
        serde_json::from_value(json).expect("parse workflow io response");
    assert_eq!(parsed.inputs[0].name.as_deref(), Some("Prompt"));
    assert_eq!(
        parsed.outputs[0].ports[0].data_type.as_deref(),
        Some("string")
    );
}

#[test]
fn workflow_service_error_envelope_roundtrip() {
    let err = WorkflowServiceError::OutputNotProduced(
        "requested output target 'vector-output-1.vector' was not produced".to_string(),
    );

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::OutputNotProduced);
    assert!(envelope.message.contains("vector-output-1.vector"));
    assert_eq!(envelope.details, None);

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(parsed.code, WorkflowErrorCode::OutputNotProduced);
    assert!(parsed.message.contains("vector-output-1.vector"));
    assert_eq!(parsed.details, None);
}

#[test]
fn workflow_service_cancelled_envelope_roundtrip() {
    let err = WorkflowServiceError::Cancelled("workflow run cancelled".to_string());

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::Cancelled);
    assert_eq!(envelope.message, "workflow run cancelled");
    assert_eq!(envelope.details, None);

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(parsed.code, WorkflowErrorCode::Cancelled);
    assert_eq!(parsed.message, "workflow run cancelled");
    assert_eq!(parsed.details, None);
}

#[test]
fn workflow_service_scheduler_busy_envelope_includes_structured_details() {
    let err = WorkflowServiceError::scheduler_runtime_capacity_exhausted(2, 2, 0);

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::SchedulerBusy);
    assert_eq!(
        envelope.details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(2, 2, 0),
        ))
    );

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(
        parsed.details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(2, 2, 0),
        ))
    );
}
