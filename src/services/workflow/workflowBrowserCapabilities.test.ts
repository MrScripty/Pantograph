import test from 'node:test';
import assert from 'node:assert/strict';
import { browserLibraryActionAllowed, type WorkflowBrowserCapabilities } from './workflowBrowserCapabilities.ts';

const readOnlyHost: WorkflowBrowserCapabilities = {
  model_library_read: true, model_library_write: false, model_search: false,
  graph_edit: false, description: 'Isolated read-only catalog',
};

test('library actions wait for explicit browser capabilities while native behavior is preserved', () => {
  assert.equal(browserLibraryActionAllowed(false, null, 'search'), true);
  assert.equal(browserLibraryActionAllowed(false, null, 'write'), true);
  assert.equal(browserLibraryActionAllowed(true, null, 'write'), false);
  assert.equal(browserLibraryActionAllowed(true, readOnlyHost, 'search'), false);
  assert.equal(browserLibraryActionAllowed(true, readOnlyHost, 'write'), false);
  assert.equal(browserLibraryActionAllowed(true, { ...readOnlyHost, model_search: true }, 'search'), true);
  assert.equal(browserLibraryActionAllowed(true, { ...readOnlyHost, model_search: true }, 'write'), false);
});
