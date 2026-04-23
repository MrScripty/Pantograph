use std::path::PathBuf;
use std::sync::Arc;

use node_engine::{EventSink, TaskExecutor, WorkflowExecutor, WorkflowGraph};
use rustler::{Atom, Encoder, NifResult, OwnedEnv, ResourceArc};

use crate::atoms;
use crate::binding_types::ElixirCacheStats;
use crate::callback_bridge::{BeamEventSink, CoreFirstExecutor, ElixirCallbackTaskExecutor};
use crate::resources::{InferenceGatewayResource, WorkflowExecutorResource};

pub(crate) fn new_executor(
    graph_json: String,
    caller_pid: rustler::LocalPid,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    create_executor_resource(graph_json, caller_pid, None, None)
}

pub(crate) fn new_executor_with_timeout(
    graph_json: String,
    caller_pid: rustler::LocalPid,
    timeout_secs: u64,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    create_executor_resource(graph_json, caller_pid, Some(timeout_secs), None)
}

pub(crate) fn new_inference_gateway(
    binaries_dir: String,
    data_dir: String,
) -> NifResult<ResourceArc<InferenceGatewayResource>> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    let gateway = Arc::new(inference::InferenceGateway::new());
    let spawner = Arc::new(inference::StdProcessSpawner::new(
        PathBuf::from(binaries_dir),
        PathBuf::from(data_dir),
    ));
    runtime.block_on(async { gateway.set_spawner(spawner).await });

    Ok(ResourceArc::new(InferenceGatewayResource {
        gateway,
        runtime: Arc::new(runtime),
    }))
}

pub(crate) fn new_executor_with_inference(
    graph_json: String,
    caller_pid: rustler::LocalPid,
    gateway_resource: ResourceArc<InferenceGatewayResource>,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    create_executor_resource(graph_json, caller_pid, None, Some(gateway_resource))
}

pub(crate) fn new_executor_with_inference_timeout(
    graph_json: String,
    caller_pid: rustler::LocalPid,
    gateway_resource: ResourceArc<InferenceGatewayResource>,
    timeout_secs: u64,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    create_executor_resource(
        graph_json,
        caller_pid,
        Some(timeout_secs),
        Some(gateway_resource),
    )
}

fn create_executor_resource(
    graph_json: String,
    caller_pid: rustler::LocalPid,
    timeout_secs: Option<u64>,
    gateway_resource: Option<ResourceArc<InferenceGatewayResource>>,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    let event_sink: Arc<dyn EventSink> = Arc::new(BeamEventSink::new(caller_pid));
    let core = match gateway_resource {
        Some(gateway_resource) => node_engine::CoreTaskExecutor::new()
            .with_gateway(gateway_resource.gateway.clone())
            .with_event_sink(event_sink.clone())
            .with_execution_id("nif-execution".to_string()),
        None => node_engine::CoreTaskExecutor::new(),
    };
    let elixir = match timeout_secs {
        Some(timeout_secs) => {
            ElixirCallbackTaskExecutor::new(caller_pid).with_timeout(timeout_secs)
        }
        None => ElixirCallbackTaskExecutor::new(caller_pid),
    };
    let task_executor: Arc<dyn TaskExecutor> = Arc::new(CoreFirstExecutor::new(core, elixir));

    let executor = WorkflowExecutor::new("nif-execution", graph, event_sink);

    Ok(ResourceArc::new(WorkflowExecutorResource {
        executor: Arc::new(tokio::sync::RwLock::new(executor)),
        task_executor,
        runtime: Arc::new(runtime),
    }))
}

pub(crate) fn demand(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
) -> NifResult<String> {
    let rt = &resource.runtime;
    let executor = &resource.executor;
    let task_exec = &resource.task_executor;

    rt.block_on(async {
        let exec = executor.read().await;
        let result = exec
            .demand(&node_id, task_exec.as_ref())
            .await
            .map_err(|e| rustler::Error::Term(Box::new(format!("Demand error: {}", e))))?;
        serde_json::to_string(&result)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
    })
}

pub(crate) fn demand_async(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    caller_pid: rustler::LocalPid,
) -> Atom {
    let executor = resource.executor.clone();
    let task_exec = resource.task_executor.clone();
    let nid = node_id.clone();

    resource.runtime.spawn(async move {
        let exec = executor.read().await;
        let result = exec.demand(&nid, task_exec.as_ref()).await;

        let mut owned_env = OwnedEnv::new();
        match result {
            Ok(outputs) => {
                let json = serde_json::to_string(&outputs)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialization: {}\"}}", e));
                let _ = owned_env.send_and_clear(&caller_pid, |env| {
                    (
                        atoms::demand_complete().encode(env),
                        nid.encode(env),
                        json.encode(env),
                    )
                        .encode(env)
                });
            }
            Err(e) => {
                let _ = owned_env.send_and_clear(&caller_pid, |env| {
                    (
                        atoms::demand_error().encode(env),
                        nid.encode(env),
                        e.to_string().encode(env),
                    )
                        .encode(env)
                });
            }
        }
    });

    atoms::ok()
}

pub(crate) fn update_node_data(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    data_json: String,
) -> NifResult<Atom> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    let data: serde_json::Value = serde_json::from_str(&data_json).unwrap_or_default();

    rt.block_on(async {
        let exec = executor.read().await;
        exec.update_node_data(&node_id, data)
            .await
            .map_err(|e| rustler::Error::Term(Box::new(format!("Update error: {}", e))))?;
        Ok(atoms::ok())
    })
}

pub(crate) fn mark_modified(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
) -> NifResult<Atom> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    rt.block_on(async {
        let exec = executor.read().await;
        exec.mark_modified(&node_id).await;
        Ok(atoms::ok())
    })
}

pub(crate) fn cache_stats(
    resource: ResourceArc<WorkflowExecutorResource>,
) -> NifResult<ElixirCacheStats> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    rt.block_on(async {
        let exec = executor.read().await;
        let stats = exec.cache_stats().await;
        Ok(ElixirCacheStats {
            cached_nodes: stats.cached_nodes as u32,
            total_versions: stats.total_versions as u32,
            global_version: stats.global_version,
        })
    })
}

pub(crate) fn get_graph_snapshot(
    resource: ResourceArc<WorkflowExecutorResource>,
) -> NifResult<String> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    rt.block_on(async {
        let exec = executor.read().await;
        let graph = exec.get_graph_snapshot().await;
        serde_json::to_string(&graph)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
    })
}

pub(crate) fn set_input(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    port: String,
    value_json: String,
) -> NifResult<Atom> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    let value: serde_json::Value = serde_json::from_str(&value_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let key = node_engine::ContextKeys::input(&node_id, &port);

    rt.block_on(async {
        let exec = executor.read().await;
        exec.set_context_value(&key, value).await;
        Ok(atoms::ok())
    })
}

pub(crate) fn get_output(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    port: String,
) -> NifResult<Option<String>> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    let key = node_engine::ContextKeys::output(&node_id, &port);

    rt.block_on(async {
        let exec = executor.read().await;
        let value: Option<serde_json::Value> = exec.get_context_value(&key).await;
        match value {
            Some(v) => {
                let json = serde_json::to_string(&v).map_err(|e| {
                    rustler::Error::Term(Box::new(format!("Serialization error: {}", e)))
                })?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    })
}
