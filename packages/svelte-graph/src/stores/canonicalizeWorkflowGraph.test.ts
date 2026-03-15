import test from 'node:test';
import assert from 'node:assert/strict';

import type { NodeDefinition, WorkflowGraph } from '../types/workflow.js';
import { canonicalizeWorkflowGraph } from './canonicalizeWorkflowGraph.ts';

const definitions: NodeDefinition[] = [
  {
    node_type: 'puma-lib',
    category: 'processing',
    label: 'Puma Lib',
    description: 'Model source',
    io_binding_origin: 'integrated',
    inputs: [],
    outputs: [
      { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'expand-settings',
    category: 'processing',
    label: 'Expand Settings',
    description: 'Expose settings',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
    ],
    outputs: [
      { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'llamacpp-inference',
    category: 'processing',
    label: 'LlamaCpp Inference',
    description: 'Run inference',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false, default_value: 0.7 },
      { id: 'max_tokens', label: 'Max Tokens', data_type: 'number', required: false, multiple: false, default_value: 512 },
      { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'stream',
  },
];

test('canonicalizeWorkflowGraph refreshes legacy expand-settings overlays and missing param edges on load', () => {
  const graph: WorkflowGraph = {
    nodes: [
      {
        id: 'puma-1',
        node_type: 'puma-lib',
        position: { x: 0, y: 0 },
        data: {
          inference_settings: [
            {
              key: 'temperature',
              label: 'Temperature',
              param_type: 'Number',
              default: 0.8,
            },
          ],
        },
      },
      {
        id: 'expand-1',
        node_type: 'expand-settings',
        position: { x: 200, y: 0 },
        data: {
          definition: {
            node_type: 'expand-settings',
            inputs: [
              { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
            ],
            outputs: [
              { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
            ],
          },
        },
      },
      {
        id: 'llama-1',
        node_type: 'llamacpp-inference',
        position: { x: 400, y: 0 },
        data: {
          definition: {
            node_type: 'llamacpp-inference',
            inputs: [
              { id: 'model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
              { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
              { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false },
              { id: 'max_tokens', label: 'Max Tokens', data_type: 'number', required: false, multiple: false },
              { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
            ],
            outputs: [
              { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
            ],
          },
        },
      },
    ],
    edges: [
      {
        id: 'puma-expand-settings',
        source: 'puma-1',
        source_handle: 'inference_settings',
        target: 'expand-1',
        target_handle: 'inference_settings',
      },
      {
        id: 'expand-llama-settings',
        source: 'expand-1',
        source_handle: 'inference_settings',
        target: 'llama-1',
        target_handle: 'inference_settings',
      },
    ],
  };

  const canonicalGraph = canonicalizeWorkflowGraph(graph, definitions);
  const expandNode = canonicalGraph.nodes.find((node) => node.id === 'expand-1');
  const inferenceNode = canonicalGraph.nodes.find((node) => node.id === 'llama-1');

  assert.ok(expandNode, 'expand-settings node should exist');
  assert.ok(inferenceNode, 'inference node should exist');
  assert.deepEqual(
    (expandNode.data.definition as { inputs: Array<{ id: string }> }).inputs.map((port) => port.id),
    ['inference_settings', 'temperature', 'max_tokens'],
  );
  assert.deepEqual(
    (expandNode.data.inference_settings as Array<{ key: string }>).map((param) => param.key),
    ['temperature', 'max_tokens'],
  );
  assert.deepEqual(
    (inferenceNode.data.definition as { inputs: Array<{ id: string }> }).inputs.map((port) => port.id),
    ['model_path', 'prompt', 'inference_settings', 'temperature', 'max_tokens'],
  );
  assert.ok(
    canonicalGraph.edges.some((edge) =>
      edge.source === 'expand-1' &&
      edge.source_handle === 'temperature' &&
      edge.target === 'llama-1' &&
      edge.target_handle === 'temperature'
    ),
    'temperature passthrough edge should be added',
  );
  assert.ok(
    canonicalGraph.edges.some((edge) =>
      edge.source === 'expand-1' &&
      edge.source_handle === 'max_tokens' &&
      edge.target === 'llama-1' &&
      edge.target_handle === 'max_tokens'
    ),
    'inference default passthrough edge should be added',
  );
});

test('canonicalizeWorkflowGraph migrates system-prompt graphs before reconciliation', () => {
  const graph: WorkflowGraph = {
    nodes: [
      {
        id: 'prompt-1',
        node_type: 'system-prompt',
        position: { x: 0, y: 0 },
        data: {
          prompt: 'hello',
        },
      },
    ],
    edges: [],
  };

  const canonicalGraph = canonicalizeWorkflowGraph(graph, definitions);

  assert.equal(canonicalGraph.nodes[0]?.node_type, 'text-input');
  assert.equal(canonicalGraph.nodes[0]?.data.text, 'hello');
  assert.equal(canonicalGraph.nodes[0]?.data.prompt, undefined);
});
