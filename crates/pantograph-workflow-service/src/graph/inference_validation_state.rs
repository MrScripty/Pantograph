use std::collections::HashMap;

use pantograph_inference_interface_contracts::{
    DependencyEnvironmentActionIntent, DependencyEnvironmentActionIntentResult,
    DependencyEnvironmentActionIntentStatus, DraftGraphValidationSessionId,
    DraftGraphValidationStatus, DraftGraphValidationSummary, InferenceDiagnosticCode,
    InferenceDiagnosticSeverity, InferenceInterfaceDiagnostic,
    ValidatedDependencyEnvironmentActionIntent, WorkflowGraphRevision,
};
use tokio::sync::RwLock;

use super::inference_interface_validation::{
    InferenceInterfaceValidationSessionError, WorkflowGraphInferenceValidationSession,
};

#[derive(Debug)]
pub(crate) struct CurrentInferenceValidationStateStore {
    summaries:
        RwLock<HashMap<CurrentInferenceValidationStateKey, CurrentInferenceValidationStateRecord>>,
}

impl Default for CurrentInferenceValidationStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentInferenceValidationStateStore {
    pub(crate) fn new() -> Self {
        Self {
            summaries: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) async fn record_validation_session(
        &self,
        graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
        session: WorkflowGraphInferenceValidationSession,
    ) -> Result<(), InferenceInterfaceValidationSessionError> {
        session.validate()?;
        let key = CurrentInferenceValidationStateKey {
            graph_session_id,
            graph_revision: session.graph_revision.clone(),
        };
        let record = CurrentInferenceValidationStateRecord {
            validation_session_id: session.validation_session_id,
            summary: session.summary,
        };
        self.summaries.write().await.insert(key, record);
        Ok(())
    }

    pub(crate) async fn resolve_dependency_environment_action_intent(
        &self,
        request: DependencyEnvironmentActionIntentStateRequest,
    ) -> DependencyEnvironmentActionIntentResult {
        let intent = request.intent.into_inner();
        if request.current_graph_revision != intent.graph_revision {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::GraphRevisionMismatch,
                "Dependency environment action was requested for a stale graph revision.",
                Some(
                    "Refresh graph validation for the current graph before resolving dependencies.",
                ),
            );
        }

        if !request.target_node_exists {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::TargetNodeMissing,
                "Dependency environment action target node does not exist in the current graph.",
                None,
            );
        }

        let key = CurrentInferenceValidationStateKey {
            graph_session_id: intent.graph_session_id.clone(),
            graph_revision: intent.graph_revision.clone(),
        };
        let summaries = self.summaries.read().await;
        let Some(record) = summaries.get(&key) else {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::ValidationSummaryMissing,
                "Inference validation has not completed for this graph revision.",
                Some("Run descriptor validation before resolving dependency environments."),
            );
        };

        if intent
            .validation_session_id
            .as_ref()
            .is_some_and(|validation_session_id| {
                validation_session_id != &record.validation_session_id
            })
        {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::DescriptorStale,
                "Dependency environment action was requested for a stale validation session.",
                Some(
                    "Refresh graph validation for the current graph before resolving dependencies.",
                ),
            );
        }

        if !record.summary.executable {
            return blocked_dependency_environment_action_result(
                &intent,
                diagnostic_code_for_summary_status(record.summary.status),
                "Inference validation summary is not executable for this graph revision.",
                Some("Resolve blocking inference validation diagnostics before resolving dependencies."),
            );
        }

        blocked_dependency_environment_action_result(
            &intent,
            InferenceDiagnosticCode::DependencyRequirementsMissing,
            "Dependency environment request derivation requires dependency requirements for this graph revision.",
            Some("Resolve dependency requirements from the current validation state before checking or installing environments."),
        )
    }
}

