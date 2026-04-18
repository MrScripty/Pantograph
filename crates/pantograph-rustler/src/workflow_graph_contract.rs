use node_engine::{GraphEdge, GraphNode, WorkflowGraph};
use rustler::{Error, NifResult};

fn parse_error(message: impl Into<String>) -> Error {
    Error::Term(Box::new(format!("Parse error: {}", message.into())))
}

fn serialization_error(message: impl Into<String>) -> Error {
    Error::Term(Box::new(format!("Serialization error: {}", message.into())))
}

fn parse_graph(graph_json: &str) -> NifResult<WorkflowGraph> {
    serde_json::from_str(graph_json).map_err(|error| parse_error(error.to_string()))
}

fn serialize_graph(graph: &WorkflowGraph) -> NifResult<String> {
    serde_json::to_string(graph).map_err(|error| serialization_error(error.to_string()))
}

fn parse_node_data(data_json: &str) -> serde_json::Value {
    serde_json::from_str(data_json).unwrap_or_default()
}

pub(crate) fn workflow_new_json(id: String, name: String) -> NifResult<String> {
    serialize_graph(&WorkflowGraph::new(&id, &name))
}

pub(crate) fn workflow_from_json_string(graph_json: String) -> NifResult<String> {
    let graph = parse_graph(&graph_json)?;
    serialize_graph(&graph)
}

pub(crate) fn workflow_add_node_json(
    graph_json: String,
    node_id: String,
    node_type: String,
    x: f64,
    y: f64,
    data_json: String,
) -> NifResult<String> {
    let mut graph = parse_graph(&graph_json)?;
    graph.nodes.push(GraphNode {
        id: node_id,
        node_type,
        position: (x, y),
        data: parse_node_data(&data_json),
    });

    serialize_graph(&graph)
}

pub(crate) fn workflow_remove_node_json(graph_json: String, node_id: String) -> NifResult<String> {
    let mut graph = parse_graph(&graph_json)?;
    graph.nodes.retain(|node| node.id != node_id);
    graph
        .edges
        .retain(|edge| edge.source != node_id && edge.target != node_id);

    serialize_graph(&graph)
}

pub(crate) fn workflow_add_edge_json(
    graph_json: String,
    source: String,
    source_handle: String,
    target: String,
    target_handle: String,
) -> NifResult<String> {
    let mut graph = parse_graph(&graph_json)?;
    graph.edges.push(GraphEdge {
        id: format!(
            "e-{}-{}-{}-{}",
            source, source_handle, target, target_handle
        ),
        source,
        source_handle,
        target,
        target_handle,
    });

    serialize_graph(&graph)
}

pub(crate) fn workflow_remove_edge_json(graph_json: String, edge_id: String) -> NifResult<String> {
    let mut graph = parse_graph(&graph_json)?;
    graph.edges.retain(|edge| edge.id != edge_id);

    serialize_graph(&graph)
}

pub(crate) fn workflow_update_node_data_json(
    graph_json: String,
    node_id: String,
    data_json: String,
) -> NifResult<String> {
    let mut graph = parse_graph(&graph_json)?;

    if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == node_id) {
        node.data = parse_node_data(&data_json);
    }

    serialize_graph(&graph)
}

pub(crate) fn workflow_validate_json(graph_json: String) -> NifResult<Vec<String>> {
    let graph = parse_graph(&graph_json)?;
    let errors = node_engine::validation::validate_workflow(&graph, None);

    Ok(errors.iter().map(|error| error.to_string()).collect())
}
