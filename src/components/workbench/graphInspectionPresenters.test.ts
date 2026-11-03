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
  savedGraphNodeFocusDomId,
  buildNodePaletteTree,
  editNodeExecutionGlyph,
  formatEditCurrentLabel,
  formatEditDimensionLabel,
  formatRunDimensionLabel,
  formatRunModeLabel,
  formatWorkflowModifiedShort,
  formatWorkflowSaveLine,
  resolveGraphPageMode,
  runIdSuffix,
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

test('saved graph node focus ids are stable and selector-safe', () => {
  assert.equal(savedGraphNodeFocusDomId('puma-lib-1'), 'saved-graph-node-puma-lib-1');
  assert.equal(
    savedGraphNodeFocusDomId('puma-lib-1:pumas_model_ref'),
    'saved-graph-node-puma-lib-1_003a_pumas_model_ref',
  );
  assert.equal(savedGraphNodeFocusDomId(''), 'saved-graph-node-node');
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

const RUN = { workflow_run_id: 'run_7c9e6679-7425-40de-944b-e07fc1f90ae7', workflow_id: 'ingest-docs-rag' };

test('graph page opens a run as run when its workflow is the one being edited', () => {
  assert.deepEqual(
    resolveGraphPageMode({ activeRun: RUN, loadedWorkflowId: 'ingest-docs-rag', request: null }),
    { mode: 'run', runAvailable: true, workflowMismatch: false },
  );
});

test('graph page stays in edit mode and offers the run when the run is of another workflow', () => {
  assert.deepEqual(
    resolveGraphPageMode({ activeRun: RUN, loadedWorkflowId: 'summarize-daily', request: null }),
    { mode: 'edit', runAvailable: true, workflowMismatch: true },
  );
  assert.deepEqual(
    resolveGraphPageMode({ activeRun: RUN, loadedWorkflowId: null, request: null }),
    { mode: 'edit', runAvailable: true, workflowMismatch: true },
    'a system graph or a loading editor is never the run workflow',
  );
});

test('graph page without a run is always the editor', () => {
  assert.deepEqual(
    resolveGraphPageMode({
      activeRun: null,
      loadedWorkflowId: 'ingest-docs-rag',
      request: { runId: null, mode: 'run' },
    }),
    { mode: 'edit', runAvailable: false, workflowMismatch: false },
  );
});

test('an explicit mode choice holds for its run and resets when another run is selected', () => {
  const editRequest = { runId: RUN.workflow_run_id, mode: 'edit' as const };
  assert.equal(
    resolveGraphPageMode({ activeRun: RUN, loadedWorkflowId: 'ingest-docs-rag', request: editRequest }).mode,
    'edit',
  );
  const runRequest = { runId: RUN.workflow_run_id, mode: 'run' as const };
  const shown = resolveGraphPageMode({ activeRun: RUN, loadedWorkflowId: 'summarize-daily', request: runRequest });
  assert.equal(shown.mode, 'run');
  assert.equal(shown.workflowMismatch, false);
  const otherRun = { workflow_run_id: 'run_other', workflow_id: 'ingest-docs-rag' };
  assert.equal(
    resolveGraphPageMode({ activeRun: otherRun, loadedWorkflowId: 'ingest-docs-rag', request: editRequest }).mode,
    'run',
  );
  const noWorkflow = { workflow_run_id: 'run_x', workflow_id: null };
  assert.deepEqual(
    resolveGraphPageMode({ activeRun: noWorkflow, loadedWorkflowId: 'ingest-docs-rag', request: null }),
    { mode: 'edit', runAvailable: true, workflowMismatch: false },
  );
});

test('graph page labels use the last six characters of the run id', () => {
  assert.equal(runIdSuffix(RUN.workflow_run_id), 'f90ae7');
  assert.equal(runIdSuffix('run_1'), 'run_1');
  assert.equal(formatRunModeLabel(RUN.workflow_run_id), 'As run f90ae7');
  assert.equal(formatRunModeLabel(null), 'As a run…');
  assert.equal(formatEditDimensionLabel('1.4.1', 6, 1), 'Editing v1.4.1, 6 nodes, 1 edge');
  assert.equal(formatEditDimensionLabel(null, 1, 0), 'Viewing, 1 node, 0 edges');
  assert.equal(
    formatRunDimensionLabel(1, RUN.workflow_run_id, 'Running'),
    '1 node as run f90ae7 ran them, running',
  );
  assert.equal(formatEditCurrentLabel('1.4.1'), 'Edit current v1.4.1');
  assert.equal(formatEditCurrentLabel(null), 'Edit current version');
});

test('save line prefers unsaved changes, then the saved time', () => {
  assert.equal(formatWorkflowSaveLine({ isDirty: true, savedModifiedAt: '2026-11-03T10:00:00Z' }), 'Unsaved changes');
  assert.equal(formatWorkflowSaveLine({ isDirty: false, savedModifiedAt: null }), 'Not saved yet');
  assert.equal(formatWorkflowSaveLine({ isDirty: false, savedModifiedAt: 'not a date' }), 'Saved');
  const modified = new Date(2026, 10, 3, 20, 4).toISOString();
  assert.equal(formatWorkflowSaveLine({ isDirty: false, savedModifiedAt: modified }), 'Saved 20:04');
});

test('editor execution states map to the shared glyphs', () => {
  assert.deepEqual(editNodeExecutionGlyph('running'), { glyph: 'running', label: 'Running' });
  assert.deepEqual(editNodeExecutionGlyph('waiting'), { glyph: 'waiting', label: 'Waiting' });
  assert.deepEqual(editNodeExecutionGlyph('success'), { glyph: 'succeeded', label: 'Done' });
  assert.deepEqual(editNodeExecutionGlyph('error'), { glyph: 'failed', label: 'Failed' });
  assert.deepEqual(editNodeExecutionGlyph(undefined), { glyph: 'idle', label: 'Idle' });
});

test('palette tree counts nodes per category, shows in:out ports and opens matches while searching', () => {
  const def = (node_type: string, label: string, inputs: number, outputs: number, description = '') => ({
    node_type,
    label,
    description,
    inputs: Array.from({ length: inputs }, () => ({})),
    outputs: Array.from({ length: outputs }, () => ({})),
  });
  const catalogue = new Map([
    ['input', [def('text-input', 'Text input', 0, 1), def('read-file', 'Read file', 0, 1, 'reads text')]],
    ['processing', [def('llm-inference', 'LLM inference', 4, 3)]],
    ['tool_kit', [def('unload-model', 'Unload model', 1, 0)]],
  ]);

  const tree = buildNodePaletteTree(catalogue, '', new Set(['input']));
  assert.deepEqual(
    tree.map((category) => [category.label, category.count, category.expanded]),
    [['Input', 2, true], ['Processing', 1, false], ['Tool kit', 1, false]],
  );
  assert.equal(tree[1].items[0].ports, '4:3');
  assert.equal(tree[1].items[0].portsTitle, '4 inputs, 3 outputs');

  const searched = buildNodePaletteTree(catalogue, '  TEXT ', new Set());
  assert.deepEqual(
    searched.map((category) => [category.key, category.count, category.expanded]),
    [['input', 2, true]],
    'description matches count too',
  );
  assert.deepEqual(buildNodePaletteTree(catalogue, 'nothing', new Set()), []);
});

test('workflow list shows the time for today and the date otherwise', () => {
  const now = new Date(2026, 10, 3, 21, 0).getTime();
  assert.equal(formatWorkflowModifiedShort(new Date(2026, 10, 3, 20, 4).toISOString(), now), '20:04');
  assert.equal(formatWorkflowModifiedShort(new Date(2026, 9, 29, 9, 0).toISOString(), now), 'Oct 29');
  assert.equal(formatWorkflowModifiedShort('garbage', now), '');
});
