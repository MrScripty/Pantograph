import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const mockWorkflowBackendSourcePath = fileURLToPath(
  new URL('./MockWorkflowBackend.ts', import.meta.url),
);

test('mock workflow backend keeps Pumas inference definitions intent-only', () => {
  const source = readFileSync(mockWorkflowBackendSourcePath, 'utf8');

  for (const canonicalPortId of ['task_kind', 'runtime', 'device', 'pumas_model_ref']) {
    assert.match(source, new RegExp(`id: '${canonicalPortId}'`));
  }

  for (const retiredPortId of [
    'backend_key',
    'runtime_hint',
    'resolved_model_source',
    'resolved_model_package_facts',
    'dependency_requirements',
    'inference_settings',
  ]) {
    assert.doesNotMatch(source, new RegExp(`id: '${retiredPortId}'`));
  }
});
