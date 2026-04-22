use std::sync::Arc;

use node_engine::{OrchestrationGraph, OrchestrationStore};
use rustler::{Atom, NifResult, ResourceArc};

use crate::atoms;
use crate::binding_types::ElixirOrchestrationMetadata;
use crate::resources::OrchestrationStoreResource;

pub(crate) fn new_store() -> ResourceArc<OrchestrationStoreResource> {
    ResourceArc::new(OrchestrationStoreResource {
        store: Arc::new(tokio::sync::RwLock::new(OrchestrationStore::new())),
    })
}

pub(crate) fn with_persistence(path: String) -> ResourceArc<OrchestrationStoreResource> {
    ResourceArc::new(OrchestrationStoreResource {
        store: Arc::new(tokio::sync::RwLock::new(
            OrchestrationStore::with_persistence(path),
        )),
    })
}

pub(crate) fn insert(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_json: String,
) -> NifResult<Atom> {
    let graph: OrchestrationGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let mut guard = resource.store.blocking_write();
    guard
        .insert_graph(graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Insert error: {}", e))))?;

    Ok(atoms::ok())
}

pub(crate) fn get(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
) -> NifResult<Option<String>> {
    let guard = resource.store.blocking_read();
    match guard.get_graph(&graph_id) {
        Some(graph) => {
            let json = serde_json::to_string(graph).map_err(|e| {
                rustler::Error::Term(Box::new(format!("Serialization error: {}", e)))
            })?;
            Ok(Some(json))
        }
        None => Ok(None),
    }
}

pub(crate) fn list(
    resource: ResourceArc<OrchestrationStoreResource>,
) -> Vec<ElixirOrchestrationMetadata> {
    let guard = resource.store.blocking_read();
    guard
        .list_graphs()
        .into_iter()
        .map(|m| ElixirOrchestrationMetadata {
            id: m.id,
            name: m.name,
            description: m.description,
            node_count: m.node_count as u32,
        })
        .collect()
}

pub(crate) fn remove(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
) -> NifResult<bool> {
    let mut guard = resource.store.blocking_write();
    guard
        .remove_graph(&graph_id)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Remove error: {}", e))))?;
    Ok(true)
}
