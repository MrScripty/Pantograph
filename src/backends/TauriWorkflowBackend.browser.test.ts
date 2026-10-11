import test from 'node:test';
import assert from 'node:assert/strict';
import { TauriWorkflowBackend } from './TauriWorkflowBackend.ts';
import { WorkflowCommandService } from '../services/workflow/WorkflowCommandService.ts';
import type { WorkflowGraph } from '../services/workflow/types.ts';

// Exercise the browser transport through production adapters. Native-only Channel
// construction would throw: this window intentionally has no Tauri internals.
test('browser graph load, edit, save and scheduler submit use HTTP without native IPC', async () => {
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, 'window');
  const previousFetch = globalThis.fetch;
  Object.defineProperty(globalThis, 'window', {
    configurable: true, value: { location: { search: '?workflowHost=http://127.0.0.1:8765' } },
  });
  const graph: WorkflowGraph = { nodes: [], edges: [] };
  const calls: Array<{ command: string; args: unknown }> = [];
  const responses: Record<string, unknown> = {
    load_workflow: { graph },
    create_workflow_execution_session: { session_id: 'edit-test', session_kind: 'edit' },
    update_node_data: { graph },
    save_workflow: '/isolated/workflows/example.json',
    workflow_run_execution_session: { workflow_run_id: 'test-run', outputs: [], timing_ms: 1 },
  };
  globalThis.fetch = async (url, options) => {
    const command = String(url).split('/').pop()!;
    calls.push({ command, args: JSON.parse(String(options?.body)) });
    return new Response(JSON.stringify(responses[command]), { status: 200 });
  };
  try {
    const backend = new TauriWorkflowBackend();
    await backend.loadWorkflow('example');
    await backend.createSession(graph, 'example');
    await backend.updateNodeData('input', { text: ' hello\n' }, 'edit-test');
    await backend.saveWorkflow('example', graph);
    const request = { session_id: 'scheduler-test', workflow_semantic_version: '1.0.0', inputs: [{ node_id: 'input', port_id: 'text', value: ' hello\n' }] };
    const result = await new WorkflowCommandService().runWorkflowExecutionSession(request);
    assert.equal(result.workflow_run_id, 'test-run');
    assert.deepEqual(calls, [
      { command: 'load_workflow', args: { path: 'example' } },
      { command: 'create_workflow_execution_session', args: { graph, workflowId: 'example' } },
      { command: 'update_node_data', args: { executionId: 'edit-test', nodeId: 'input', data: { text: ' hello\n' } } },
      { command: 'save_workflow', args: { name: 'example', graph } },
      { command: 'workflow_run_execution_session', args: { request } },
    ]);
  } finally {
    globalThis.fetch = previousFetch;
    if (previousWindow) Object.defineProperty(globalThis, 'window', previousWindow);
    else Reflect.deleteProperty(globalThis, 'window');
  }
});
