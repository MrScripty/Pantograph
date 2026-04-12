import test from 'node:test';
import assert from 'node:assert/strict';

import {
  createDiagnosticsSnapshot,
  createWorkflowDiagnosticsContext,
  createWorkflowDiagnosticsState,
  recordWorkflowEvent,
  updateDiagnosticsStateRuntimeSnapshot,
  updateDiagnosticsStateSchedulerSnapshot,
  updateDiagnosticsStateWorkflowContext,
  updateWorkflowContext,
} from './traceAccumulator.ts';
import type { WorkflowEvent, WorkflowGraph } from '../workflow/types.ts';

function createGraph(): WorkflowGraph {
  return {
    nodes: [
      {
        id: 'llm-1',
        node_type: 'llm-inference',
        position: { x: 0, y: 0 },
        data: {},
      },
    ],
    edges: [],
    derived_graph: {
      schema_version: 1,
      graph_fingerprint: 'graph-123',
      consumer_count_map: {},
    },
  };
}

test('recordWorkflowEvent creates a run trace and computes node duration', () => {
  const state = createWorkflowDiagnosticsState();
  let context = createWorkflowDiagnosticsContext();
  context = updateWorkflowContext(context, {
    workflowId: 'wf-1',
    workflowName: 'Test Workflow',
    graph: createGraph(),
    graphFingerprint: 'graph-123',
    nodeTypesById: {
      'llm-1': 'llm-inference',
    },
  });

  const started: WorkflowEvent = {
    type: 'Started',
    data: {
      workflow_id: 'wf-1',
      node_count: 1,
      execution_id: 'exec-1',
    },
  };
  const nodeStarted: WorkflowEvent = {
    type: 'NodeStarted',
    data: {
      node_id: 'llm-1',
      node_type: '',
      execution_id: 'exec-1',
    },
  };
  const nodeCompleted: WorkflowEvent = {
    type: 'NodeCompleted',
    data: {
      node_id: 'llm-1',
      outputs: {},
      execution_id: 'exec-1',
    },
  };
  const completed: WorkflowEvent = {
    type: 'Completed',
    data: {
      workflow_id: 'wf-1',
      outputs: {},
      execution_id: 'exec-1',
    },
  };

  recordWorkflowEvent(state, context, started, 1_000);
  recordWorkflowEvent(state, context, nodeStarted, 1_010);
  recordWorkflowEvent(state, context, nodeCompleted, 1_050);
  recordWorkflowEvent(state, context, completed, 1_100);

  const snapshot = createDiagnosticsSnapshot(state);
  const run = snapshot.selectedRun;
  assert.ok(run);
  assert.equal(run.executionId, 'exec-1');
  assert.equal(run.workflowName, 'Test Workflow');
  assert.equal(run.graphFingerprintAtStart, 'graph-123');
  assert.equal(run.status, 'completed');
  assert.equal(run.durationMs, 100);
  assert.equal(run.events.length, 4);

  const node = run.nodes['llm-1'];
  assert.ok(node);
  assert.equal(node.nodeType, 'llm-inference');
  assert.equal(node.status, 'completed');
  assert.equal(node.durationMs, 40);
});

test('recordWorkflowEvent marks waiting runs and retains the waiting node message', () => {
  const state = createWorkflowDiagnosticsState();
  let context = createWorkflowDiagnosticsContext();
  context = updateWorkflowContext(context, {
    workflowId: 'wf-2',
    graph: createGraph(),
    nodeTypesById: {
      'llm-1': 'llm-inference',
    },
  });

  recordWorkflowEvent(state, context, {
    type: 'Started',
    data: {
      workflow_id: 'wf-2',
      node_count: 1,
      execution_id: 'exec-2',
    },
  }, 2_000);

  recordWorkflowEvent(state, context, {
    type: 'WaitingForInput',
    data: {
      workflow_id: 'wf-2',
      execution_id: 'exec-2',
      node_id: 'llm-1',
      message: 'Provide confirmation',
    },
  }, 2_050);

  const snapshot = createDiagnosticsSnapshot(state);
  const run = snapshot.selectedRun;
  assert.ok(run);
  assert.equal(run.status, 'waiting');
  assert.equal(run.waitingForInput, true);
  assert.deepEqual(run.events.map(event => event.type), ['Started', 'WaitingForInput']);

  const node = run.nodes['llm-1'];
  assert.ok(node);
  assert.equal(node.status, 'waiting');
  assert.equal(node.lastMessage, 'Provide confirmation');
});

