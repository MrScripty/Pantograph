use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, DraftGraphValidationSummary, InferenceInterfaceContractError,
    InferenceInterfaceDiagnostic, InferenceInterfaceDriftReport, InferenceInterfaceFingerprint,
    INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::inference_interface_patch::{
    InferenceInterfaceGraphPatchError, InferenceInterfaceUpdateProposal,
};

const MAX_EVENTS: usize = 512;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceInterfaceValidationSessionError {
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("{field} contains {actual_len} items; maximum is {max_len}")]
    TooManyItems {
        field: &'static str,
        actual_len: usize,
        max_len: usize,
    },
    #[error("inference interface contract error: {0}")]
    InferenceContract(#[from] InferenceInterfaceContractError),
    #[error("inference interface graph patch error: {0}")]
    GraphPatch(#[from] InferenceInterfaceGraphPatchError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphInferenceValidationSession {
    #[serde(default = "default_contract_version")]
    pub contract_version: u32,
    pub validation_session_id: DraftGraphValidationSessionId,
    pub client_graph_revision: u64,
    pub latest_sequence: u64,
    pub summary: DraftGraphValidationSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<WorkflowGraphInferenceValidationEvent>,
}

impl WorkflowGraphInferenceValidationSession {
    pub fn validate(&self) -> Result<(), InferenceInterfaceValidationSessionError> {
        validate_contract_version("validation_session.contract_version", self.contract_version)?;
        validate_revision(
            "validation_session.client_graph_revision",
            self.client_graph_revision,
        )?;
        validate_collection_len("validation_session.events", self.events.len(), MAX_EVENTS)?;
        self.summary.validate()?;
        let mut previous_sequence = 0;
        for event in &self.events {
            event.validate()?;
            if event.validation_session_id != self.validation_session_id {
                return Err(InferenceInterfaceValidationSessionError::InvalidField {
                    field: "validation_session.events.validation_session_id",
                    reason: "event session id must match validation session",
                });
            }
            if event.client_graph_revision != self.client_graph_revision {
                return Err(InferenceInterfaceValidationSessionError::InvalidField {
                    field: "validation_session.events.client_graph_revision",
                    reason: "event graph revision must match validation session",
                });
            }
            if event.sequence <= previous_sequence {
                return Err(InferenceInterfaceValidationSessionError::InvalidField {
                    field: "validation_session.events.sequence",
                    reason: "event sequences must be strictly increasing",
                });
            }
            previous_sequence = event.sequence;
        }
        if previous_sequence > self.latest_sequence {
            return Err(InferenceInterfaceValidationSessionError::InvalidField {
                field: "validation_session.latest_sequence",
                reason: "latest sequence must include all events",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphInferenceValidationEvent {
    pub validation_session_id: DraftGraphValidationSessionId,
    pub client_graph_revision: u64,
    pub sequence: u64,
    pub payload: WorkflowGraphInferenceValidationEventPayload,
}

impl WorkflowGraphInferenceValidationEvent {
    pub fn validate(&self) -> Result<(), InferenceInterfaceValidationSessionError> {
        validate_revision(
            "validation_event.client_graph_revision",
            self.client_graph_revision,
        )?;
        if self.sequence == 0 {
            return Err(InferenceInterfaceValidationSessionError::InvalidField {
                field: "validation_event.sequence",
                reason: "event sequence must be non-zero",
            });
        }
        self.payload.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum WorkflowGraphInferenceValidationEventPayload {
    DescriptorResolved(InferenceInterfaceFingerprint),
    DriftReported(InferenceInterfaceDriftReport),
    Diagnostic(InferenceInterfaceDiagnostic),
    UpdateProposal(InferenceInterfaceUpdateProposal),
    Summary(DraftGraphValidationSummary),
}

impl WorkflowGraphInferenceValidationEventPayload {
    fn validate(&self) -> Result<(), InferenceInterfaceValidationSessionError> {
        match self {
            Self::DescriptorResolved(_) => Ok(()),
            Self::DriftReported(report) => report.validate().map_err(Into::into),
            Self::Diagnostic(diagnostic) => diagnostic.validate().map_err(Into::into),
            Self::UpdateProposal(proposal) => proposal.validate().map_err(Into::into),
            Self::Summary(summary) => summary.validate().map_err(Into::into),
        }
    }
}

fn default_contract_version() -> u32 {
    INFERENCE_INTERFACE_CONTRACT_VERSION
}

fn validate_contract_version(
    field: &'static str,
    version: u32,
) -> Result<(), InferenceInterfaceValidationSessionError> {
    if version != INFERENCE_INTERFACE_CONTRACT_VERSION {
        return Err(InferenceInterfaceValidationSessionError::InvalidField {
            field,
            reason: "unsupported inference interface contract version",
        });
    }
    Ok(())
}

fn validate_revision(
    field: &'static str,
    revision: u64,
) -> Result<(), InferenceInterfaceValidationSessionError> {
    if revision == 0 {
        return Err(InferenceInterfaceValidationSessionError::InvalidField {
            field,
            reason: "client graph revision must be non-zero",
        });
    }
    Ok(())
}

fn validate_collection_len(
    field: &'static str,
    actual_len: usize,
    max_len: usize,
) -> Result<(), InferenceInterfaceValidationSessionError> {
    if actual_len > max_len {
        return Err(InferenceInterfaceValidationSessionError::TooManyItems {
            field,
            actual_len,
            max_len,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_inference_interface_contracts::{
        DraftGraphEnqueueDisabledReason, DraftGraphValidationStatus,
    };

    #[test]
    fn validation_session_accepts_monotonic_events() {
        let session = WorkflowGraphInferenceValidationSession {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            validation_session_id: validation_session_id(),
            client_graph_revision: 7,
            latest_sequence: 2,
            summary: pending_summary(),
            events: vec![
                event(
                    1,
                    WorkflowGraphInferenceValidationEventPayload::DescriptorResolved(fingerprint()),
                ),
                event(
                    2,
                    WorkflowGraphInferenceValidationEventPayload::Summary(pending_summary()),
                ),
            ],
        };

        session.validate().expect("session should validate");
    }

    #[test]
    fn validation_session_rejects_stale_revision_event() {
        let mut stale_event = event(
            1,
            WorkflowGraphInferenceValidationEventPayload::DescriptorResolved(fingerprint()),
        );
        stale_event.client_graph_revision = 6;
        let session = WorkflowGraphInferenceValidationSession {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            validation_session_id: validation_session_id(),
            client_graph_revision: 7,
            latest_sequence: 1,
            summary: pending_summary(),
            events: vec![stale_event],
        };

        assert_eq!(
            session.validate().expect_err("stale event must fail"),
            InferenceInterfaceValidationSessionError::InvalidField {
                field: "validation_session.events.client_graph_revision",
                reason: "event graph revision must match validation session"
            }
        );
    }

    #[test]
    fn validation_session_rejects_non_monotonic_sequences() {
        let session = WorkflowGraphInferenceValidationSession {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            validation_session_id: validation_session_id(),
            client_graph_revision: 7,
            latest_sequence: 1,
            summary: pending_summary(),
            events: vec![
                event(
                    1,
                    WorkflowGraphInferenceValidationEventPayload::DescriptorResolved(fingerprint()),
                ),
                event(
                    1,
                    WorkflowGraphInferenceValidationEventPayload::Summary(pending_summary()),
                ),
            ],
        };

        assert_eq!(
            session
                .validate()
                .expect_err("duplicate sequence must fail"),
            InferenceInterfaceValidationSessionError::InvalidField {
                field: "validation_session.events.sequence",
                reason: "event sequences must be strictly increasing"
            }
        );
    }

    fn event(
        sequence: u64,
        payload: WorkflowGraphInferenceValidationEventPayload,
    ) -> WorkflowGraphInferenceValidationEvent {
        WorkflowGraphInferenceValidationEvent {
            validation_session_id: validation_session_id(),
            client_graph_revision: 7,
            sequence,
            payload,
        }
    }

    fn pending_summary() -> DraftGraphValidationSummary {
        DraftGraphValidationSummary {
            status: DraftGraphValidationStatus::Pending,
            executable: false,
            enqueue_disabled_reasons: vec![DraftGraphEnqueueDisabledReason::ValidationPending],
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
        }
    }

    fn validation_session_id() -> DraftGraphValidationSessionId {
        DraftGraphValidationSessionId::parse("validation.session.1").unwrap()
    }

    fn fingerprint() -> InferenceInterfaceFingerprint {
        InferenceInterfaceFingerprint::parse("iface.test.v1").unwrap()
    }
}
