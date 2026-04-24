use super::*;

#[tokio::test]
async fn invalidate_all_session_runtimes_clears_loaded_state_for_active_sessions() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let first = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create first session");
    let second = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-2".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create second session");
    let third = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-3".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create third session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .mark_runtime_loaded(&first.session_id, true)
            .expect("mark first runtime loaded");
        store
            .mark_runtime_loaded(&second.session_id, true)
            .expect("mark second runtime loaded");
    }

    let mut invalidated = service
        .invalidate_all_session_runtimes()
        .expect("invalidate session runtimes");
    invalidated.sort();

    let mut expected = vec![first.session_id.clone(), second.session_id.clone()];
    expected.sort();
    assert_eq!(invalidated, expected);

    let first_status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: first.session_id,
        })
        .await
        .expect("first session status");
    let second_status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: second.session_id,
        })
        .await
        .expect("second session status");
    let third_status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: third.session_id,
        })
        .await
        .expect("third session status");

    assert_eq!(
        first_status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
    assert_eq!(
        second_status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
    assert_eq!(
        third_status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
}
