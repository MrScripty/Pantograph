//! Canonical image-generation batch execution contracts.
//!
//! These DTOs describe already-planned batch members at the inference boundary.
//! They do not define a fallback execution strategy; gateway and backend code
//! must still fail closed until a true batch execution path is implemented.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::image_generation_planner::ImageGenerationExecutionPlan;
use crate::types::{ImageGenerationRequest, ImageGenerationResult};

pub const IMAGE_GENERATION_BATCH_ID_MAX_LEN: usize = 128;
pub const IMAGE_GENERATION_BATCH_MEMBER_ID_MAX_LEN: usize = 128;
pub const IMAGE_GENERATION_BATCH_MAX_MEMBERS: usize = 256;
pub const IMAGE_GENERATION_BATCH_MAX_DIAGNOSTICS: usize = 256;

/// Planned image-generation batch request accepted by gateway batch execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationBatchExecutionRequest {
    pub batch_execution_id: String,
    pub anchor_member_id: String,
    pub members: Vec<ImageGenerationBatchExecutionMemberRequest>,
}

impl ImageGenerationBatchExecutionRequest {
    pub fn validate(&self) -> Result<(), ImageGenerationBatchContractError> {
        validate_id(
            "batch_execution_id",
            &self.batch_execution_id,
            IMAGE_GENERATION_BATCH_ID_MAX_LEN,
        )?;
        validate_id(
            "anchor_member_id",
            &self.anchor_member_id,
            IMAGE_GENERATION_BATCH_MEMBER_ID_MAX_LEN,
        )?;

        if self.members.is_empty() {
            return Err(ImageGenerationBatchContractError::EmptyMembers);
        }
        if self.members.len() > IMAGE_GENERATION_BATCH_MAX_MEMBERS {
            return Err(ImageGenerationBatchContractError::TooManyMembers {
                max: IMAGE_GENERATION_BATCH_MAX_MEMBERS,
                actual: self.members.len(),
            });
        }

        let mut member_ids = HashSet::with_capacity(self.members.len());
        for member in &self.members {
            member.validate()?;
            if !member_ids.insert(member.member_id.as_str()) {
                return Err(ImageGenerationBatchContractError::DuplicateMemberId {
                    member_id: member.member_id.clone(),
                });
            }
        }

        if !member_ids.contains(self.anchor_member_id.as_str()) {
            return Err(ImageGenerationBatchContractError::UnknownAnchorMemberId {
                anchor_member_id: self.anchor_member_id.clone(),
            });
        }

        Ok(())
    }
}

/// One planned image-generation member in a batch execution request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationBatchExecutionMemberRequest {
    pub member_id: String,
    pub request: ImageGenerationRequest,
    pub plan: ImageGenerationExecutionPlan,
}

impl ImageGenerationBatchExecutionMemberRequest {
    pub fn validate(&self) -> Result<(), ImageGenerationBatchContractError> {
        validate_id(
            "member_id",
            &self.member_id,
            IMAGE_GENERATION_BATCH_MEMBER_ID_MAX_LEN,
        )?;
        validate_request_plan_correlation(&self.member_id, &self.request, &self.plan)
    }
}

/// Batch execution response preserving batch/member correlation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationBatchExecutionResponse {
    pub batch_execution_id: String,
    pub state: ImageGenerationBatchExecutionState,
    pub members: Vec<ImageGenerationBatchExecutionMemberResponse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ImageGenerationBatchDiagnostic>,
}

