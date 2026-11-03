use super::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use node_engine::{Context, ExecutorExtensions, NodeExecutor};
use parking_lot::Mutex;
use serde_json::{json, Value};

use crate::custom_nodes::tests::{contract, digest};
use crate::custom_nodes::{
    CustomNodeInvocation, TrustedCustomNode, CUSTOM_NODE_INVOCATION_EXTENSION,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Mode {
    RoundTrip,
    Repeat,
    Unauthorized,
    Malformed,
    ExtraField,
    Oversized,
    Pending,
    Fail,
    Panic,
    Uppercase,
    LargeObservation,
    DelayedController,
    DelayedTool,
    SixteenTurns,
}

pub(crate) struct Executor {
    mode: Mode,
    pub(crate) calls: Mutex<Vec<(CustomNodeInvocation, ToolLoopPrimitiveInvocation)>>,
    pub(crate) entered: tokio::sync::Notify,
    pub(crate) dropped: Arc<AtomicBool>,
}

impl Executor {
    pub(crate) fn new(mode: Mode) -> Arc<Self> {
        Arc::new(Self {
            mode,
            calls: Mutex::new(Vec::new()),
            entered: tokio::sync::Notify::new(),
            dropped: Arc::new(AtomicBool::new(false)),
        })
    }
}

struct DropFlag(Arc<AtomicBool>);
impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[async_trait]
impl NodeExecutor for Executor {
    async fn execute(
        &self,
        task_id: &str,
        inputs: HashMap<String, Value>,
        context: &Context,
        extensions: &ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, Value>> {
        let parent = extensions
            .get::<CustomNodeInvocation>(CUSTOM_NODE_INVOCATION_EXTENSION)
            .unwrap();
        let primitive = extensions
            .get::<ToolLoopPrimitiveInvocation>(TOOL_LOOP_PRIMITIVE_EXTENSION)
            .unwrap();
        assert_eq!(parent.task_id, task_id);
        assert!(context.get::<String>("untrusted").await.is_none());
        self.calls.lock().push((parent.clone(), primitive.clone()));
        let _drop = DropFlag(self.dropped.clone());
        self.entered.notify_one();
        if matches!(self.mode, Mode::DelayedController | Mode::DelayedTool) {
            tokio::time::sleep(std::time::Duration::from_secs(11)).await;
        }
        let input = inputs["input"].as_str().unwrap();
        let output = match self.mode {
            Mode::Pending => std::future::pending().await,
            Mode::Fail => {
                return Err(node_engine::NodeEngineError::ExecutionFailed(
                    "sensitive callback text".into(),
                ))
            }
            Mode::Panic => panic!("sensitive callback text"),
            Mode::Uppercase | Mode::DelayedTool => input.to_uppercase(),
            Mode::LargeObservation => "\\".repeat(1100),
            Mode::Malformed => "invalid JSON".into(),
            Mode::ExtraField => r#"{"action":"complete","response":"x","code":"execute"}"#.into(),
            Mode::Oversized => "x".repeat(2049),
            Mode::Unauthorized => serde_json::to_string(&ToolLoopAction::CallTool {
                tool: "not-granted".into(),
                input: "x".into(),
            })
            .unwrap(),
            Mode::RoundTrip | Mode::Repeat | Mode::DelayedController | Mode::SixteenTurns => {
                let state: ToolLoopTurnInput = serde_json::from_str(input).unwrap();
                let action = match state.observation {
                    Some(observation)
                        if !matches!(self.mode, Mode::Repeat)
                            && (!matches!(self.mode, Mode::SixteenTurns)
                                || state.turn == MAX_TURNS) =>
                    {
                        ToolLoopAction::Complete {
                            response: observation.output,
                        }
                    }
                    _ => ToolLoopAction::CallTool {
                        tool: "uppercase".into(),
                        input: state.prompt,
                    },
                };
                serde_json::to_string(&action).unwrap()
            }
        };
        Ok(HashMap::from([("output".into(), Value::String(output))]))
    }
}

pub(crate) fn node(name: &str, executor: Arc<Executor>) -> TrustedCustomNode {
    TrustedCustomNode::admit(
        contract(name),
        digest(&format!("{name}:{:?}", executor.mode)),
        digest("pure trusted host"),
        executor,
    )
    .unwrap()
}

pub(crate) fn registration(
    controller: &TrustedCustomNode,
    tool: &TrustedCustomNode,
    max_turns: u8,
) -> ToolLoopRegistration {
    ToolLoopRegistration {
        id: "pure-loop".into(),
        controller: controller.identity().clone(),
        tools: BTreeMap::from([("uppercase".into(), tool.identity().clone())]),
        max_turns,
    }
}

pub(crate) fn catalog(
    controller: Arc<Executor>,
    tool: Arc<Executor>,
    max_turns: u8,
) -> Arc<CustomNodeCatalog> {
    let controller = node("loop-controller", controller);
    let tool = node("loop-uppercase", tool);
    let registration = registration(&controller, &tool, max_turns);
    Arc::new(
        CustomNodeCatalog::try_new_with_tool_loops(vec![controller, tool], vec![registration])
            .unwrap(),
    )
}

pub(crate) fn invocation() -> CustomNodeInvocation {
    CustomNodeInvocation {
        workflow_id: "workflow".into(),
        workflow_run_id: "run".into(),
        task_id: "loop".into(),
        attempt_id: "attempt".into(),
    }
}

#[test]
fn tool_loop_admission_rejects_unknown_stale_duplicate_and_unbounded_authority() {
    for case in [
        "unknown",
        "stale",
        "zero",
        "turns",
        "tools",
        "name",
        "duplicate",
    ] {
        let controller = node("controller", Executor::new(Mode::RoundTrip));
        let tool = node("tool", Executor::new(Mode::Uppercase));
        let mut grant = registration(&controller, &tool, 2);
        match case {
            "unknown" => grant.controller.node_type = "missing".into(),
            "stale" => grant.tools.get_mut("uppercase").unwrap().adapter_version += 1,
            "zero" => grant.max_turns = 0,
            "turns" => grant.max_turns = MAX_TURNS + 1,
            "tools" => {
                grant.tools = (0..=MAX_TOOLS)
                    .map(|i| (format!("tool{i}"), tool.identity().clone()))
                    .collect()
            }
            "name" => grant.id = "untrusted/script.py".into(),
            _ => {}
        }
        let duplicate = registration(&controller, &tool, 2);
        let grants = if case == "duplicate" {
            vec![grant, duplicate]
        } else {
            vec![grant]
        };
        let error = CustomNodeCatalog::try_new_with_tool_loops(vec![controller, tool], grants)
            .err()
            .unwrap();
        assert!(
            match case {
                "unknown" => matches!(error, CustomNodeError::Unavailable),
                "stale" => matches!(error, CustomNodeError::IdentityMismatch),
                _ => matches!(error, CustomNodeError::InvalidLoop(_)),
            },
            "{case}: {error}"
        );
    }
}

#[tokio::test]
async fn tool_loop_authorized_round_trip_preserves_primitive_and_parent_ownership() {
    let controller = Executor::new(Mode::RoundTrip);
    let tool = Executor::new(Mode::Uppercase);
    let catalog = catalog(controller.clone(), tool.clone(), 3);
    let prepared = catalog.prepare(invocation()).unwrap();
    let result = crate::scheduler::tool_loop::execute(
        &prepared,
        catalog.tool_loop_identity("pure-loop").unwrap(),
        "Mixed case".into(),
    )
    .await
    .unwrap();
    assert_eq!(result["response"], json!("MIXED CASE"));
    assert_eq!(result["turns"], json!(2));
    assert_eq!(result["tool_calls"][0]["call_id"], json!("attempt.turn.1"));
    assert_eq!(controller.calls.lock().len(), 2);
    let calls = tool.calls.lock();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, invocation());
    assert_eq!(
        calls[0].1.identity,
        catalog.get("loop-uppercase").unwrap().identity().clone()
    );
    assert_eq!(
        calls[0].1.kind,
        ToolLoopPrimitiveKind::PureStringTransform {
            tool: "uppercase".into(),
            call_id: "attempt.turn.1".into()
        }
    );
}

#[tokio::test]
async fn tool_loop_rejects_actions_state_and_exhaustion_without_extra_tool_work() {
    for (mode, tool_mode, expected, tool_calls) in [
        (
            Mode::Unauthorized,
            Mode::Uppercase,
            "outside its host grant",
            0,
        ),
        (
            Mode::Malformed,
            Mode::Uppercase,
            "invalid bounded action",
            0,
        ),
        (
            Mode::ExtraField,
            Mode::Uppercase,
            "invalid bounded action",
            0,
        ),
        (
            Mode::Oversized,
            Mode::Uppercase,
            "bounded required string",
            0,
        ),
        (Mode::Repeat, Mode::Uppercase, "turn budget", 1),
        (
            Mode::RoundTrip,
            Mode::LargeObservation,
            "invalid bounded action",
            1,
        ),
        (Mode::RoundTrip, Mode::Fail, "callback failed", 1),
        (Mode::RoundTrip, Mode::Panic, "callback panicked", 1),
    ] {
        let controller = Executor::new(mode);
        let tool = Executor::new(tool_mode);
        let catalog = catalog(controller, tool.clone(), 2);
        let prepared = catalog.prepare(invocation()).unwrap();
        let error = crate::scheduler::tool_loop::execute(
            &prepared,
            catalog.tool_loop_identity("pure-loop").unwrap(),
            "prompt".into(),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{mode:?}: {error}");
        assert_eq!(tool.calls.lock().len(), tool_calls);
        assert!(!error.to_string().contains("sensitive"));
    }
}

#[tokio::test]
async fn tool_loop_pre_cancel_and_in_flight_cancel_dispose_owned_future() {
    for pre_cancel in [true, false] {
        let controller = Executor::new(Mode::Pending);
        let catalog = catalog(controller.clone(), Executor::new(Mode::Uppercase), 3);
        let prepared = catalog.prepare(invocation()).unwrap();
        if pre_cancel {
            catalog.cancel("run", "loop", "attempt").unwrap();
        }
        let (result, ()) = tokio::join!(
            crate::scheduler::tool_loop::execute(
                &prepared,
                catalog.tool_loop_identity("pure-loop").unwrap(),
                "x".into()
            ),
            async {
                if !pre_cancel {
                    controller.entered.notified().await;
                    catalog.cancel("run", "loop", "attempt").unwrap();
                }
            }
        );
        assert!(matches!(result, Err(CustomNodeError::Cancelled)));
        assert_eq!(controller.calls.lock().len(), usize::from(!pre_cancel));
        if !pre_cancel {
            assert!(controller.dropped.load(Ordering::SeqCst));
        }
        drop(prepared);
        assert!(catalog.cancel("run", "loop", "attempt").is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn tool_loop_deadline_covers_all_turns_and_drops_current_primitive() {
    let controller = Executor::new(Mode::DelayedController);
    let tool = Executor::new(Mode::DelayedTool);
    let catalog = catalog(controller.clone(), tool.clone(), 3);
    let prepared = catalog.prepare(invocation()).unwrap();
    let start = tokio::time::Instant::now();
    let result = crate::scheduler::tool_loop::execute(
        &prepared,
        catalog.tool_loop_identity("pure-loop").unwrap(),
        "x".into(),
    )
    .await;
    assert!(matches!(result, Err(CustomNodeError::TimedOut)));
    assert_eq!(start.elapsed(), std::time::Duration::from_secs(30));
    assert_eq!(controller.calls.lock().len(), 2);
    assert_eq!(tool.calls.lock().len(), 1);
    assert!(controller.dropped.load(Ordering::SeqCst));
}

#[test]
fn tool_loop_rejects_multi_port_primitives_at_admission() {
    let controller = node("controller", Executor::new(Mode::RoundTrip));
    let mut multi_contract = contract("multi");
    let mut port = multi_contract.inputs[0].clone();
    port.id = "another".parse().unwrap();
    multi_contract.inputs.push(port);
    let multi = TrustedCustomNode::admit(
        multi_contract,
        digest("multi"),
        digest("host"),
        Executor::new(Mode::Uppercase),
    )
    .unwrap();
    let registration = registration(&controller, &multi, 2);
    assert!(matches!(
        CustomNodeCatalog::try_new_with_tool_loops(vec![controller, multi], vec![registration]),
        Err(CustomNodeError::InvalidLoop(_))
    ));
}

#[tokio::test]
async fn tool_loop_tampered_template_identity_cannot_invoke_a_primitive() {
    let controller = Executor::new(Mode::RoundTrip);
    let tool = Executor::new(Mode::Uppercase);
    let catalog = catalog(controller.clone(), tool.clone(), 3);
    let prepared = catalog.prepare(invocation()).unwrap();
    let mut identity = catalog.tool_loop_identity("pure-loop").unwrap().clone();
    identity.max_turns += 1;
    assert!(matches!(
        crate::scheduler::tool_loop::execute(&prepared, &identity, "x".into()).await,
        Err(CustomNodeError::IdentityMismatch)
    ));
    assert!(controller.calls.lock().is_empty());
    assert!(tool.calls.lock().is_empty());
}

#[tokio::test]
async fn tool_loop_maximum_progress_has_unique_call_ids_and_bounded_results() {
    let controller = Executor::new(Mode::SixteenTurns);
    let tool = Executor::new(Mode::Uppercase);
    let catalog = catalog(controller.clone(), tool.clone(), MAX_TURNS);
    let prepared = catalog.prepare(invocation()).unwrap();
    let result = crate::scheduler::tool_loop::execute(
        &prepared,
        catalog.tool_loop_identity("pure-loop").unwrap(),
        "x".into(),
    )
    .await
    .unwrap();
    assert_eq!(result["turns"], json!(16));
    let calls = result["tool_calls"].as_array().unwrap();
    assert_eq!(calls.len(), 15);
    let ids: std::collections::BTreeSet<_> = calls
        .iter()
        .map(|call| call["call_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 15);
    assert_eq!(controller.calls.lock().len(), 16);
    assert_eq!(tool.calls.lock().len(), 15);
    assert!(
        serde_json::to_vec(&result["tool_calls"]).unwrap().len()
            < pantograph_runtime_host_contracts::RUNTIME_HOST_STRUCTURED_OUTPUT_MAX_BYTES
    );
}
