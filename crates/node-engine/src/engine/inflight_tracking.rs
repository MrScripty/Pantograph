use std::collections::HashSet;

use crate::error::{NodeEngineError, Result};
use crate::types::NodeId;

pub(super) fn begin_node_compute(computing: &mut HashSet<NodeId>, node_id: &NodeId) -> Result<()> {
    if computing.contains(node_id) {
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Cycle detected: node '{}' is already being computed",
            node_id
        )));
    }

    computing.insert(node_id.clone());
    Ok(())
}

pub(super) fn finish_node_compute(computing: &mut HashSet<NodeId>, node_id: &NodeId) {
    computing.remove(node_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_node_compute_inserts_node_id() {
        let mut computing = HashSet::new();

        begin_node_compute(&mut computing, &"node-a".to_string()).expect("begin compute");

        assert!(computing.contains("node-a"));
    }

    #[test]
    fn begin_node_compute_rejects_cycles() {
        let mut computing = HashSet::from(["node-a".to_string()]);

        let error = begin_node_compute(&mut computing, &"node-a".to_string())
            .expect_err("existing in-flight node should fail");

        assert!(matches!(error, NodeEngineError::ExecutionFailed(message)
            if message.contains("Cycle detected")));
    }

    #[test]
    fn finish_node_compute_removes_node_id() {
        let mut computing = HashSet::from(["node-a".to_string(), "node-b".to_string()]);

        finish_node_compute(&mut computing, &"node-a".to_string());

        assert!(!computing.contains("node-a"));
        assert!(computing.contains("node-b"));
    }
}