fn diagnostic_code_for_summary_status(
    status: DraftGraphValidationStatus,
) -> InferenceDiagnosticCode {
    match status {
        DraftGraphValidationStatus::Pending => InferenceDiagnosticCode::GraphValidationPending,
        DraftGraphValidationStatus::Stale => InferenceDiagnosticCode::DescriptorStale,
        DraftGraphValidationStatus::Unresolved | DraftGraphValidationStatus::Unavailable => {
            InferenceDiagnosticCode::DescriptorUnavailable
        }
        DraftGraphValidationStatus::Blocked => InferenceDiagnosticCode::DriftDetected,
        DraftGraphValidationStatus::Executable => {
            InferenceDiagnosticCode::DependencyRequirementsMissing
        }
        _ => InferenceDiagnosticCode::DescriptorUnavailable,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyEnvironmentActionIntentStateRequest {
    pub intent: ValidatedDependencyEnvironmentActionIntent,
    pub current_graph_revision: WorkflowGraphRevision,
    pub target_node_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CurrentInferenceValidationStateKey {
    graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
    graph_revision: WorkflowGraphRevision,
}

#[derive(Debug, Clone)]
struct CurrentInferenceValidationStateRecord {
    validation_session_id: DraftGraphValidationSessionId,
    summary: DraftGraphValidationSummary,
}

fn blocked_dependency_environment_action_result(
    intent: &DependencyEnvironmentActionIntent,
    code: InferenceDiagnosticCode,
    message: &str,
    hint: Option<&str>,
) -> DependencyEnvironmentActionIntentResult {
    DependencyEnvironmentActionIntentResult {
        contract_version: intent.contract_version,
        graph_session_id: intent.graph_session_id.clone(),
        graph_revision: intent.graph_revision.clone(),
        validation_session_id: intent.validation_session_id.clone(),
        target_node_id: intent.target_node_id.clone(),
        action: intent.action,
        status: DependencyEnvironmentActionIntentStatus::Blocked,
        diagnostics: vec![InferenceInterfaceDiagnostic {
            severity: InferenceDiagnosticSeverity::Error,
            code,
            message: message.to_string(),
            hint: hint.map(str::to_string),
            port_id: None,
        }],
    }
}

#[cfg(test)]
mod tests {
    use pantograph_inference_interface_contracts::{
        DependencyEnvironmentAction, DraftGraphValidationStatus, InferenceDiagnosticCode,
    };

    use super::*;

    #[tokio::test]
    async fn action_intent_state_rejects_stale_graph_revision() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbb",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::GraphRevisionMismatch
        );
    }

    #[tokio::test]
    async fn action_intent_state_rejects_missing_target_node() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                false,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::TargetNodeMissing
        );
    }

    #[tokio::test]
    async fn action_intent_state_requires_current_summary() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::ValidationSummaryMissing
        );
    }

    #[tokio::test]
    async fn action_intent_state_blocks_pending_summary() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_session(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Pending,
                    false,
                ),
            )
            .await
            .expect("valid validation session");

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::GraphValidationPending
        );
    }

    #[tokio::test]
    async fn action_intent_state_rejects_stale_validation_session() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_session(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
            )
            .await
            .expect("valid validation session");

        let result = store
            .resolve_dependency_environment_action_intent(state_request_with_validation_session(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "validation.session.old",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::DescriptorStale
        );
    }

    #[tokio::test]
    async fn action_intent_state_accepts_executable_summary_until_requirements_derivation() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_session(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
            )
            .await
            .expect("valid validation session");

        let result = store
            .resolve_dependency_environment_action_intent(state_request_with_validation_session(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::DependencyRequirementsMissing
        );
    }

    fn state_request(
        graph_session_id: &str,
        intent_revision: &str,
        current_revision: &str,
        target_node_id: &str,
        target_node_exists: bool,
    ) -> DependencyEnvironmentActionIntentStateRequest {
        DependencyEnvironmentActionIntentStateRequest {
            intent: ValidatedDependencyEnvironmentActionIntent::try_from(
                DependencyEnvironmentActionIntent {
                    contract_version: 1,
                    graph_session_id: graph_session_id.parse().expect("valid graph session id"),
                    graph_revision: intent_revision.parse().expect("valid graph revision"),
                    validation_session_id: None,
                    target_node_id: target_node_id.parse().expect("valid target node id"),
                    action: DependencyEnvironmentAction::Resolve,
                },
            )
            .expect("valid action intent"),
            current_graph_revision: current_revision.parse().expect("valid current revision"),
            target_node_exists,
        }
    }

    fn state_request_with_validation_session(
        graph_session_id: &str,
        intent_revision: &str,
        current_revision: &str,
        validation_session_id: &str,
        target_node_id: &str,
        target_node_exists: bool,
    ) -> DependencyEnvironmentActionIntentStateRequest {
        let mut request = state_request(
            graph_session_id,
            intent_revision,
            current_revision,
            target_node_id,
            target_node_exists,
        );
        let mut intent = request.intent.into_inner();
        intent.validation_session_id = Some(
            validation_session_id
                .parse()
                .expect("valid validation session id"),
        );
        request.intent = ValidatedDependencyEnvironmentActionIntent::try_from(intent)
            .expect("valid action intent");
        request
    }

    fn validation_session(
        graph_revision: &str,
        status: DraftGraphValidationStatus,
        executable: bool,
    ) -> WorkflowGraphInferenceValidationSession {
        WorkflowGraphInferenceValidationSession {
            contract_version:
                pantograph_inference_interface_contracts::INFERENCE_INTERFACE_CONTRACT_VERSION,
            validation_session_id: "validation.session.1"
                .parse()
                .expect("valid validation session id"),
            graph_revision: graph_revision.parse().expect("valid graph revision"),
            latest_sequence: 0,
            summary: DraftGraphValidationSummary {
                status,
                executable,
                enqueue_disabled_reasons: Vec::new(),
                diagnostics_count: 0,
                blocking_diagnostics_count: 0,
            },
            events: Vec::new(),
        }
    }
}