test('recordWorkflowEvent enforces the retained event limit per run', () => {
  const state = createWorkflowDiagnosticsState(2);
  let context = createWorkflowDiagnosticsContext();
  context = updateWorkflowContext(context, {
    workflowId: 'wf-3',
  });

  recordWorkflowEvent(state, context, {
    type: 'Started',
    data: {
      workflow_id: 'wf-3',
      node_count: 0,
      execution_id: 'exec-3',
    },
  }, 10);

  recordWorkflowEvent(state, context, {
    type: 'GraphModified',
    data: {
      workflow_id: 'wf-3',
      execution_id: 'exec-3',
      graph: null,
      dirty_tasks: ['node-a'],
    },
  }, 20);

  recordWorkflowEvent(state, context, {
    type: 'Completed',
    data: {
      workflow_id: 'wf-3',
      outputs: {},
      execution_id: 'exec-3',
    },
  }, 30);

  const snapshot = createDiagnosticsSnapshot(state);
  const run = snapshot.selectedRun;
  assert.ok(run);
  assert.equal(run.events.length, 2);
  assert.deepEqual(run.events.map(event => event.type), ['GraphModified', 'Completed']);
});

test('updateWorkflowContext preserves unspecified fields and clears explicit nulls', () => {
  const initial = updateWorkflowContext(createWorkflowDiagnosticsContext(), {
    workflowId: 'wf-4',
    workflowName: 'Workflow Four',
    graph: createGraph(),
    graphFingerprint: 'graph-444',
    nodeTypesById: {
      'llm-1': 'llm-inference',
    },
  });

  const preserved = updateWorkflowContext(initial, {
    workflowId: 'wf-5',
  });
  assert.equal(preserved.workflowId, 'wf-5');
  assert.equal(preserved.workflowName, 'Workflow Four');
  assert.equal(preserved.graphFingerprint, 'graph-444');
  assert.equal(preserved.graph?.nodes.length, 1);
  assert.equal(preserved.nodeTypesById['llm-1'], 'llm-inference');

  const cleared = updateWorkflowContext(preserved, {
    workflowName: null,
    graph: null,
  });
  assert.equal(cleared.workflowId, 'wf-5');
  assert.equal(cleared.workflowName, null);
  assert.equal(cleared.graph, null);
  assert.equal(cleared.graphFingerprint, 'graph-444');
});

test('diagnostics state captures runtime scheduler and graph summaries', () => {
  const context = updateWorkflowContext(createWorkflowDiagnosticsContext(), {
    workflowId: 'wf-6',
    workflowName: 'Workflow Six',
    graph: createGraph(),
    graphFingerprint: 'graph-666',
    nodeTypesById: {
      'llm-1': 'llm-inference',
    },
  });

  let state = createWorkflowDiagnosticsState();
  state = updateDiagnosticsStateWorkflowContext(state, context);
  state = updateDiagnosticsStateRuntimeSnapshot(state, 'wf-6', {
    max_input_bindings: 8,
    max_output_targets: 4,
    max_value_bytes: 1000,
    runtime_requirements: {
      estimation_confidence: 'medium',
      required_models: ['model-1'],
      required_backends: ['onnx-runtime'],
      required_extensions: [],
    },
    models: [],
    runtime_capabilities: [{
      runtime_id: 'onnx-runtime',
      display_name: 'ONNX Runtime',
      install_state: 'installed',
      available: true,
      configured: true,
      can_install: false,
      can_remove: false,
      backend_keys: ['onnx-runtime'],
      missing_files: [],
    }],
  }, 1_000, null);
  state = updateDiagnosticsStateSchedulerSnapshot(state, 'wf-6', 'session-6', {
    session: {
      session_id: 'session-6',
      workflow_id: 'wf-6',
      keep_alive: false,
      state: 'idle_loaded',
      queued_runs: 2,
      run_count: 5,
    },
  }, {
    session_id: 'session-6',
    items: [{
      queue_id: 'queue-6',
      run_id: 'run-6',
      priority: 4,
      status: 'pending',
    }],
  }, 2_000, null);

  assert.equal(state.currentGraphFingerprint, 'graph-666');
  assert.equal(state.currentGraphNodeCount, 1);
  assert.equal(state.currentGraphEdgeCount, 0);
  assert.equal(state.runtime.maxInputBindings, 8);
  assert.equal(state.runtime.runtimeRequirements?.required_backends[0], 'onnx-runtime');
  assert.equal(state.scheduler.session?.queued_runs, 2);
  assert.equal(state.scheduler.items[0]?.queue_id, 'queue-6');
});
