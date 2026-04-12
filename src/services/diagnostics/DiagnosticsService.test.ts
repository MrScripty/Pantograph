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
