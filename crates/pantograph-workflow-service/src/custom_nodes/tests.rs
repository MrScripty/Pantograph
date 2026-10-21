use super::*;
use async_trait::async_trait;
use pantograph_node_contracts::{NodeAuthoringMetadata, PortId};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub(crate) fn contract(name: &str) -> NodeTypeContract {
    NodeTypeContract {
        node_type: name.parse().unwrap(),
        category: NodeCategory::Processing,
        label: "Uppercase".into(),
        description: "Trusted string transformation".into(),
        inputs: vec![PortContract::input(
            "input".parse::<PortId>().unwrap(),
            "Input",
            PortValueType::String,
            PortRequirement::Required,
        )],
        outputs: vec![{
            let mut port = PortContract::output(
                "output".parse::<PortId>().unwrap(),
                "Output",
                PortValueType::String,
            );
            port.requirement = PortRequirement::Required;
            port
        }],
        execution_semantics: NodeExecutionSemantics::Batch,
        capability_requirements: vec![],
        inference_tasks: vec![],
        authoring: NodeAuthoringMetadata::default(),
        contract_version: Some("1.0.0".into()),
        contract_digest: None,
    }
}

pub(crate) fn digest(value: &str) -> String {
    format!("blake3:{}", blake3::hash(value.as_bytes()))
}
pub(crate) fn admitted(executor: Arc<dyn NodeExecutor>) -> TrustedCustomNode {
    TrustedCustomNode::admit(
        contract("custom-uppercase"),
        digest("immutable uppercase implementation"),
        digest("trusted rust host v1"),
        executor,
    )
    .unwrap()
}

#[derive(Clone, Copy)]
pub(crate) enum Behavior {
    Uppercase,
    Fail,
    Panic,
    Pending,
}
pub(crate) struct Executor {
    pub(crate) behavior: Behavior,
    pub(crate) calls: Arc<AtomicUsize>,
    pub(crate) dropped: Arc<AtomicBool>,
    pub(crate) entered: Arc<tokio::sync::Notify>,
    pub(crate) invocation: Arc<Mutex<Option<CustomNodeInvocation>>>,
}
impl Executor {
    pub(crate) fn new(behavior: Behavior) -> Self {
        Self {
            behavior,
            calls: Arc::new(AtomicUsize::new(0)),
            dropped: Arc::new(AtomicBool::new(false)),
            entered: Arc::new(tokio::sync::Notify::new()),
            invocation: Arc::new(Mutex::new(None)),
        }
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
        self.calls.fetch_add(1, Ordering::SeqCst);
        let invocation = extensions
            .get::<CustomNodeInvocation>(CUSTOM_NODE_INVOCATION_EXTENSION)
            .expect("host invocation extension");
        assert_eq!(task_id, invocation.task_id);
        assert!(context.get::<String>("untrusted").await.is_none());
        *self.invocation.lock() = Some(invocation.clone());
        let _drop = DropFlag(self.dropped.clone());
        self.entered.notify_one();
        match self.behavior {
            Behavior::Uppercase => Ok(HashMap::from([(
                "output".into(),
                Value::String(inputs["input"].as_str().unwrap().to_uppercase()),
            )])),
            Behavior::Fail => Err(node_engine::NodeEngineError::ExecutionFailed(
                "sensitive internal error".into(),
            )),
            Behavior::Panic => panic!("sensitive internal panic"),
            Behavior::Pending => std::future::pending().await,
        }
    }
}
fn invocation() -> CustomNodeInvocation {
    CustomNodeInvocation {
        workflow_id: "workflow.custom".into(),
        workflow_run_id: "run.custom".into(),
        task_id: "task.custom".into(),
        attempt_id: "attempt.custom".into(),
    }
}
fn inputs() -> HashMap<String, Value> {
    HashMap::from([("input".into(), Value::String("Mixed case".into()))])
}

