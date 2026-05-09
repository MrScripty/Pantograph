use std::collections::BTreeMap;

use pantograph_workflow_service::WorkflowDiagnosticsProjectionInvalidation;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const WORKFLOW_DIAGNOSTICS_PROJECTION_INVALIDATED_EVENT: &str =
    "workflow://diagnostics/projection-invalidated";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsProjectionInvalidationEvent {
    pub invalidations: Vec<WorkflowDiagnosticsProjectionInvalidation>,
}

pub fn emit_projection_invalidations(
    app: &AppHandle,
    invalidations: &[WorkflowDiagnosticsProjectionInvalidation],
) -> Result<(), tauri::Error> {
    let invalidations = coalesce_projection_invalidations(invalidations.iter().cloned());
    if invalidations.is_empty() {
        return Ok(());
    }

    app.emit(
        WORKFLOW_DIAGNOSTICS_PROJECTION_INVALIDATED_EVENT,
        WorkflowDiagnosticsProjectionInvalidationEvent { invalidations },
    )
}

fn coalesce_projection_invalidations(
    invalidations: impl IntoIterator<Item = WorkflowDiagnosticsProjectionInvalidation>,
) -> Vec<WorkflowDiagnosticsProjectionInvalidation> {
    let mut by_scope = BTreeMap::new();
    for invalidation in invalidations {
        let key = (
            invalidation.projection_kind,
            invalidation.workflow_run_id.clone(),
            invalidation.workflow_id.clone(),
        );
        by_scope
            .entry(key)
            .and_modify(|current: &mut WorkflowDiagnosticsProjectionInvalidation| {
                if invalidation.last_event_seq > current.last_event_seq
                    || (invalidation.last_event_seq == current.last_event_seq
                        && invalidation.updated_at_ms >= current.updated_at_ms)
                {
                    *current = invalidation.clone();
                }
            })
            .or_insert(invalidation);
    }
    by_scope.into_values().collect()
}

#[cfg(test)]
mod tests {
    use pantograph_workflow_service::{
        WorkflowDiagnosticsProjectionKind, WorkflowDiagnosticsProjectionRefreshReason,
    };

    use super::*;

    fn invalidation(
        projection_kind: WorkflowDiagnosticsProjectionKind,
        workflow_run_id: Option<&str>,
        workflow_id: Option<&str>,
        last_event_seq: i64,
        updated_at_ms: i64,
    ) -> WorkflowDiagnosticsProjectionInvalidation {
        WorkflowDiagnosticsProjectionInvalidation {
            projection_kind,
            workflow_run_id: workflow_run_id.map(ToOwned::to_owned),
            workflow_id: workflow_id.map(ToOwned::to_owned),
            last_event_seq,
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            updated_at_ms,
        }
    }

    #[test]
    fn coalesce_projection_invalidations_keeps_latest_per_projection_scope() {
        let invalidations = coalesce_projection_invalidations([
            invalidation(
                WorkflowDiagnosticsProjectionKind::RunDetail,
                Some("run-a"),
                Some("wf-a"),
                1,
                10,
            ),
            invalidation(
                WorkflowDiagnosticsProjectionKind::RunDetail,
                Some("run-a"),
                Some("wf-a"),
                3,
                20,
            ),
            invalidation(
                WorkflowDiagnosticsProjectionKind::RunList,
                None,
                Some("wf-a"),
                2,
                15,
            ),
            invalidation(
                WorkflowDiagnosticsProjectionKind::RunDetail,
                Some("run-b"),
                Some("wf-a"),
                2,
                18,
            ),
        ]);

        assert_eq!(invalidations.len(), 3);
        assert!(invalidations.iter().any(|event| {
            event.projection_kind == WorkflowDiagnosticsProjectionKind::RunDetail
                && event.workflow_run_id.as_deref() == Some("run-a")
                && event.last_event_seq == 3
        }));
        assert!(invalidations.iter().any(|event| {
            event.projection_kind == WorkflowDiagnosticsProjectionKind::RunDetail
                && event.workflow_run_id.as_deref() == Some("run-b")
                && event.last_event_seq == 2
        }));
        assert!(invalidations.iter().any(|event| {
            event.projection_kind == WorkflowDiagnosticsProjectionKind::RunList
                && event.workflow_id.as_deref() == Some("wf-a")
                && event.last_event_seq == 2
        }));
    }

    #[test]
    fn invalidation_event_payload_mirrors_backend_dto_shape() {
        let event = WorkflowDiagnosticsProjectionInvalidationEvent {
            invalidations: vec![invalidation(
                WorkflowDiagnosticsProjectionKind::IoArtifact,
                Some("run-a"),
                Some("wf-a"),
                7,
                42,
            )],
        };

        let value = serde_json::to_value(event).expect("event serializes");

        assert_eq!(
            value,
            serde_json::json!({
                "invalidations": [{
                    "projection_kind": "io_artifact",
                    "workflow_run_id": "run-a",
                    "workflow_id": "wf-a",
                    "last_event_seq": 7,
                    "reason": "explicit_refresh",
                    "updated_at_ms": 42
                }]
            })
        );
    }
}
