import test from 'node:test';
import assert from 'node:assert/strict';
import { invokeBrowserWorkflowCommand, resolveWorkflowBrowserHost } from './workflowBrowserTransport.ts';
import { normalizeWorkflowServiceError } from './workflowServiceErrors.ts';
import { pollDiagnosticsProjections } from './WorkflowProjectionSubscriptionService.ts';

test('browser transport is explicit, loopback-only and never overrides native IPC', () => {
  assert.equal(resolveWorkflowBrowserHost(''), null);
  assert.equal(resolveWorkflowBrowserHost('?workflowHost=http://127.0.0.1:8765'), 'http://127.0.0.1:8765');
  assert.equal(resolveWorkflowBrowserHost('?workflowHost=invalid', true), null);
  for (const host of ['https://127.0.0.1', 'http://example.com', 'http://localhost/private', 'http://user@localhost']) {
    assert.throws(() => resolveWorkflowBrowserHost(`?workflowHost=${encodeURIComponent(host)}`));
  }
});

test('browser requests real commands and preserves backend diagnostic failures', async () => {
  const calls: unknown[] = [];
  const backendError = {
    code: 'runtime_not_ready', message: 'No backend loaded',
    diagnostics: { workflow_run_id: 'actual-run', diagnostic_event_id: 'actual-event' },
  };
  const request: typeof fetch = async (url, options) => {
    calls.push([url, JSON.parse(String(options?.body))]);
    return new Response(JSON.stringify(backendError), { status: 503 });
  };
  await assert.rejects(
    invokeBrowserWorkflowCommand('http://127.0.0.1:8765', 'workflow_run_detail_query', { request: { workflow_run_id: 'actual-run' } }, request),
    (error: unknown) => {
      const normalized = normalizeWorkflowServiceError(error);
      assert.equal(normalized.code, 'runtime_not_ready');
      assert.equal(normalized.diagnostics?.diagnostic_event_id, 'actual-event');
      return true;
    },
  );
  assert.deepEqual(calls, [['http://127.0.0.1:8765/api/workflow/commands/workflow_run_detail_query', { request: { workflow_run_id: 'actual-run' } }]]);
});

test('polling serializes refreshes, reports failures, recovers and stops on unsubscribe', async () => {
  let calls = 0;
  let concurrent = 0;
  let maximum = 0;
  const errors: unknown[] = [];
  let complete!: () => void;
  const recovered = new Promise<void>((resolve) => { complete = resolve; });
  const stop = pollDiagnosticsProjections({
    projections: ['run_list'],
    refresh: async (event) => {
      assert.equal(event.reason, 'explicit_refresh');
      maximum = Math.max(maximum, ++concurrent);
      calls += 1;
      await new Promise((resolve) => setTimeout(resolve, 5));
      concurrent -= 1;
      if (calls === 1) throw new Error('host disconnected');
      complete();
    },
    onRefreshError: (error) => errors.push(error),
  }, 1);
  await recovered;
  stop();
  const finalCalls = calls;
  await new Promise((resolve) => setTimeout(resolve, 15));
  assert.equal(maximum, 1);
  assert.equal(errors.length, 1);
  assert.equal((errors[0] as Error).message, 'host disconnected');
  assert.equal(calls, finalCalls);
});
