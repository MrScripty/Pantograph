import test from 'node:test';
import assert from 'node:assert/strict';
import type { WorkflowGraphCurrentValidationSummaryResponse } from '../services/workflow/types.ts';
import { currentWorkflowValidation, currentWorkflowSubmitGate, executableWorkflowValidation, WorkflowValidationReadFence } from './workflowValidationAuthority.ts';

// Controlled negative contract tests; browser replay uses fresh Rust-produced responses.
const valid = (): WorkflowGraphCurrentValidationSummaryResponse => ({
  graph_session_id: 'session', requested_graph_revision: 'a3fee3326cbb58cd', current_graph_revision: 'a3fee3326cbb58cd',
  validation_session_id: 'generation', state: 'current', submit_gate: { allowed: true },
  summary: { status: 'executable', executable: true, diagnostics_count: 0, blocking_diagnostics_count: 0 },
  group_preflight: { graph_session_id: 'session', graph_revision: 'a3fee3326cbb58cd', validation_session_id: 'generation', group_count: 1 },
});
const allowed = (response: WorkflowGraphCurrentValidationSummaryResponse) => executableWorkflowValidation(response, 'session', 'a3fee3326cbb58cd');

test('positive gates require current status, generation, complete authored identity and no typed blocking facts', () => {
  assert.equal(allowed(valid()), true);
  for (const field of ['graph_session_id', 'requested_graph_revision', 'current_graph_revision', 'validation_session_id'] as const) {
    const response = valid(); response[field] = '';
    assert.equal(allowed(response), false, field);
  }
  for (const state of ['missing', 'stale', 'invalid'] as const) {
    const response = valid(); response.state = state; assert.equal(allowed(response), false, state);
  }
  for (const field of ['graph_session_id', 'graph_revision', 'validation_session_id'] as const) {
    const response = valid(); response.group_preflight![field] = 'foreign';
    assert.equal(currentWorkflowValidation(response, 'session', 'a3fee3326cbb58cd'), null);
  }
  const blocked = valid();
  blocked.group_preflight!.failures = [{ code: 'group_child_unsupported', rejection_kind: 'unsupported',
    field: 'group.nodes.node_type', message: 'Blocked', repair_hint: 'Repair', blocking_submission: true }];
  assert.equal(allowed(blocked), false);
  assert.equal(currentWorkflowSubmitGate(blocked, 'session', 'a3fee3326cbb58cd'), null);
  for (const patch of [{ executable: false }, { status: 'blocked' as const }, { blocking_diagnostics_count: 1 }, { enqueue_disabled_reasons: ['validation_pending' as const] }]) {
    const response = valid(); Object.assign(response.summary!, patch); assert.equal(allowed(response), false);
  }
});

test('one invalidation epoch rejects old successes, old errors and graph A-B-A reads without ordering generation UUIDs', () => {
  const fence = new WorkflowValidationReadFence();
  const refresh = fence.begin('session:a');
  const lifecycle = fence.begin('session:a');
  assert.equal(fence.isCurrent(refresh), false);
  assert.equal(fence.isCurrent(lifecycle), true);
  fence.begin('session:b');
  const returned = fence.begin('session:a');
  assert.equal(fence.isCurrent(refresh), false);
  assert.equal(fence.isCurrent(lifecycle), false);
  assert.equal(fence.isCurrent(returned), true);
  fence.begin(null);
  assert.equal(fence.isCurrent(returned), false);
});
