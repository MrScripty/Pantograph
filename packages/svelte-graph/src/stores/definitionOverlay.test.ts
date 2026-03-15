import test from 'node:test';
import assert from 'node:assert/strict';

import type { NodeDefinition } from '../types/workflow.ts';
import { resolveNodeDefinitionOverlay } from './definitionOverlay.ts';

test('resolveNodeDefinitionOverlay preserves additive dynamic ports from backend data', () => {
  const baseDefinitions: NodeDefinition[] = [
    {
      node_type: 'expand-settings',
      category: 'processing',
      label: 'Expand Settings',
      description: 'Expose settings',
      io_binding_origin: 'integrated',
      inputs: [
        { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: true, multiple: false },
      ],
      outputs: [
        { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: true, multiple: false },
      ],
      execution_mode: 'reactive',
    },
  ];

  const resolved = resolveNodeDefinitionOverlay(
    'expand-settings',
    {
      definition: {
        node_type: 'expand-settings',
        inputs: [
          { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: true, multiple: false },
          { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false },
        ],
        outputs: [
          { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: true, multiple: false },
          { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false },
        ],
      },
    },
    baseDefinitions,
  );

  assert.ok(resolved, 'definition should resolve');
  assert.deepEqual(
    resolved.inputs.map((port) => port.id),
    ['inference_settings', 'temperature'],
  );
  assert.deepEqual(
    resolved.outputs.map((port) => port.id),
    ['inference_settings', 'temperature'],
  );
});