impl ImageGenerationBatchExecutionResponse {
    pub fn validate(&self) -> Result<(), ImageGenerationBatchContractError> {
        validate_id(
            "batch_execution_id",
            &self.batch_execution_id,
            IMAGE_GENERATION_BATCH_ID_MAX_LEN,
        )?;
        validate_diagnostics("diagnostics", &self.diagnostics)?;

        if self.members.is_empty() {
            return Err(ImageGenerationBatchContractError::EmptyMembers);
        }
        if self.members.len() > IMAGE_GENERATION_BATCH_MAX_MEMBERS {
            return Err(ImageGenerationBatchContractError::TooManyMembers {
                max: IMAGE_GENERATION_BATCH_MAX_MEMBERS,
                actual: self.members.len(),
            });
        }

        let mut member_ids = HashSet::with_capacity(self.members.len());
        for member in &self.members {
            member.validate()?;
            if !member_ids.insert(member.member_id.as_str()) {
                return Err(ImageGenerationBatchContractError::DuplicateMemberId {
                    member_id: member.member_id.clone(),
                });
            }
        }

        validate_batch_state(self.state, &self.members, &self.diagnostics)
    }
}

/// Explicit batch execution state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageGenerationBatchExecutionState {
    Accepted,
    Running,
    Completed,
    PartiallyCompleted,
    Failed,
    Rejected,
    Cancelled,
}

impl ImageGenerationBatchExecutionState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::PartiallyCompleted
                | Self::Failed
                | Self::Rejected
                | Self::Cancelled
        )
    }
}

/// One image-generation batch member response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationBatchExecutionMemberResponse {
    pub member_id: String,
    pub state: ImageGenerationBatchMemberExecutionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ImageGenerationResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ImageGenerationBatchDiagnostic>,
}

impl ImageGenerationBatchExecutionMemberResponse {
    pub fn validate(&self) -> Result<(), ImageGenerationBatchContractError> {
        validate_id(
            "member_id",
            &self.member_id,
            IMAGE_GENERATION_BATCH_MEMBER_ID_MAX_LEN,
        )?;
        validate_diagnostics("member.diagnostics", &self.diagnostics)?;

        match self.state {
            ImageGenerationBatchMemberExecutionState::Completed => {
                if self.result.is_none() {
                    return Err(
                        ImageGenerationBatchContractError::MissingCompletedMemberResult {
                            member_id: self.member_id.clone(),
                        },
                    );
                }
            }
            ImageGenerationBatchMemberExecutionState::Failed
            | ImageGenerationBatchMemberExecutionState::Rejected
            | ImageGenerationBatchMemberExecutionState::Cancelled => {
                if self.result.is_some() {
                    return Err(ImageGenerationBatchContractError::TerminalMemberHasResult {
                        member_id: self.member_id.clone(),
                        state: self.state,
                    });
                }
                if self.diagnostics.is_empty() {
                    return Err(
                        ImageGenerationBatchContractError::TerminalMemberMissingDiagnostics {
                            member_id: self.member_id.clone(),
                            state: self.state,
                        },
                    );
                }
            }
            ImageGenerationBatchMemberExecutionState::Accepted
            | ImageGenerationBatchMemberExecutionState::Running => {
                if self.result.is_some() {
                    return Err(
                        ImageGenerationBatchContractError::NonTerminalMemberHasResult {
                            member_id: self.member_id.clone(),
                            state: self.state,
                        },
                    );
                }
            }
        }

        Ok(())
    }
}

/// Explicit per-member execution state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageGenerationBatchMemberExecutionState {
    Accepted,
    Running,
    Completed,
    Failed,
    Rejected,
    Cancelled,
}

impl ImageGenerationBatchMemberExecutionState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Rejected | Self::Cancelled
        )
    }
}

/// Bounded batch/member diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationBatchDiagnostic {
    pub code: ImageGenerationBatchDiagnosticCode,
    pub severity: ImageGenerationBatchDiagnosticSeverity,
    pub member_id: Option<String>,
    pub field_path: String,
    pub message: String,
}

/// Stable image-generation batch diagnostic codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageGenerationBatchDiagnosticCode {
    UnsupportedBatchExecution,
    BatchPlanningRejected,
    BatchExecutionRejected,
    MemberPlanningRejected,
    MemberExecutionFailed,
    MemberCancelled,
    ContractViolation,
}

