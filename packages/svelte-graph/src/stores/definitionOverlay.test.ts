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

test('resolveNodeDefinitionOverlay renders inference ports from authored snapshot', () => {
  const baseDefinitions: NodeDefinition[] = [
    {
      node_type: 'llm-inference',
      category: 'processing',
      label: 'Inference',
      description: 'Run inference',
      io_binding_origin: 'integrated',
      inputs: [
        { id: 'pumas_model_ref', label: 'Pumas Model Ref', data_type: 'json', required: true, multiple: false },
      ],
      outputs: [
        { id: 'diagnostics', label: 'Diagnostics', data_type: 'json', required: false, multiple: false },
      ],
      execution_mode: 'manual',
    },
  ];

  const resolved = resolveNodeDefinitionOverlay(
    'llm-inference',
    {
      inference_interface_snapshot: {
        contract_version: 1,
        descriptor_fingerprint: 'descriptor.image_generation.1',
        task_kind: 'image_generation',
        inputs: [
          {
            port_id: 'prompt',
            label: 'Prompt',
            direction: 'input',
            requirement: 'required',
            value_type: { category: 'scalar', kind: 'string' },
            availability: { status: 'available' },
          },
          {
            port_id: 'steps',
            label: 'Steps',
            direction: 'input',
            requirement: 'optional',
            value_type: { category: 'scalar', kind: 'u64' },
            default: { kind: 'u64', value: 4 },
            availability: { status: 'available' },
          },
        ],
        outputs: [
          {
            port_id: 'image',
            label: 'Image',
            direction: 'output',
            requirement: 'required',
            value_type: { category: 'artifact', kind: 'image' },
            availability: { status: 'available' },
          },
        ],
      },
    },
    baseDefinitions,
  );

  assert.ok(resolved, 'definition should resolve');
  assert.deepEqual(
    resolved.inputs.map((port) => [port.id, port.data_type, port.required]),
    [
      ['prompt', 'string', true],
      ['steps', 'number', false],
    ],
  );
  assert.deepEqual(
    resolved.outputs.map((port) => [port.id, port.data_type, port.required]),
    [['image', 'image', true]],
  );
  assert.deepEqual(resolved.inputs[1]?.default_value, { kind: 'u64', value: 4 });
});

test('resolveNodeDefinitionOverlay ignores retired inference definition overlays', () => {
  const baseDefinitions: NodeDefinition[] = [
    {
      node_type: 'llm-inference',
      category: 'processing',
      label: 'Inference',
      description: 'Run inference',
      io_binding_origin: 'integrated',
      inputs: [
        { id: 'pumas_model_ref', label: 'Pumas Model Ref', data_type: 'json', required: true, multiple: false },
      ],
      outputs: [],
      execution_mode: 'manual',
    },
  ];

  const resolved = resolveNodeDefinitionOverlay(
    'llm-inference',
    {
      definition: {
        node_type: 'llm-inference',
        inputs: [
          { id: 'legacy_model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
        ],
        outputs: [
          { id: 'legacy_image', label: 'Image', data_type: 'image', required: true, multiple: false },
        ],
      },
    },
    baseDefinitions,
  );

  assert.ok(resolved, 'definition should resolve');
  assert.deepEqual(
    resolved.inputs.map((port) => port.id),
    ['pumas_model_ref'],
  );
  assert.deepEqual(resolved.outputs, []);
});
