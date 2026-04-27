import test from 'node:test';
import assert from 'node:assert/strict';

import {
  classifyLibraryAsset,
  formatLibraryAssetCategory,
  formatLibraryBytes,
  formatLibraryProjectionFreshness,
  isLibraryAssetLastUsedByRun,
} from './libraryUsagePresenters.ts';

test('classifyLibraryAsset uses explicit asset id prefixes only', () => {
  assert.equal(classifyLibraryAsset('model:llama'), 'model');
  assert.equal(classifyLibraryAsset('runtimes/llama.cpp'), 'runtime');
  assert.equal(classifyLibraryAsset('workflow:caption'), 'workflow');
  assert.equal(classifyLibraryAsset('nodes:resize'), 'node');
  assert.equal(classifyLibraryAsset('pumas:model-id'), 'pumas');
  assert.equal(classifyLibraryAsset('unknown-asset'), 'unclassified');
});

test('formatLibraryAssetCategory exposes readable labels', () => {
  assert.equal(formatLibraryAssetCategory('connector:hf'), 'Connector');
  assert.equal(formatLibraryAssetCategory('pantograph:starter'), 'Pantograph');
  assert.equal(formatLibraryAssetCategory('opaque-id'), 'Unclassified');
});

test('isLibraryAssetLastUsedByRun highlights only exact last-run matches', () => {
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: 'run-a' }, 'run-a'), true);
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: 'run-a' }, 'run-b'), false);
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: null }, 'run-a'), false);
  assert.equal(isLibraryAssetLastUsedByRun({ last_workflow_run_id: 'run-a' }, null), false);
});

test('formatLibraryBytes renders compact network totals', () => {
  assert.equal(formatLibraryBytes(512), '512 B');
  assert.equal(formatLibraryBytes(2_048), '2.0 KiB');
  assert.equal(formatLibraryBytes(2_097_152), '2.0 MiB');
});

test('formatLibraryProjectionFreshness keeps projection status visible', () => {
  assert.equal(formatLibraryProjectionFreshness(null), 'Projection unavailable');
  assert.equal(
    formatLibraryProjectionFreshness({
      projection_name: 'library_usage',
      projection_version: 1,
      last_applied_event_seq: 7,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 100,
    }),
    'Current at seq 7',
  );
});
