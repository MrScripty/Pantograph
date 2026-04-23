use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use node_engine::EventSink;
use node_engine::{
    Context, ExecutorExtensions, GraphNode as EngineGraphNode, TaskExecutor, WorkflowExecutor,
};
use pantograph_workflow_service::{
    graph::WorkflowDerivedGraph, GraphNode, Position, WorkflowGraph,
};
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};

use super::diagnostics_bridge::translate_node_event_with_diagnostics;
use super::translation::translated_execution_id;
use super::{TauriEventAdapter, TauriWorkflowEvent};
use crate::workflow::WorkflowDiagnosticsStore;

fn sample_parallel_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "left".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            },
            GraphNode {
                id: "right".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 100.0, y: 0.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: Vec::new(),
        derived_graph: Some(WorkflowDerivedGraph {
            schema_version: 1,
            graph_fingerprint: "graph-parallel".to_string(),
            consumer_count_map: HashMap::new(),
        }),
    }
}

fn make_parallel_roots_graph() -> node_engine::WorkflowGraph {
    let mut graph = node_engine::WorkflowGraph::new("parallel", "Parallel");
    for (id, x) in [("left", 0.0), ("right", 100.0)] {
        graph.nodes.push(EngineGraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (x, 0.0),
        });
    }
    graph
}

struct YieldingAcceptanceExecutor {
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingAcceptanceExecutor {
    fn new() -> Self {
        Self {
            current_in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }

    fn record_max_in_flight(&self, observed: usize) {
        let mut current_max = self.max_in_flight.load(Ordering::SeqCst);
        while observed > current_max {
            match self.max_in_flight.compare_exchange(
                current_max,
                observed,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }
}

#[async_trait]
impl TaskExecutor for YieldingAcceptanceExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        _inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);
        tokio::task::yield_now().await;
        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({ "task": task_id }),
        )]))
    }
}

#[path = "tests/channel_transport.rs"]
mod channel_transport;
#[path = "tests/executor_integration.rs"]
mod executor_integration;
#[path = "tests/translation_projection.rs"]
mod translation_projection;
