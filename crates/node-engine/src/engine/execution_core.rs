use std::collections::{HashMap, HashSet};

use crate::error::NodeEngineError;
use crate::types::NodeId;

pub(super) struct DemandExecutionCore<'a> {
    engine: &'a mut super::DemandEngine,
    runtime: super::DemandRuntimeContext<'a>,
    computing: &'a mut HashSet<NodeId>,
}

impl<'a> DemandExecutionCore<'a> {
    pub(super) fn new(
        engine: &'a mut super::DemandEngine,
        runtime: super::DemandRuntimeContext<'a>,
        computing: &'a mut HashSet<NodeId>,
    ) -> Self {
        Self {
            engine,
            runtime,
            computing,
        }
    }

    pub(super) fn run_node<'b>(
        &'b mut self,
        node_id: &'b NodeId,
    ) -> super::DemandFuture<'b, super::NodeOutputMap> {
        Box::pin(async move {
            super::inflight_tracking::begin_node_compute(self.computing, node_id)?;
            let result = async {
                let dependency_outputs = self.collect_dependency_outputs(node_id).await?;
                let mut inputs = super::dependency_inputs::resolve_dependency_inputs(
                    self.runtime.graph,
                    node_id,
                    &dependency_outputs,
                );
                let input_version = self
                    .engine
                    .compute_input_version(node_id, self.runtime.graph);

                if let Some(outputs) = super::output_cache::resolve_fresh_cached_output(
                    &self.engine.cache,
                    node_id,
                    input_version,
                )? {
                    return Ok(outputs);
                }

                if let Some(snapshot) = self
                    .runtime
                    .node_memories
                    .and_then(|node_memories| node_memories.get(node_id))
                {
                    super::workflow_session::inject_node_memory_input(&mut inputs, snapshot)?;
                }

                if let Some(prompt) = super::node_preparation::prepare_node_inputs(
                    self.runtime.graph,
                    node_id,
                    &mut inputs,
                ) {
                    super::execution_events::emit_task_started(
                        self.runtime.event_sink,
                        node_id.clone(),
                        self.engine.execution_id.clone(),
                    );
                    super::execution_events::emit_waiting_for_input(
                        self.runtime.event_sink,
                        self.runtime.graph.id.clone(),
                        self.engine.execution_id.clone(),
                        node_id.clone(),
                        prompt.clone(),
                    );
                    return Err(NodeEngineError::waiting_for_input(node_id.clone(), prompt));
                }

                super::execution_events::emit_task_started(
                    self.runtime.event_sink,
                    node_id.clone(),
                    self.engine.execution_id.clone(),
                );

                let task_inputs = inputs.clone();
                let outputs = match self
                    .runtime
                    .executor
                    .execute_task(
                        node_id,
                        inputs,
                        self.runtime.context,
                        self.runtime.extensions,
                    )
                    .await
                {
                    Ok(outputs) => outputs,
                    Err(NodeEngineError::WaitingForInput { task_id, prompt }) => {
                        super::execution_events::emit_waiting_for_input(
                            self.runtime.event_sink,
                            self.runtime.graph.id.clone(),
                            self.engine.execution_id.clone(),
                            task_id.clone(),
                            prompt.clone(),
                        );
                        return Err(NodeEngineError::WaitingForInput { task_id, prompt });
                    }
                    Err(error) => return Err(error),
                };

                super::execution_events::emit_task_completed(
                    self.runtime.event_sink,
                    node_id.clone(),
                    self.engine.execution_id.clone(),
                    &outputs,
                )?;

                super::output_cache::store_completed_output(
                    &mut self.engine.cache,
                    &mut self.engine.versions,
                    &mut self.engine.global_version,
                    node_id,
                    input_version,
                    &outputs,
                )?;
                self.engine.record_input_snapshot(node_id, task_inputs);

                Ok(outputs)
            }
            .await;

            super::inflight_tracking::finish_node_compute(self.computing, node_id);
            result
        })
    }

    fn collect_dependency_outputs<'b>(
        &'b mut self,
        node_id: &'b NodeId,
    ) -> super::DemandFuture<'b, super::MultiNodeOutputMap> {
        Box::pin(async move {
            let mut dependency_outputs = HashMap::new();

            for dep_id in self.runtime.graph.get_dependencies(node_id) {
                let dep_outputs = self.run_node(&dep_id).await?;
                dependency_outputs.insert(dep_id, dep_outputs);
            }

            Ok(dependency_outputs)
        })
    }
}
