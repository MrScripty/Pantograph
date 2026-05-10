import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildSavedGraphInspectionDisplayModel,
  buildSavedGraphInspectionOptions,
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
});
