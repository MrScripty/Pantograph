use std::collections::HashSet;

use crate::events::WorkflowEvent;
use crate::types::{NodeId, WorkflowGraph};

pub(super) fn collect_dirty_tasks(graph: &WorkflowGraph, root_node_id: &NodeId) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut stack = vec![root_node_id.clone()];

    while let Some(node_id) = stack.pop() {
        if !visited.insert(node_id.clone()) {
            continue;
        }

        for dependent in graph.get_dependents(&node_id) {
            stack.push(dependent);
        }
    }

    let mut dirty_tasks = visited.into_iter().collect::<Vec<_>>();
    dirty_tasks.sort();
    dirty_tasks
}

pub(super) fn snapshot_dirty_tasks(graph: &WorkflowGraph) -> Vec<NodeId> {
    graph.nodes.iter().map(|node| node.id.clone()).collect()
}

pub(super) fn graph_modified_event(
    workflow_id: String,
    execution_id: &str,
    dirty_tasks: Vec<NodeId>,
) -> Option<WorkflowEvent> {
    if dirty_tasks.is_empty() {
        return None;
    }

    Some(WorkflowEvent::GraphModified {
        workflow_id,
        execution_id: execution_id.to_string(),
        dirty_tasks,
        occurred_at_ms: Some(crate::events::unix_timestamp_ms()),
    })
}

pub(super) fn incremental_execution_started_event(
    workflow_id: String,
    execution_id: &str,
    task_ids: Vec<NodeId>,
) -> Option<WorkflowEvent> {
    if task_ids.is_empty() {
        return None;
    }

    Some(WorkflowEvent::IncrementalExecutionStarted {
        workflow_id,
        execution_id: execution_id.to_string(),
        tasks: task_ids,
        occurred_at_ms: Some(crate::events::unix_timestamp_ms()),
    })
}