/// Stable image-generation batch diagnostic severity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageGenerationBatchDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageGenerationBatchContractError {
    #[error("{field_path} must not be blank")]
    BlankId { field_path: &'static str },
    #[error("{field_path} must be at most {max} bytes, got {actual}")]
    IdTooLong {
        field_path: &'static str,
        max: usize,
        actual: usize,
    },
    #[error("image-generation batch must contain at least one member")]
    EmptyMembers,
    #[error("image-generation batch supports at most {max} members, got {actual}")]
    TooManyMembers { max: usize, actual: usize },
    #[error("image-generation batch member id '{member_id}' appears more than once")]
    DuplicateMemberId { member_id: String },
    #[error("image-generation batch anchor member id '{anchor_member_id}' is not present")]
    UnknownAnchorMemberId { anchor_member_id: String },
    #[error("image-generation batch diagnostics support at most {max} entries, got {actual}")]
    TooManyDiagnostics { max: usize, actual: usize },
    #[error("image-generation batch diagnostic at {field_path}[{index}] must have a message")]
    BlankDiagnosticMessage {
        field_path: &'static str,
        index: usize,
    },
    #[error(
        "image-generation batch diagnostic member id at {field_path}[{index}] must not be blank"
    )]
    BlankDiagnosticMemberId {
        field_path: &'static str,
        index: usize,
    },
    #[error(
        "image-generation batch member '{member_id}' request/plan field '{field_path}' mismatch"
    )]
    RequestPlanMismatch {
        member_id: String,
        field_path: &'static str,
    },
    #[error("image-generation batch member '{member_id}' completed without a result")]
    MissingCompletedMemberResult { member_id: String },
    #[error(
        "image-generation batch member '{member_id}' state {state:?} must not include a result"
    )]
    TerminalMemberHasResult {
        member_id: String,
        state: ImageGenerationBatchMemberExecutionState,
    },
    #[error("image-generation batch member '{member_id}' state {state:?} requires diagnostics")]
    TerminalMemberMissingDiagnostics {
        member_id: String,
        state: ImageGenerationBatchMemberExecutionState,
    },
    #[error(
        "image-generation batch member '{member_id}' state {state:?} must not include a result"
    )]
    NonTerminalMemberHasResult {
        member_id: String,
        state: ImageGenerationBatchMemberExecutionState,
    },
    #[error("image-generation batch completed state requires every member to be completed")]
    CompletedBatchHasNonCompletedMembers,
    #[error("image-generation batch partially_completed state requires completed and failed terminal members")]
    InvalidPartiallyCompletedBatchState,
    #[error("image-generation batch terminal state {state:?} requires terminal member states")]
    TerminalBatchHasNonTerminalMembers {
        state: ImageGenerationBatchExecutionState,
    },
    #[error("image-generation batch terminal state {state:?} requires diagnostics")]
    TerminalBatchMissingDiagnostics {
        state: ImageGenerationBatchExecutionState,
    },
    #[error("image-generation batch non-terminal state {state:?} must not contain only terminal members")]
    NonTerminalBatchHasOnlyTerminalMembers {
        state: ImageGenerationBatchExecutionState,
    },
}

fn validate_id(
    field_path: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), ImageGenerationBatchContractError> {
    if value.trim().is_empty() {
        return Err(ImageGenerationBatchContractError::BlankId { field_path });
    }
    if value.len() > max_len {
        return Err(ImageGenerationBatchContractError::IdTooLong {
            field_path,
            max: max_len,
            actual: value.len(),
        });
    }
    Ok(())
}