#[test]
fn admission_rejects_unsupported_contracts_and_invalid_ids() {
    let base = contract("custom-uppercase");
    let mut cases = Vec::new();
    let mut c = base.clone();
    c.category = NodeCategory::Tool;
    cases.push(c);
    let mut c = base.clone();
    c.execution_semantics = NodeExecutionSemantics::Stream;
    cases.push(c);
    let mut c = base.clone();
    c.inputs[0].value_type = PortValueType::Json;
    cases.push(c);
    let mut c = base.clone();
    c.inputs[0].requirement = PortRequirement::Optional;
    cases.push(c);
    let mut c = base.clone();
    c.inputs[0].cardinality = PortCardinality::Multiple;
    cases.push(c);
    let mut c = base.clone();
    c.inputs.push(c.inputs[0].clone());
    cases.push(c);
    let mut c = base.clone();
    c.inputs.clear();
    cases.push(c);
    let mut c = base.clone();
    c.outputs = vec![c.outputs[0].clone(); 9];
    cases.push(c);
    let mut c = base.clone();
    c.capability_requirements
        .push(pantograph_node_contracts::NodeCapabilityRequirement::required("filesystem"));
    cases.push(c);
    let mut c = base.clone();
    c.inputs[0].options_provider = Some(pantograph_node_contracts::PortOptionsProviderRef::new(
        c.node_type.clone(),
        c.inputs[0].id.clone(),
    ));
    cases.push(c);
    let mut encoded = serde_json::to_value(&base).unwrap();
    encoded["node_type"] = Value::String("invalid type".into());
    cases.push(serde_json::from_value(encoded).unwrap());
    for contract in cases {
        assert!(matches!(
            TrustedCustomNode::admit(
                contract,
                digest("code"),
                digest("env"),
                Arc::new(Executor::new(Behavior::Uppercase))
            ),
            Err(CustomNodeError::InvalidAdmission(_))
        ));
    }
    assert!(matches!(
        TrustedCustomNode::admit(
            base,
            "mutable-file-path".into(),
            digest("env"),
            Arc::new(Executor::new(Behavior::Uppercase))
        ),
        Err(CustomNodeError::InvalidAdmission(_))
    ));
}

#[test]
fn catalog_rejects_duplicate_and_builtin_names_including_palette_only() {
    assert!(matches!(
        CustomNodeCatalog::try_new(vec![
            admitted(Arc::new(Executor::new(Behavior::Uppercase))),
            admitted(Arc::new(Executor::new(Behavior::Uppercase)))
        ]),
        Err(CustomNodeError::InvalidAdmission(
            "duplicate or built-in node type"
        ))
    ));
    for name in ["text-input", "json-filter", "audio-output"] {
        let node = TrustedCustomNode::admit(
            contract(name),
            digest("code"),
            digest("env"),
            Arc::new(Executor::new(Behavior::Uppercase)),
        )
        .unwrap();
        assert!(matches!(
            CustomNodeCatalog::try_new(vec![node]),
            Err(CustomNodeError::InvalidAdmission(
                "duplicate or built-in node type"
            ))
        ));
    }
}

#[test]
fn identity_uses_content_and_both_host_attestations() {
    let node = admitted(Arc::new(Executor::new(Behavior::Uppercase)));
    for (code, env) in [
        ("changed implementation", "trusted rust host v1"),
        ("immutable uppercase implementation", "changed environment"),
    ] {
        let changed = TrustedCustomNode::admit(
            contract("custom-uppercase"),
            digest(code),
            digest(env),
            Arc::new(Executor::new(Behavior::Uppercase)),
        )
        .unwrap();
        assert_ne!(node.identity(), changed.identity());
        assert_ne!(
            node.contract().contract_digest,
            changed.contract().contract_digest
        );
    }
    let mut c = contract("custom-uppercase");
    c.label = "Changed".into();
    c.contract_digest = node.contract().contract_digest.clone();
    let changed = TrustedCustomNode::admit(
        c,
        node.identity().implementation_digest.clone(),
        node.identity().environment_digest.clone(),
        Arc::new(Executor::new(Behavior::Uppercase)),
    )
    .unwrap();
    assert_ne!(
        node.identity().contract_digest,
        changed.identity().contract_digest
    );
}

#[tokio::test]
async fn managed_invocation_validates_identity_and_complete_inputs_before_callback() {
    let executor = Arc::new(Executor::new(Behavior::Uppercase));
    let node = admitted(executor.clone());
    let identity = node.identity().clone();
    let catalog = Arc::new(CustomNodeCatalog::try_new(vec![node]).unwrap());
    let prepared = catalog.prepare(invocation()).unwrap();
    let mut wrong = identity.clone();
    wrong.environment_digest = digest("wrong");
    assert!(matches!(
        prepared.invoke(&wrong, inputs()).await,
        Err(CustomNodeError::IdentityMismatch)
    ));
    wrong.node_type = "unknown".into();
    assert!(matches!(
        prepared.invoke(&wrong, inputs()).await,
        Err(CustomNodeError::Unavailable)
    ));
    for invalid in [
        HashMap::new(),
        HashMap::from([("input".into(), Value::Bool(true))]),
        HashMap::from([(
            "input".into(),
            Value::String("é".repeat(CUSTOM_NODE_STRING_MAX_BYTES)),
        )]),
        HashMap::from([
            ("input".into(), Value::String("ok".into())),
            ("_data".into(), Value::Null),
        ]),
    ] {
        assert!(matches!(
            prepared.invoke(&identity, invalid).await,
            Err(CustomNodeError::InvalidPayload("inputs"))
        ));
    }
    assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
    let outputs = prepared.invoke(&identity, inputs()).await.unwrap();
    assert_eq!(outputs["output"], "MIXED CASE");
    assert_eq!(executor.invocation.lock().as_ref(), Some(&invocation()));
}

