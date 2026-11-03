import assert from 'node:assert/strict';
import test from 'node:test';
import { graphSessionStorageKey, selectBrowserStartupWorkflow } from './browserGraphStartup.ts';

test('browser startup restores only a workflow in the selected host catalog', () => {
  const workflows = [{ id: 'fixture', name: 'Fixture' }, { id: 'other', name: 'Other' }];
  assert.equal(selectBrowserStartupWorkflow(workflows, { id: 'other', type: 'workflow' }), 'other');
  assert.equal(selectBrowserStartupWorkflow(workflows, { id: 'coding-agent', type: 'workflow' }), 'fixture');
  assert.equal(selectBrowserStartupWorkflow(workflows, { id: 'app-architecture', type: 'system' }), 'fixture');
  assert.equal(selectBrowserStartupWorkflow([], null), null);
});

test('browser host graph persistence cannot overwrite native or another host selection', () => {
  assert.equal(graphSessionStorageKey(null), 'pantograph.lastGraph');
  assert.notEqual(graphSessionStorageKey('http://127.0.0.1:8766'), graphSessionStorageKey(null));
  assert.notEqual(graphSessionStorageKey('http://127.0.0.1:8766'), graphSessionStorageKey('http://127.0.0.1:8765'));
});
