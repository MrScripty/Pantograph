import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { get, writable } from 'svelte/store';

import type { NodeDefinition } from '../types/workflow.ts';
import { resolveNodeDefinitionOverlay } from './definitionOverlay.ts';
import { createWorkflowStoreGraphState } from './workflowStoreGraphState.ts';

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

const dependencySidecar = {
  id: 'dependency_environment_sidecar', label: 'Dependency Environment',
  data_type: 'dependency_environment_sidecar', required: false, multiple: false,
  description: 'Typed dependency control',
} as const;

function embeddingSnapshot() {
  return JSON.parse(readFileSync(new URL(
    '../../../../crates/pantograph-inference-interface-contracts/tests/fixtures/authored_snapshot_embedding.json',
    import.meta.url,
  ), 'utf8'));
}

function inferenceDefinition(): NodeDefinition {
  return {
    node_type: 'llm-inference', category: 'processing', label: 'Inference',
    description: 'Run inference', io_binding_origin: 'integrated', execution_mode: 'manual',
    inputs: [
      { id: 'pumas_model_ref', label: 'Model', data_type: 'json', required: true, multiple: false },
      dependencySidecar,
    ],
    outputs: [{ id: 'diagnostics', label: 'Diagnostics', data_type: 'json', required: false, multiple: false }],
  };
}

test('authored inference payload ports retain only the registered dependency control', () => {
  const snapshot = embeddingSnapshot();
  const definition = inferenceDefinition();
  const before = JSON.stringify({ snapshot, definition });
  const resolved = resolveNodeDefinitionOverlay('llm-inference', {
    inference_interface_snapshot: snapshot,
  }, [definition]);
  assert.ok(resolved);
  assert.deepEqual(resolved.inputs.map(port => port.id), ['text', 'dependency_environment_sidecar']);
  assert.equal(resolved.inputs[1], dependencySidecar);
  assert.deepEqual(resolved.outputs.map(port => port.id), ['embedding', 'metadata', 'usage']);
  assert.equal(JSON.stringify({ snapshot, definition }), before);
});

test('an authored dependency control is not duplicated by the registered control', () => {
  const snapshot = embeddingSnapshot();
  snapshot.inputs.push({
    port_id: dependencySidecar.id, label: 'Authored control', direction: 'input',
    requirement: 'optional', value_type: { category: 'structured', kind: 'json' },
    availability: { status: 'available' },
  });
  const resolved = resolveNodeDefinitionOverlay('llm-inference', {
    inference_interface_snapshot: snapshot,
  }, [inferenceDefinition()]);
  assert.ok(resolved);
  assert.equal(resolved.inputs.filter(port => port.id === dependencySidecar.id).length, 1);
  assert.equal(resolved.inputs[1].label, 'Authored control');
});

test('saved dependency edge keeps its target handle after load and definition refresh', () => {
  const nodeDefinitions = writable<NodeDefinition[]>([]);
  const state = createWorkflowStoreGraphState({ nodeDefinitions, selectedNodeIds: writable([]) });
  const graph = JSON.parse(JSON.stringify({
    nodes: [{ id: 'infer', node_type: 'llm-inference', position: { x: 300, y: 100 },
      data: { inference_interface_snapshot: embeddingSnapshot() } }],
    edges: [{ id: 'deps-to-infer', source: 'deps', source_handle: dependencySidecar.id,
      target: 'infer', target_handle: dependencySidecar.id }],
  }));
  state.applyWorkflowGraph(graph);
  nodeDefinitions.set([inferenceDefinition()]);
  const target = get(state.nodes)[0].data.definition as NodeDefinition;
  assert.ok(target.inputs.some(port => port.id === get(state.edges)[0].targetHandle));
  state.updateNodeRuntimeData('infer', { streamContent: 'runtime overlay' });
  nodeDefinitions.set([{ ...inferenceDefinition(), label: 'Refreshed definition' }]);
  const refreshed = get(state.nodes)[0];
  assert.deepEqual((refreshed.data.definition as NodeDefinition).inputs.map(port => port.id),
    ['text', dependencySidecar.id]);
  assert.equal(refreshed.data.streamContent, 'runtime overlay');
  assert.deepEqual(get(state.workflowGraph).edges, graph.edges);
});
