use super::*;

#[tokio::test]
async fn workflow_session_capacity_rebalance_uses_host_selected_candidate() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let service = WorkflowService::with_capacity_limits(3, 2);

    let first = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("first".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create first keep-alive session");
    let second = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-2".to_string(),
                usage_profile: Some("second".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create second keep-alive session");
    let third = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-3".to_string(),
                usage_profile: Some("third".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create third session");
    let third_session_id = third.session_id.clone();

    let selecting_host = SelectingRuntimeHost::new(second.session_id.clone(), unloads.clone());

    service
        .run_workflow_session(
            &selecting_host,
            WorkflowSessionRunRequest {
                session_id: third_session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run third session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first(),
        Some(&(
            second.session_id.clone(),
            WorkflowSessionUnloadReason::CapacityRebalance,
        ))
    );
    assert!(
        unloads
            .iter()
            .any(|(session_id, _)| session_id == &third_session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|(session_id, _)| session_id == &first.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_affine_idle_runtime_by_default() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::new(unloads.clone());
    let service = WorkflowService::with_capacity_limits(3, 2);

    let affine = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create affine keep-alive session");
    let non_affine = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other".to_string(),
                usage_profile: Some("batch".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create non-affine keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(non_affine.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &affine.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_shared_model_idle_runtime() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::with_runtime_affinity(
        unloads.clone(),
        HashMap::from([
            ("wf-target".to_string(), vec!["llama_cpp".to_string()]),
            ("wf-shared-model".to_string(), vec!["llama_cpp".to_string()]),
            ("wf-other-model".to_string(), vec!["pytorch".to_string()]),
        ]),
        HashMap::from([
            ("wf-target".to_string(), vec!["model-a".to_string()]),
            ("wf-shared-model".to_string(), vec!["model-a".to_string()]),
            ("wf-other-model".to_string(), vec!["model-b".to_string()]),
        ]),
    );
    let service = WorkflowService::with_capacity_limits(3, 2);

    let shared_model = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared-model".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create shared-model keep-alive session");
    let other_model = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other-model".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create other-model keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-target".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(other_model.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &shared_model.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_shared_backend_idle_runtime() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::with_runtime_affinity(
        unloads.clone(),
        HashMap::from([
            ("wf-target".to_string(), vec!["llama_cpp".to_string()]),
            (
                "wf-shared-backend".to_string(),
                vec!["llama_cpp".to_string()],
            ),
            ("wf-other-backend".to_string(), vec!["pytorch".to_string()]),
        ]),
        HashMap::from([
            ("wf-target".to_string(), vec!["model-a".to_string()]),
            ("wf-shared-backend".to_string(), vec!["model-z".to_string()]),
            ("wf-other-backend".to_string(), vec!["model-a".to_string()]),
        ]),
    );
    let service = WorkflowService::with_capacity_limits(3, 2);

    let shared_backend = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared-backend".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create shared-backend keep-alive session");
    let other_backend = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other-backend".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create other-backend keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-target".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(other_backend.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &shared_backend.session_id)
    );
}