fn validate_diagnostics(
    field_path: &'static str,
    diagnostics: &[ImageGenerationBatchDiagnostic],
) -> Result<(), ImageGenerationBatchContractError> {
    if diagnostics.len() > IMAGE_GENERATION_BATCH_MAX_DIAGNOSTICS {
        return Err(ImageGenerationBatchContractError::TooManyDiagnostics {
            max: IMAGE_GENERATION_BATCH_MAX_DIAGNOSTICS,
            actual: diagnostics.len(),
        });
    }

    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic.message.trim().is_empty() {
            return Err(ImageGenerationBatchContractError::BlankDiagnosticMessage {
                field_path,
                index,
            });
        }
        if diagnostic
            .member_id
            .as_deref()
            .is_some_and(|member_id| member_id.trim().is_empty())
        {
            return Err(ImageGenerationBatchContractError::BlankDiagnosticMemberId {
                field_path,
                index,
            });
        }
    }

    Ok(())
}

fn validate_request_plan_correlation(
    member_id: &str,
    request: &ImageGenerationRequest,
    plan: &ImageGenerationExecutionPlan,
) -> Result<(), ImageGenerationBatchContractError> {
    if request.model != plan.model_ref.model_id {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "model",
        });
    }
    if request.prompt != plan.prompt {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "prompt",
        });
    }
    if request.negative_prompt != plan.negative_prompt {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "negative_prompt",
        });
    }
    if request.width != plan.width {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "width",
        });
    }
    if request.height != plan.height {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "height",
        });
    }
    if request.num_inference_steps != plan.num_inference_steps {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "num_inference_steps",
        });
    }
    if request.guidance_scale != plan.guidance_scale {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "guidance_scale",
        });
    }
    if request.seed != plan.seed {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "seed",
        });
    }
    if request.denoising_scheduler.as_deref()
        != plan.denoising_scheduler.as_ref().map(|id| id.as_str())
    {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "denoising_scheduler",
        });
    }
    if request.num_images_per_prompt != plan.num_images_per_prompt {
        return Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id: member_id.to_string(),
            field_path: "num_images_per_prompt",
        });
    }

    Ok(())
}

fn validate_batch_state(
    state: ImageGenerationBatchExecutionState,
    members: &[ImageGenerationBatchExecutionMemberResponse],
    diagnostics: &[ImageGenerationBatchDiagnostic],
) -> Result<(), ImageGenerationBatchContractError> {
    let terminal_count = members
        .iter()
        .filter(|member| member.state.is_terminal())
        .count();
    let completed_count = members
        .iter()
        .filter(|member| member.state == ImageGenerationBatchMemberExecutionState::Completed)
        .count();
    let all_terminal = terminal_count == members.len();
    let all_completed = completed_count == members.len();

    match state {
        ImageGenerationBatchExecutionState::Completed => {
            if !all_completed {
                return Err(
                    ImageGenerationBatchContractError::CompletedBatchHasNonCompletedMembers,
                );
            }
        }
        ImageGenerationBatchExecutionState::PartiallyCompleted => {
            if !all_terminal || completed_count == 0 || completed_count == members.len() {
                return Err(ImageGenerationBatchContractError::InvalidPartiallyCompletedBatchState);
            }
        }
        ImageGenerationBatchExecutionState::Failed
        | ImageGenerationBatchExecutionState::Rejected
        | ImageGenerationBatchExecutionState::Cancelled => {
            if !all_terminal {
                return Err(
                    ImageGenerationBatchContractError::TerminalBatchHasNonTerminalMembers { state },
                );
            }
            if diagnostics.is_empty() && members.iter().all(|member| member.diagnostics.is_empty())
            {
                return Err(
                    ImageGenerationBatchContractError::TerminalBatchMissingDiagnostics { state },
                );
            }
        }
        ImageGenerationBatchExecutionState::Accepted
        | ImageGenerationBatchExecutionState::Running => {
            if all_terminal {
                return Err(
                    ImageGenerationBatchContractError::NonTerminalBatchHasOnlyTerminalMembers {
                        state,
                    },
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "image_generation_batch_tests.rs"]
mod tests;
