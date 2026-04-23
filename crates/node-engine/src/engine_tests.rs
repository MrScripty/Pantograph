use super::*;
use crate::error::NodeEngineError;
use crate::events::{NullEventSink, VecEventSink};
use crate::types::{GraphEdge, GraphNode};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn make_linear_graph() -> WorkflowGraph {
    let mut graph = WorkflowGraph::new("test", "Test");
    graph.nodes.push(GraphNode {
        id: "a".to_string(),
        node_type: "input".to_string(),
        data: serde_json::Value::Null,
        position: (0.0, 0.0),
    });
    graph.nodes.push(GraphNode {
        id: "b".to_string(),
        node_type: "process".to_string(),
        data: serde_json::Value::Null,
        position: (100.0, 0.0),
    });
    graph.nodes.push(GraphNode {
        id: "c".to_string(),
        node_type: "output".to_string(),
        data: serde_json::Value::Null,
        position: (200.0, 0.0),
    });
    graph.edges.push(GraphEdge {
        id: "e1".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "b".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e2".to_string(),
        source: "b".to_string(),
        source_handle: "out".to_string(),
        target: "c".to_string(),
        target_handle: "in".to_string(),
    });
    graph
}

fn make_diamond_graph() -> WorkflowGraph {
    // Diamond pattern: a -> b, a -> c, b -> d, c -> d
    let mut graph = WorkflowGraph::new("diamond", "Diamond");
    for (id, pos) in [("a", 0.0), ("b", 100.0), ("c", 100.0), ("d", 200.0)] {
        graph.nodes.push(GraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (pos, 0.0),
        });
    }
    graph.edges.push(GraphEdge {
        id: "e1".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "b".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e2".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "c".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e3".to_string(),
        source: "b".to_string(),
        source_handle: "out".to_string(),
        target: "d".to_string(),
        target_handle: "in_b".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e4".to_string(),
        source: "c".to_string(),
        source_handle: "out".to_string(),
        target: "d".to_string(),
        target_handle: "in_c".to_string(),
    });
    graph
}

fn make_shared_dependency_graph() -> WorkflowGraph {
    let mut graph = WorkflowGraph::new("shared", "Shared");
    for (id, x, y) in [("a", 0.0, 0.0), ("b", 100.0, 0.0), ("c", 100.0, 100.0)] {
        graph.nodes.push(GraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (x, y),
        });
    }
    graph.edges.push(GraphEdge {
        id: "e1".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "b".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e2".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "c".to_string(),
        target_handle: "in".to_string(),
    });
    graph
}

fn make_parallel_roots_graph() -> WorkflowGraph {
    let mut graph = WorkflowGraph::new("parallel", "Parallel");
    for (id, x) in [("left", 0.0), ("right", 100.0)] {
        graph.nodes.push(GraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (x, 0.0),
        });
    }
    graph
}

/// A simple test executor that counts invocations
struct CountingExecutor {
    execution_count: AtomicUsize,
}

impl CountingExecutor {
    fn new() -> Self {
        Self {
            execution_count: AtomicUsize::new(0),
        }
    }

    fn count(&self) -> usize {
        self.execution_count.load(Ordering::SeqCst)
    }
}

struct YieldingExecutor {
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingExecutor {
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

struct YieldingFailingExecutor {
    fail_on: String,
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingFailingExecutor {
    fn new(fail_on: impl Into<String>) -> Self {
        Self {
            fail_on: fail_on.into(),
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

struct YieldingWaitingExecutor {
    wait_on: String,
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingWaitingExecutor {
    fn new(wait_on: impl Into<String>) -> Self {
        Self {
            wait_on: wait_on.into(),
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

struct FailingExecutor {
    fail_on: String,
    execution_log: Mutex<Vec<String>>,
}

impl FailingExecutor {
    fn new(fail_on: impl Into<String>) -> Self {
        Self {
            fail_on: fail_on.into(),
            execution_log: Mutex::new(Vec::new()),
        }
    }

    fn executed_tasks(&self) -> Vec<String> {
        self.execution_log.lock().expect("execution log").clone()
    }
}

struct WaitingExecutor {
    wait_on: String,
    execution_log: Mutex<Vec<String>>,
}

impl WaitingExecutor {
    fn new(wait_on: impl Into<String>) -> Self {
        Self {
            wait_on: wait_on.into(),
            execution_log: Mutex::new(Vec::new()),
        }
    }

    fn executed_tasks(&self) -> Vec<String> {
        self.execution_log.lock().expect("execution log").clone()
    }
}

#[async_trait]
impl TaskExecutor for CountingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.execution_count.fetch_add(1, Ordering::SeqCst);

        // Simple passthrough: combine all inputs into output
        let mut outputs = HashMap::new();
        outputs.insert(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        );
        Ok(outputs)
    }
}

#[async_trait]
impl TaskExecutor for YieldingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);

        tokio::task::yield_now().await;

        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for YieldingFailingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);

        tokio::task::yield_now().await;

        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        if task_id == self.fail_on {
            return Err(NodeEngineError::failed(format!(
                "forced failure at {task_id}"
            )));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for YieldingWaitingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);

        tokio::task::yield_now().await;

        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        if task_id == self.wait_on {
            return Err(NodeEngineError::waiting_for_input(
                task_id.to_string(),
                Some(format!("waiting at {task_id}")),
            ));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for FailingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.execution_log
            .lock()
            .expect("execution log")
            .push(task_id.to_string());

        if task_id == self.fail_on {
            return Err(NodeEngineError::failed(format!(
                "forced failure at {task_id}"
            )));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for WaitingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.execution_log
            .lock()
            .expect("execution log")
            .push(task_id.to_string());

        if task_id == self.wait_on {
            return Err(NodeEngineError::waiting_for_input(
                task_id.to_string(),
                Some(format!("waiting at {task_id}")),
            ));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[path = "engine_tests/cache_state.rs"]
mod cache_state;
#[path = "engine_tests/demand.rs"]
mod demand;
#[path = "engine_tests/human_input.rs"]
mod human_input;
#[path = "engine_tests/multi_demand.rs"]
mod multi_demand;
#[path = "engine_tests/snapshot.rs"]
mod snapshot;
#[path = "engine_tests/workflow_events.rs"]
mod workflow_events;
