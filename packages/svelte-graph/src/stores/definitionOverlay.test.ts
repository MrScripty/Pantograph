import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import type { NodeDefinition } from '../types/workflow.ts';
import { resolveNodeDefinitionOverlay } from './definitionOverlay.ts';

test('resolveNodeDefinitionOverlay preserves additive descriptor ports from backend data', () => {
  const baseDefinitions: NodeDefinition[] = [
    {
      node_type: 'json-filter',
      category: 'processing',
      label: 'JSON Filter',
      description: 'Filter JSON',
      io_binding_origin: 'integrated',
      inputs: [
        { id: 'input', label: 'Input', data_type: 'json', required: true, multiple: false },
      ],
      outputs: [
        { id: 'output', label: 'Output', data_type: 'json', required: false, multiple: false },
      ],
      execution_mode: 'reactive',
    },
  ];

  const resolved = resolveNodeDefinitionOverlay(
    'json-filter',
    {
      definition: {
        node_type: 'json-filter',
        inputs: [
          { id: 'input', label: 'Input', data_type: 'json', required: true, multiple: false },
          { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false },
        ],
        outputs: [
          { id: 'output', label: 'Output', data_type: 'json', required: false, multiple: false },
          { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false },
        ],
      },
    },
    baseDefinitions,
  );

  assert.ok(resolved, 'definition should resolve');
  assert.deepEqual(
    resolved.inputs.map((port) => port.id),
    ['input', 'temperature'],
  );
  assert.deepEqual(
    resolved.outputs.map((port) => port.id),
    ['output', 'temperature'],
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

test('saved embedding descriptor ports retain embedding and JSON types in the graph overlay', () => {
  const snapshot = JSON.parse(readFileSync(new URL(
    '../../../../crates/pantograph-inference-interface-contracts/tests/fixtures/authored_snapshot_embedding.json',
    import.meta.url,
  ), 'utf8'));
  const definitions: NodeDefinition[] = [{
    node_type: 'llm-inference', category: 'processing', label: 'Inference',
    description: 'Run inference', io_binding_origin: 'integrated',
    inputs: [], outputs: [], execution_mode: 'manual',
  }];
  const resolved = resolveNodeDefinitionOverlay('llm-inference', {
    inference_interface_snapshot: JSON.parse(JSON.stringify(snapshot)),
  }, definitions);
  assert.ok(resolved);
  assert.deepEqual(resolved.inputs.map(port => [port.id, port.data_type, port.required]), [
    ['text', 'string', true],
  ]);
  assert.equal('default_value' in resolved.inputs[0], false);
  assert.deepEqual(resolved.outputs.map(port => [port.id, port.data_type, port.required]), [
    ['embedding', 'embedding', true], ['metadata', 'json', true], ['usage', 'json', false],
  ]);
  assert.equal(resolved.outputs[0].data_type, 'embedding');
  assert.notEqual(resolved.outputs[0].data_type, 'tensor');
});

test('inference payload overlay retains the schema-owned dependency sidecar handle', () => {
  const sidecar = { id: 'dependency_environment_sidecar', label: 'Dependencies',
    data_type: 'dependency_environment_sidecar' as const, required: false, multiple: false };
  const definition: NodeDefinition = { node_type: 'llm-inference', category: 'processing',
    label: 'Inference', description: 'Run inference', io_binding_origin: 'integrated',
    inputs: [sidecar, { id: 'runtime', label: 'Runtime', data_type: 'string', required: false, multiple: false }],
    outputs: [], execution_mode: 'manual' };
  const resolved = resolveNodeDefinitionOverlay('llm-inference', {
    inference_interface_snapshot: { inputs: [{ port_id: 'text', label: 'Text', direction: 'input',
      requirement: 'required', value_type: { category: 'scalar', kind: 'string' } }], outputs: [] },
  }, [definition]);
  assert.deepEqual(resolved?.inputs.map((port) => port.id), ['text', 'dependency_environment_sidecar']);
  assert.deepEqual(resolved?.inputs[1], sidecar);
  assert.deepEqual(definition.inputs.map((port) => port.id), ['dependency_environment_sidecar', 'runtime']);
});
