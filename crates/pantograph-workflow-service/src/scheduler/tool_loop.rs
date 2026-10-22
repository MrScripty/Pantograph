//! Serial pure orchestration under one already-started nonruntime attempt.

use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::custom_nodes::{
    CustomNodeError, CustomNodeIdentity, PreparedCustomNodeInvocation, CUSTOM_NODE_STRING_MAX_BYTES,
};
use crate::tool_loops::{
    ToolLoopAction, ToolLoopIdentity, ToolLoopObservation, ToolLoopPrimitiveInvocation,
    ToolLoopPrimitiveKind, ToolLoopTurnInput,
};

const LOOP_DEADLINE: Duration = Duration::from_secs(30);

#[derive(Serialize)]
struct ToolCallRecord {
    call_id: String,
    turn: u8,
    tool: String,
    identity: CustomNodeIdentity,
}

pub(crate) async fn execute(
    prepared: &PreparedCustomNodeInvocation,
    identity: &ToolLoopIdentity,
    prompt: String,
) -> Result<HashMap<String, Value>, CustomNodeError> {
    if prepared.catalog().tool_loop_identity(&identity.id) != Some(identity) {
        return Err(CustomNodeError::IdentityMismatch);
    }
    if prompt.len() > CUSTOM_NODE_STRING_MAX_BYTES {
        return Err(CustomNodeError::InvalidLoopAction);
    }
    // One deadline includes every child invocation, rather than resetting per turn.
    tokio::time::timeout(LOOP_DEADLINE, run(prepared, identity, prompt))
        .await
        .map_err(|_| CustomNodeError::TimedOut)?
}

async fn run(
    prepared: &PreparedCustomNodeInvocation,
    identity: &ToolLoopIdentity,
    prompt: String,
) -> Result<HashMap<String, Value>, CustomNodeError> {
    let mut observation = None;
    let mut calls = Vec::new();
    for turn in 1..=identity.max_turns {
        if prepared.is_cancelled() {
            return Err(CustomNodeError::Cancelled);
        }
        let input = serde_json::to_string(&ToolLoopTurnInput {
            prompt: prompt.clone(),
            turn,
            observation: observation.take(),
        })
        .map_err(|_| CustomNodeError::InvalidLoopAction)?;
        if input.len() > CUSTOM_NODE_STRING_MAX_BYTES {
            return Err(CustomNodeError::InvalidLoopAction);
        }
        let action = invoke(
            prepared,
            identity,
            &identity.controller,
            turn,
            ToolLoopPrimitiveKind::Controller,
            input,
        )
        .await?;
        let action: ToolLoopAction =
            serde_json::from_str(&action).map_err(|_| CustomNodeError::InvalidLoopAction)?;
        match action {
            ToolLoopAction::Complete { response } => {
                if response.len() > CUSTOM_NODE_STRING_MAX_BYTES {
                    return Err(CustomNodeError::InvalidLoopAction);
                }
                return Ok(HashMap::from([
                    ("response".into(), Value::String(response)),
                    ("turns".into(), Value::from(turn)),
                    (
                        "tool_calls".into(),
                        serde_json::to_value(calls)
                            .map_err(|_| CustomNodeError::InvalidLoopAction)?,
                    ),
                ]));
            }
            ToolLoopAction::CallTool { tool, input } => {
                let tool_identity = identity
                    .tools
                    .get(&tool)
                    .ok_or(CustomNodeError::UnauthorizedTool)?;
                // Do not execute a tool without budget to consume its result.
                if turn == identity.max_turns {
                    return Err(CustomNodeError::TurnLimit);
                }
                let call_id = format!("{}.turn.{turn}", prepared.invocation().attempt_id);
                let output = invoke(
                    prepared,
                    identity,
                    tool_identity,
                    turn,
                    ToolLoopPrimitiveKind::PureStringTransform {
                        tool: tool.clone(),
                        call_id: call_id.clone(),
                    },
                    input,
                )
                .await?;
                calls.push(ToolCallRecord {
                    call_id,
                    turn,
                    tool: tool.clone(),
                    identity: tool_identity.clone(),
                });
                observation = Some(ToolLoopObservation { tool, output });
                // Ready callbacks must not starve cancellation or other tasks.
                tokio::task::yield_now().await;
            }
        }
    }
    Err(CustomNodeError::TurnLimit)
}

async fn invoke(
    prepared: &PreparedCustomNodeInvocation,
    loop_identity: &ToolLoopIdentity,
    identity: &CustomNodeIdentity,
    turn: u8,
    kind: ToolLoopPrimitiveKind,
    input: String,
) -> Result<String, CustomNodeError> {
    let node = prepared
        .catalog()
        .get(&identity.node_type)
        .ok_or(CustomNodeError::Unavailable)?;
    let input_port = node.contract().inputs[0].id.as_str();
    let output_port = node.contract().outputs[0].id.as_str();
    let outputs = prepared
        .invoke_primitive(
            identity,
            HashMap::from([(input_port.to_string(), Value::String(input))]),
            Some(ToolLoopPrimitiveInvocation {
                loop_id: loop_identity.id.clone(),
                turn,
                identity: identity.clone(),
                kind,
            }),
        )
        .await?;
    outputs
        .get(output_port)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(CustomNodeError::InvalidLoopAction)
}
