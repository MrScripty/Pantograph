import test from 'node:test';
import assert from 'node:assert/strict';
import { WorkflowCommandService } from './WorkflowCommandService.ts';

test('browser Submit uses HTTP without constructing a native channel', async (context) => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'window');
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: { location: { search: '?workflowHost=http://127.0.0.1:8766' } },
  });
  context.after(() => {
    if (descriptor) Object.defineProperty(globalThis, 'window', descriptor);
    else Reflect.deleteProperty(globalThis, 'window');
  });
  const calls: unknown[] = [];
  context.mock.method(globalThis, 'fetch', async (url: string, options: RequestInit) => {
    calls.push([url, JSON.parse(String(options.body))]);
    return new Response(JSON.stringify({ workflow_run_id: 'unit-response', outputs: [], timing_ms: 0 }));
  });
  const request = {
    session_id: 'unit-session', workflow_semantic_version: '0.1.0', inputs: [],
    output_targets: null, override_selection: null, timeout_ms: null, priority: null,
  };
  const result = await new WorkflowCommandService().runWorkflowExecutionSession(request);
  assert.equal(result.workflow_run_id, 'unit-response');
  assert.deepEqual(calls, [['http://127.0.0.1:8766/api/workflow/commands/workflow_run_execution_session', { request }]]);
});

test('browser graph mutations preserve capability errors from the real transport boundary', async (context) => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'window');
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: { location: { search: '?workflowHost=http://127.0.0.1:8766' } },
  });
  context.after(() => {
    if (descriptor) Object.defineProperty(globalThis, 'window', descriptor);
    else Reflect.deleteProperty(globalThis, 'window');
  });
  context.mock.method(globalThis, 'fetch', async () => new Response(JSON.stringify({
    code: 'capability_violation', message: 'Fixture binding cannot be changed',
  }), { status: 400 }));
  await assert.rejects(
    new WorkflowCommandService().updateNodeData('infer', { runtime: 'remote' }, 'unit-session'),
    (error: unknown) => error instanceof Error && error.message === 'Fixture binding cannot be changed',
  );
});
