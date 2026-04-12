import test from 'node:test';
import assert from 'node:assert/strict';

import { DiagnosticsService } from './DiagnosticsService.ts';

test('updateWorkflowMetadata preserves sibling fields across partial updates', () => {
  const service = new DiagnosticsService();

  service.updateWorkflowMetadata({
    workflowId: 'wf-1',
    workflowName: 'Workflow One',
  });
  service.updateWorkflowMetadata({
    workflowId: 'wf-2',
  });

  const afterIdUpdate = service.getSnapshot();
  assert.equal(afterIdUpdate.state.currentWorkflowId, 'wf-2');
  assert.equal(afterIdUpdate.state.currentWorkflowName, 'Workflow One');

  service.updateWorkflowMetadata({
    workflowName: 'Workflow Two',
  });

  const afterNameUpdate = service.getSnapshot();
  assert.equal(afterNameUpdate.state.currentWorkflowId, 'wf-2');
  assert.equal(afterNameUpdate.state.currentWorkflowName, 'Workflow Two');
});

test('updateWorkflowMetadata allows explicit clearing', () => {
  const service = new DiagnosticsService();

  service.updateWorkflowMetadata({
    workflowId: 'wf-3',
    workflowName: 'Workflow Three',
  });
  service.updateWorkflowMetadata({
    workflowId: null,
    workflowName: null,
  });

  const snapshot = service.getSnapshot();
  assert.equal(snapshot.state.currentWorkflowId, null);
  assert.equal(snapshot.state.currentWorkflowName, null);
});

test('runtime and scheduler snapshots retain workflow and session diagnostics', () => {
  const service = new DiagnosticsService();

  service.setCurrentSessionId('session-1');
  service.updateRuntimeSnapshot('wf-runtime', {
    max_input_bindings: 4,
    max_output_targets: 2,
    max_value_bytes: 1000,
    runtime_requirements: {
      estimation_confidence: 'high',
      required_models: ['model-a'],
      required_backends: ['llama-cpp'],
      required_extensions: ['kv-cache'],
    },
    models: [{
      model_id: 'model-a',
      node_ids: ['node-a'],
      roles: ['generation'],
    }],
    runtime_capabilities: [{
      runtime_id: 'python-sidecar',
      display_name: 'Python Sidecar',
      install_state: 'installed',
      available: true,
      configured: true,
      can_install: false,
      can_remove: false,
      backend_keys: ['diffusers'],
      missing_files: [],
    }],
  }, null, 5_000);
  service.updateSchedulerSnapshot('wf-runtime', 'session-1', {
    session: {
      session_id: 'session-1',
      workflow_id: 'wf-runtime',
      keep_alive: true,
      state: 'running',
      queued_runs: 1,
      run_count: 3,
    },
  }, {
    session_id: 'session-1',
    items: [{
      queue_id: 'queue-1',
      run_id: 'run-1',
      priority: 10,
      status: 'running',
    }],
  }, null, 6_000);

  const snapshot = service.getSnapshot();
  assert.equal(snapshot.state.currentSessionId, 'session-1');
  assert.equal(snapshot.state.runtime.workflowId, 'wf-runtime');
  assert.equal(snapshot.state.runtime.maxInputBindings, 4);
  assert.equal(snapshot.state.runtime.runtimeCapabilities[0]?.runtime_id, 'python-sidecar');
  assert.equal(snapshot.state.scheduler.sessionId, 'session-1');
  assert.equal(snapshot.state.scheduler.session?.state, 'running');
  assert.equal(snapshot.state.scheduler.items[0]?.queue_id, 'queue-1');
});
