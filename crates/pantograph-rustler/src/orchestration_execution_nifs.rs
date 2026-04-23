use std::collections::HashMap;
use std::sync::Arc;

use node_engine::{EventSink, TaskExecutor, WorkflowGraph};
use rustler::{Atom, NifResult, ResourceArc};

use crate::atoms;
use crate::callback_bridge::{BeamEventSink, CoreFirstExecutor, ElixirCallbackTaskExecutor};
use crate::elixir_data_graph_executor::ElixirDataGraphExecutor;
use crate::resources::{InferenceGatewayResource, OrchestrationStoreResource};

pub(crate) fn execute(
    store_resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
    initial_data_json: String,
    callback_pid: rustler::LocalPid,
) -> NifResult<String> {
    let initial_data = parse_initial_data(initial_data_json)?;
    let graph = get_orchestration_graph(&store_resource, &graph_id)?;
    let runtime = create_runtime()?;

    let core = node_engine::CoreTaskExecutor::new();
    let elixir = ElixirCallbackTaskExecutor::new(callback_pid);
    let task_executor: Arc<dyn TaskExecutor> = Arc::new(CoreFirstExecutor::new(core, elixir));
    let event_sink = BeamEventSink::new(callback_pid);

    let data_executor =
        ElixirDataGraphExecutor::new(store_resource.store.clone(), task_executor, callback_pid);

    let orch_executor = node_engine::OrchestrationExecutor::new(data_executor)
        .with_execution_id(format!("nif-orch-{}", graph_id));

    let result = runtime.block_on(async {
        orch_executor
            .execute(&graph, initial_data, &event_sink)
            .await
    });

    serialize_orchestration_result(result)
}

pub(crate) fn execute_with_inference(
    store_resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
    initial_data_json: String,
    callback_pid: rustler::LocalPid,
    gateway_resource: ResourceArc<InferenceGatewayResource>,
) -> NifResult<String> {
    let initial_data = parse_initial_data(initial_data_json)?;
    let graph = get_orchestration_graph(&store_resource, &graph_id)?;
    let runtime = create_runtime()?;

    let event_sink: Arc<dyn EventSink> = Arc::new(BeamEventSink::new(callback_pid));
    let core = node_engine::CoreTaskExecutor::new()
        .with_gateway(gateway_resource.gateway.clone())
        .with_event_sink(event_sink.clone())
        .with_execution_id(format!("nif-orch-{}", graph_id));
    let elixir = ElixirCallbackTaskExecutor::new(callback_pid);
    let task_executor: Arc<dyn TaskExecutor> = Arc::new(CoreFirstExecutor::new(core, elixir));

    let data_executor =
        ElixirDataGraphExecutor::new(store_resource.store.clone(), task_executor, callback_pid);

    let orch_executor = node_engine::OrchestrationExecutor::new(data_executor)
        .with_execution_id(format!("nif-orch-{}", graph_id));

    let result = runtime.block_on(async {
        orch_executor
            .execute(&graph, initial_data, event_sink.as_ref())
            .await
    });

    serialize_orchestration_result(result)
}

pub(crate) fn insert_data_graph(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
    graph_json: String,
) -> NifResult<Atom> {
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let mut guard = resource.store.blocking_write();
    guard.insert_data_graph(graph_id, graph);

    Ok(atoms::ok())
}

fn parse_initial_data(initial_data_json: String) -> NifResult<HashMap<String, serde_json::Value>> {
    serde_json::from_str(&initial_data_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))
}

fn get_orchestration_graph(
    store_resource: &ResourceArc<OrchestrationStoreResource>,
    graph_id: &str,
) -> NifResult<node_engine::OrchestrationGraph> {
    let store = store_resource.store.blocking_read();
    store.get_graph(graph_id).cloned().ok_or_else(|| {
        rustler::Error::Term(Box::new(format!(
            "Orchestration graph '{}' not found",
            graph_id
        )))
    })
}

fn create_runtime() -> NifResult<tokio::runtime::Runtime> {
    tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))
}

fn serialize_orchestration_result(
    result: node_engine::Result<node_engine::OrchestrationResult>,
) -> NifResult<String> {
    match result {
        Ok(orch_result) => serde_json::to_string(&orch_result)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e)))),
        Err(e) => Err(rustler::Error::Term(Box::new(format!(
            "Orchestration error: {}",
            e
        )))),
    }
}
