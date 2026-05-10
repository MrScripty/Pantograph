import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildGraphInspectionCanvasModel,
  buildSavedGraphInspectionDisplayModel,
  buildSavedGraphInspectionOptions,
  formatSavedGraphNodeAccessibleLabel,
  isSavedGraphNodeSelectionKey,
  resolveSavedGraphSelectedNodeId,
  resolveSavedWorkflowInspectionPath,
} from './graphInspectionPresenters.ts';
import type { WorkflowGraphInspectionProjection } from '../../services/workflow/types.ts';

test('saved graph inspection options require backend-listed workflow ids', () => {
  const options = buildSavedGraphInspectionOptions([
    {
      id: 'stale-workflow',
      name: 'Stale Workflow',
      created: '2026-05-10T00:00:00Z',
      modified: '2026-05-10T01:00:00Z',
    },
    {
      name: 'Name Without Stable Id',
      created: '2026-05-10T00:00:00Z',
      modified: '2026-05-10T01:00:00Z',
    },
  ]);

  assert.deepEqual(options, [
    {
      workflowId: 'stale-workflow',
      label: 'Stale Workflow',
      modified: '2026-05-10T01:00:00Z',
      inspectionPath: '.pantograph/workflows/stale-workflow.json',
    },
  ]);
});

test('saved graph inspection display model uses backend diagnostics without run metadata', () => {
  const projection: WorkflowGraphInspectionProjection = {
    graph: {
      nodes: [
        {
          id: 'diffusion',
          node_type: 'diffusion-inference',
          position: { x: 0, y: 0 },
          data: {},
        },
      ],
      edges: [],
    },
    selected_node: {
      node: {
        id: 'diffusion',
        node_type: 'diffusion-inference',
        position: { x: 0, y: 0 },
        data: {},
      },
      diagnostics: [
        {
          code: 'retired_node_type',
          severity: 'error',
          scope: 'node',
          node_id: 'diffusion',
          node_type: 'diffusion-inference',
          message: 'node type diffusion-inference is retired',
          blocking_submission: true,
        },
      ],
    },
    diagnostics: [
      {
        code: 'retired_node_type',
        severity: 'error',
        scope: 'node',
        node_id: 'diffusion',
        node_type: 'diffusion-inference',
        message: 'node type diffusion-inference is retired',
        blocking_submission: true,
      },
    ],
    run_context: null,
  };

  const model = buildSavedGraphInspectionDisplayModel(projection, 'diffusion');

  assert.equal(model.hasRunContext, false);
  assert.equal(model.artifactControlsEnabled, false);
  assert.equal(model.selectedNodeId, 'diffusion');
  assert.deepEqual(
    model.selectedNodeDiagnostics.map((diagnostic) => diagnostic.message),
    ['node type diffusion-inference is retired'],
  );
  assert.equal(model.canvas.nodes[0].staleDiagnosticCount, 1);
  assert.equal(model.canvas.nodes[0].staleSeverity, 'error');
  assert.equal(
    formatSavedGraphNodeAccessibleLabel(model.canvas.nodes[0]),
    'diffusion diffusion-inference, 1 stale graph fact',
  );
  assert.equal(isSavedGraphNodeSelectionKey('Enter'), true);
  assert.equal(isSavedGraphNodeSelectionKey(' '), true);
  assert.equal(isSavedGraphNodeSelectionKey('Escape'), false);
});

test('graph inspection canvas wrapper preserves backend stale diagnostics', () => {
  const canvas = buildGraphInspectionCanvasModel(
    {
      nodes: [
        {
          id: 'node-1',
          node_type: 'unknown-node',
          position: { x: 12, y: 24 },
          data: {},
        },
      ],
      edges: [],
    },
    [
      {
        code: 'unknown_node_type',
        severity: 'error',
        scope: 'node',
        node_id: 'node-1',
        node_type: 'unknown-node',
        message: 'node type unknown-node is not registered',
        blocking_submission: true,
      },
    ],
  );

  assert.equal(canvas.nodes[0].staleDiagnosticCount, 1);
  assert.equal(canvas.nodes[0].staleBadgeLabel, '1 stale graph fact');
});

test('saved graph inspection selection helpers preserve valid frontend selection', () => {
  const options = buildSavedGraphInspectionOptions([
    {
      id: 'first-workflow',
      name: 'First Workflow',
      created: '2026-05-10T00:00:00Z',
      modified: '2026-05-10T01:00:00Z',
    },
    {
      id: 'second-workflow',
      name: 'Second Workflow',
      created: '2026-05-10T00:00:00Z',
      modified: '2026-05-10T02:00:00Z',
    },
  ]);
  const projection: WorkflowGraphInspectionProjection = {
    graph: {
      nodes: [
        { id: 'first', node_type: 'text-input', position: { x: 0, y: 0 }, data: {} },
        { id: 'stale', node_type: 'unknown-node', position: { x: 100, y: 0 }, data: {} },
      ],
      edges: [],
    },
    selected_node: null,
    diagnostics: [
      {
        code: 'unknown_node_type',
        severity: 'error',
        scope: 'node',
        node_id: 'stale',
        node_type: 'unknown-node',
        message: 'node type unknown-node is not registered',
        blocking_submission: true,
      },
    ],
    run_context: null,
  };

  assert.equal(
    resolveSavedWorkflowInspectionPath('.pantograph/workflows/second-workflow.json', options),
    '.pantograph/workflows/second-workflow.json',
  );
  assert.equal(resolveSavedWorkflowInspectionPath('missing.json', options), options[0].inspectionPath);
  assert.equal(resolveSavedGraphSelectedNodeId('first', projection), 'first');
  assert.equal(resolveSavedGraphSelectedNodeId('missing', projection), 'stale');
});
