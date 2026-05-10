import test from 'node:test';
import assert from 'node:assert/strict';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowService } from './WorkflowService.ts';
import type { WorkflowGraphInspectionProjection } from './types.ts';

function installWindowMock(): void {
  const target = globalThis as unknown as Record<string, unknown>;
  target.window = globalThis;
}

test('inspectWorkflowGraph requests backend-owned saved graph inspection projection', async () => {
  installWindowMock();
  const response: WorkflowGraphInspectionProjection = {
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
  const calls: Array<{ cmd: string; args: unknown }> = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowService();
    const result = await service.inspectWorkflowGraph({
      path: '.pantograph/workflows/stale.json',
      selected_node_id: 'diffusion',
    });

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_graph_inspect',
        args: {
          request: {
            path: '.pantograph/workflows/stale.json',
            selected_node_id: 'diffusion',
          },
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});