#[tokio::test]
async fn cancellation_before_invocation_never_calls_executor() {
    let executor = Arc::new(Executor::new(Behavior::Uppercase));
    let node = admitted(executor.clone());
    let identity = node.identity().clone();
    let catalog = Arc::new(CustomNodeCatalog::try_new(vec![node]).unwrap());
    let prepared = catalog.prepare(invocation()).unwrap();
    let i = invocation();
    catalog
        .cancel(&i.workflow_run_id, &i.task_id, &i.attempt_id)
        .unwrap();
    assert!(matches!(
        prepared.invoke(&identity, inputs()).await,
        Err(CustomNodeError::Cancelled)
    ));
    assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
    drop(prepared);
    assert!(catalog.active.lock().is_empty());
}

#[tokio::test]
async fn cancellation_drops_inflight_future_and_rejects_stale_attempt() {
    let executor = Arc::new(Executor::new(Behavior::Pending));
    let node = admitted(executor.clone());
    let identity = node.identity().clone();
    let catalog = Arc::new(CustomNodeCatalog::try_new(vec![node]).unwrap());
    let prepared = catalog.prepare(invocation()).unwrap();
    let (result, ()) = tokio::join!(prepared.invoke(&identity, inputs()), async {
        executor.entered.notified().await;
        let i = invocation();
        assert!(matches!(
            catalog.cancel(&i.workflow_run_id, &i.task_id, "stale-attempt"),
            Err(CustomNodeError::Unavailable)
        ));
        catalog
            .cancel(&i.workflow_run_id, &i.task_id, &i.attempt_id)
            .unwrap();
    });
    assert!(matches!(result, Err(CustomNodeError::Cancelled)));
    assert!(executor.dropped.load(Ordering::SeqCst));
    drop(prepared);
    assert!(catalog.active.lock().is_empty());
}

#[tokio::test(start_paused = true)]
async fn invocation_timeout_drops_future_without_outputs() {
    let executor = Arc::new(Executor::new(Behavior::Pending));
    let node = admitted(executor.clone());
    let identity = node.identity().clone();
    let catalog = Arc::new(CustomNodeCatalog::try_new(vec![node]).unwrap());
    let prepared = catalog.prepare(invocation()).unwrap();
    assert!(matches!(
        prepared.invoke(&identity, inputs()).await,
        Err(CustomNodeError::TimedOut)
    ));
    assert!(executor.dropped.load(Ordering::SeqCst));
    drop(prepared);
    assert!(catalog.active.lock().is_empty());
}

#[tokio::test]
async fn callback_error_and_panic_are_typed_and_do_not_disclose_callback_text() {
    for behavior in [Behavior::Fail, Behavior::Panic] {
        let node = admitted(Arc::new(Executor::new(behavior)));
        let identity = node.identity().clone();
        let catalog = Arc::new(CustomNodeCatalog::try_new(vec![node]).unwrap());
        let prepared = catalog.prepare(invocation()).unwrap();
        let error = prepared.invoke(&identity, inputs()).await.unwrap_err();
        assert!(matches!(
            (behavior, &error),
            (Behavior::Fail, CustomNodeError::CallbackFailed)
                | (Behavior::Panic, CustomNodeError::Panicked)
        ));
        assert!(!error.to_string().contains("sensitive"));
    }
}

#[tokio::test]
async fn invalid_callback_outputs_never_cross_managed_boundary() {
    for outputs in [
        HashMap::new(),
        HashMap::from([("output".into(), Value::Bool(true))]),
        HashMap::from([
            ("output".into(), Value::String("ok".into())),
            ("extra".into(), Value::Null),
        ]),
        HashMap::from([(
            "output".into(),
            Value::String("é".repeat(CUSTOM_NODE_STRING_MAX_BYTES)),
        )]),
    ] {
        let executor = node_engine::SyncCallbackNodeExecutor::new(move |_, _| Ok(outputs.clone()));
        let node = admitted(Arc::new(executor));
        let identity = node.identity().clone();
        let catalog = Arc::new(CustomNodeCatalog::try_new(vec![node]).unwrap());
        let prepared = catalog.prepare(invocation()).unwrap();
        assert!(matches!(
            prepared.invoke(&identity, inputs()).await,
            Err(CustomNodeError::InvalidPayload("outputs"))
        ));
    }
}

#[test]
fn aggregate_payload_limit_applies_even_when_each_string_is_valid() {
    let mut ports = Vec::new();
    let mut values = HashMap::new();
    for n in 0..5 {
        let id = format!("port{n}");
        ports.push(PortContract::input(
            id.parse().unwrap(),
            "Input",
            PortValueType::String,
            PortRequirement::Required,
        ));
        values.insert(id, Value::String("x".repeat(CUSTOM_NODE_STRING_MAX_BYTES)));
    }
    assert!(matches!(
        validate_payload(&ports, &values, "inputs"),
        Err(CustomNodeError::InvalidPayload("inputs"))
    ));
    assert!(matches!(
        validate_payload(&ports, &values, "outputs"),
        Err(CustomNodeError::InvalidPayload("outputs"))
    ));
}
