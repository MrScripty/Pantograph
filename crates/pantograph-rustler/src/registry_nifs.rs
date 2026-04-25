use std::sync::Arc;

use pantograph_workflow_service::NodeRegistry as WorkflowNodeRegistry;
use rustler::{Atom, NifResult, ResourceArc};
use serde::Serialize;

use crate::atoms;
use crate::resources::{ExtensionsResource, NodeRegistryResource};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct QueryablePortProjection<'a> {
    node_type: &'a str,
    port_id: &'a str,
}

pub(crate) fn node_registry_new() -> ResourceArc<NodeRegistryResource> {
    ResourceArc::new(NodeRegistryResource {
        registry: Arc::new(tokio::sync::RwLock::new(node_engine::NodeRegistry::new())),
    })
}

pub(crate) fn node_registry_register(
    resource: ResourceArc<NodeRegistryResource>,
    metadata_json: String,
) -> NifResult<Atom> {
    let metadata: node_engine::TaskMetadata = serde_json::from_str(&metadata_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let mut registry = resource.registry.blocking_write();
    registry.register_metadata(metadata);

    Ok(atoms::ok())
}

pub(crate) fn node_registry_list(resource: ResourceArc<NodeRegistryResource>) -> NifResult<String> {
    let registry = resource.registry.blocking_read();
    let metadata: Vec<&node_engine::TaskMetadata> = registry.all_metadata();

    serialize_json(&metadata)
}

pub(crate) fn node_registry_list_definitions() -> NifResult<String> {
    let registry = WorkflowNodeRegistry::new();
    serialize_json(&registry.all_definitions())
}

pub(crate) fn node_registry_get_definition(node_type: String) -> NifResult<String> {
    let registry = WorkflowNodeRegistry::new();
    let definition = registry.get_definition(&node_type).ok_or_else(|| {
        rustler::Error::Term(Box::new(format!("unknown node_type '{}'", node_type)))
    })?;
    serialize_json(definition)
}

pub(crate) fn node_registry_definitions_by_category() -> NifResult<String> {
    let registry = WorkflowNodeRegistry::new();
    serialize_json(&registry.definitions_by_category())
}

pub(crate) fn node_registry_register_builtins(
    resource: ResourceArc<NodeRegistryResource>,
) -> NifResult<Atom> {
    let mut registry = resource.registry.blocking_write();
    registry.register_builtins();
    Ok(atoms::ok())
}

pub(crate) fn node_registry_queryable_ports(
    resource: ResourceArc<NodeRegistryResource>,
) -> NifResult<String> {
    let registry = resource.registry.blocking_read();
    let ports = registry
        .queryable_ports()
        .into_iter()
        .map(|(node_type, port_id)| QueryablePortProjection { node_type, port_id })
        .collect::<Vec<_>>();
    serialize_json(&ports)
}

pub(crate) fn extensions_new() -> ResourceArc<ExtensionsResource> {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    ResourceArc::new(ExtensionsResource {
        extensions: Arc::new(tokio::sync::RwLock::new(
            node_engine::ExecutorExtensions::new(),
        )),
        runtime: Arc::new(runtime),
    })
}

pub(crate) fn extensions_setup(
    resource: ResourceArc<ExtensionsResource>,
    library_path: Option<String>,
) -> NifResult<Atom> {
    let path_buf = library_path.map(std::path::PathBuf::from);
    let path_ref = path_buf.as_deref();

    resource.runtime.block_on(async {
        let mut ext = resource.extensions.write().await;
        workflow_nodes::setup_extensions_with_path(&mut ext, path_ref).await;
    });

    Ok(atoms::ok())
}

pub(crate) fn node_registry_query_port_options(
    registry_resource: ResourceArc<NodeRegistryResource>,
    extensions_resource: ResourceArc<ExtensionsResource>,
    node_type: String,
    port_id: String,
    query_json: String,
) -> NifResult<String> {
    let query: node_engine::PortOptionsQuery = serde_json::from_str(&query_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("JSON parse error: {}", e))))?;

    extensions_resource
        .runtime
        .block_on(async {
            let registry = registry_resource.registry.read().await;
            let ext = extensions_resource.extensions.read().await;
            registry
                .query_port_options(&node_type, &port_id, &query, &ext)
                .await
        })
        .map_err(|e| rustler::Error::Term(Box::new(format!("query_port_options error: {}", e))))
        .and_then(|result| {
            serde_json::to_string(&result)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
        })
}

fn serialize_json<T>(value: &T) -> NifResult<String>
where
    T: Serialize,
{
    serde_json::to_string(value)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}
