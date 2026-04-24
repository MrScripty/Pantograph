use super::*;

#[tokio::test]
async fn attributed_workflow_run_records_backend_owned_run_before_execution() {
    let host = MockWorkflowHost::new(10, 256);
    let service =
        WorkflowService::with_ephemeral_attribution_store().expect("ephemeral attribution store");
    let registered = service
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("test client".to_string()),
            metadata_json: None,
        })
        .expect("register client");
    let opened = service
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: Some("test open".to_string()),
        })
        .expect("open client session");

    let response = service
        .workflow_run_attributed(
            &host,
            WorkflowAttributedRunRequest {
                credential: registered.credential_proof_request(),
                client_session_id: opened.session.client_session_id.clone(),
                bucket_selection: BucketSelection::Default,
                run: WorkflowRunRequest {
                    workflow_id: "wf-1".to_string(),
                    inputs: vec![WorkflowPortBinding {
                        node_id: "text-output-1".to_string(),
                        port_id: "text".to_string(),
                        value: serde_json::json!("hello world"),
                    }],
                    output_targets: Some(vec![WorkflowOutputTarget {
                        node_id: "text-output-1".to_string(),
                        port_id: "text".to_string(),
                    }]),
                    override_selection: None,
                    timeout_ms: None,
                    run_id: None,
                },
            },
        )
        .await
        .expect("attributed workflow run");

    assert_eq!(
        response.run.run_id,
        response.workflow_run.workflow_run_id.to_string()
    );
    assert_eq!(
        response.attribution.workflow_run_id,
        response.workflow_run.workflow_run_id
    );
    assert_eq!(
        response.attribution.client_session_id,
        opened.session.client_session_id
    );
    assert_eq!(
        response.workflow_run.bucket_id,
        opened.default_bucket.bucket_id
    );
    assert_eq!(response.run.outputs.len(), 1);
}

#[tokio::test]
async fn attributed_workflow_run_rejects_caller_supplied_run_id() {
    let host = MockWorkflowHost::new(10, 256);
    let service =
        WorkflowService::with_ephemeral_attribution_store().expect("ephemeral attribution store");
    let registered = service
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("test client".to_string()),
            metadata_json: None,
        })
        .expect("register client");
    let opened = service
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: None,
        })
        .expect("open client session");

    let err = service
        .workflow_run_attributed(
            &host,
            WorkflowAttributedRunRequest {
                credential: registered.credential_proof_request(),
                client_session_id: opened.session.client_session_id,
                bucket_selection: BucketSelection::Default,
                run: WorkflowRunRequest {
                    workflow_id: "wf-1".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("caller-run".to_string()),
                },
            },
        )
        .await
        .expect_err("caller run id rejected");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("run_id is backend-owned"));
}

#[tokio::test]
async fn attributed_workflow_run_uses_explicit_backend_owned_bucket() {
    let host = MockWorkflowHost::new(10, 256);
    let service =
        WorkflowService::with_ephemeral_attribution_store().expect("ephemeral attribution store");
    let registered = service
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("test client".to_string()),
            metadata_json: None,
        })
        .expect("register client");
    let opened = service
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: None,
        })
        .expect("open client session");
    let bucket = service
        .create_client_bucket(BucketCreateRequest {
            credential: registered.credential_proof_request(),
            name: "analysis".to_string(),
            metadata_json: None,
        })
        .expect("create client bucket");

    let response = service
        .workflow_run_attributed(
            &host,
            WorkflowAttributedRunRequest {
                credential: registered.credential_proof_request(),
                client_session_id: opened.session.client_session_id,
                bucket_selection: BucketSelection::Explicit(bucket.bucket_id.clone()),
                run: WorkflowRunRequest {
                    workflow_id: "wf-1".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: None,
                },
            },
        )
        .await
        .expect("attributed workflow run");

    assert_eq!(response.workflow_run.bucket_id, bucket.bucket_id);
    assert_eq!(response.attribution.bucket_id, bucket.bucket_id);
}
