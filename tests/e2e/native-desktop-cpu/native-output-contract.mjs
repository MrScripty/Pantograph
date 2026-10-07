// Default GUI submission exports the vector sink; inference ports stay internal.
export function defaultVectorArtifact(artifacts, runId) {
  return artifacts.find((item) => item.workflow_run_id === runId
    && item.artifact_role === 'workflow_output'
    && item.producer_node_id === 'vectors' && item.producer_port_id === 'vector'
    && item.retention_state === 'retained' && item.lifecycle_state === 'retained');
}

export function completedCpuAttempt(events, runId) {
  return events.find((event) => event.workflow_run_id === runId
    && event.event_kind === 'scheduler_task_attempt_lifecycle_changed'
    && event.scheduler_task_id === 'infer'
    && event.scheduler_attempt_execution_class === 'runtime'
    && event.scheduler_attempt_transition === 'completed'
    && event.scheduler_attempt_runtime_id === 'candle'
    && event.scheduler_attempt_runtime_variant_id === 'candle.cpu'
    && event.scheduler_attempt_device_id === 'cpu');
}
