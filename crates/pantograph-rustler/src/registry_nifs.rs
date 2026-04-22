use std::sync::Arc;

use rustler::{Atom, NifResult, ResourceArc};

use crate::atoms;
use crate::resources::{ExtensionsResource, NodeRegistryResource};

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

    serde_json::to_string(&metadata)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

pub(crate) fn node_registry_register_builtins(
    resource: ResourceArc<NodeRegistryResource>,
) -> NifResult<Atom> {
    let mut registry = resource.registry.blocking_write();
    registry.register_builtins();
    Ok(atoms::ok())
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
