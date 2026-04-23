use super::*;

#[test]
fn runtime_preflight_prefers_selected_runtime_over_non_selected_match() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama_cpp".to_string()],
        &[
            WorkflowRuntimeCapability {
                runtime_id: "managed-llama".to_string(),
                display_name: "Managed llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: true,
                configured: true,
                can_install: false,
                can_remove: true,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: false,
                readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
                selected_version: Some("b8248".to_string()),
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string(), "llama.cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
            WorkflowRuntimeCapability {
                runtime_id: "remote-llama".to_string(),
                display_name: "Remote llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: false,
                configured: false,
                can_install: false,
                can_remove: false,
                source_kind: WorkflowRuntimeSourceKind::Host,
                selected: true,
                readiness_state: Some(WorkflowRuntimeReadinessState::Unknown),
                selected_version: None,
                supports_external_connection: false,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: Some("remote host is not configured".to_string()),
            },
        ],
    );

    assert_eq!(runtime_warnings.len(), 1);
    assert_eq!(blocking_runtime_issues.len(), 1);
    assert_eq!(blocking_runtime_issues[0].runtime_id, "remote-llama");
    assert!(blocking_runtime_issues[0].message.contains(
        "workflow requires backend 'llama_cpp' but Remote llama.cpp: remote host is not configured"
    ));
}

#[test]
fn runtime_preflight_uses_ready_fallback_when_no_runtime_is_selected() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama_cpp".to_string()],
        &[
            WorkflowRuntimeCapability {
                runtime_id: "missing-llama".to_string(),
                display_name: "Missing llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Missing,
                available: false,
                configured: false,
                can_install: true,
                can_remove: false,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: false,
                readiness_state: Some(WorkflowRuntimeReadinessState::Missing),
                selected_version: None,
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: vec!["llama-server".to_string()],
                unavailable_reason: None,
            },
            ready_runtime_capability(),
        ],
    );

    assert!(runtime_warnings.is_empty());
    assert!(blocking_runtime_issues.is_empty());
}

#[test]
fn runtime_preflight_matches_legacy_backend_aliases_against_canonical_capabilities() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama.cpp".to_string(), "PyTorch".to_string()],
        &[
            WorkflowRuntimeCapability {
                runtime_id: "llama_cpp".to_string(),
                display_name: "llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: true,
                configured: true,
                can_install: false,
                can_remove: true,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: true,
                readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
                selected_version: Some("b8248".to_string()),
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
            WorkflowRuntimeCapability {
                runtime_id: "pytorch".to_string(),
                display_name: "PyTorch".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: true,
                configured: true,
                can_install: false,
                can_remove: true,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: true,
                readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
                selected_version: None,
                supports_external_connection: true,
                backend_keys: vec!["torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
        ],
    );

    assert!(runtime_warnings.is_empty());
    assert!(blocking_runtime_issues.is_empty());
}

#[test]
fn runtime_preflight_reports_selected_version_readiness_context() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama_cpp".to_string()],
        &[WorkflowRuntimeCapability {
            runtime_id: "llama_cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            install_state: WorkflowRuntimeInstallState::Installed,
            available: false,
            configured: false,
            can_install: false,
            can_remove: true,
            source_kind: WorkflowRuntimeSourceKind::Managed,
            selected: true,
            readiness_state: Some(WorkflowRuntimeReadinessState::Validating),
            selected_version: Some("b8248".to_string()),
            supports_external_connection: true,
            backend_keys: vec!["llama_cpp".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }],
    );

    assert_eq!(runtime_warnings.len(), 1);
    assert_eq!(blocking_runtime_issues.len(), 1);
    assert!(blocking_runtime_issues[0]
        .message
        .contains("selected version 'b8248' is validating"));
}
