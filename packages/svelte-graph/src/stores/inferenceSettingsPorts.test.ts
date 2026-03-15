import test from 'node:test';
import assert from 'node:assert/strict';

import type { NodeDefinition } from '../types/workflow.js';
import {
  buildExpandSettingsSchema,
  buildDynamicExpandDefinition,
  buildDynamicInferenceDefinition,
  buildMergedInferenceSettings,
  type InferenceParamSchema,
} from './inferenceSettingsPorts.ts';

const settings: InferenceParamSchema[] = [
  {
    key: 'temperature',
    label: 'Temperature',
    param_type: 'Number',
    default: 0.7,
  },
  {
    key: 'voice',
    label: 'Voice',
    param_type: 'String',
    default: 'leo',
  },
];

test('buildDynamicExpandDefinition adds matching dynamic inputs and outputs', () => {
  const baseDef: NodeDefinition = {
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
  };

  const currentDef: NodeDefinition = {
    ...baseDef,
    inputs: [
      ...baseDef.inputs,
      { id: 'stale_setting', label: 'Stale', data_type: 'number', required: false, multiple: false },
    ],
    outputs: [
      ...baseDef.outputs,
      { id: 'stale_setting', label: 'Stale', data_type: 'number', required: false, multiple: false },
    ],
  };

  const nextDef = buildDynamicExpandDefinition(currentDef, baseDef, settings);

  assert.deepEqual(
    nextDef.inputs.map((port) => port.id),
    ['inference_settings', 'temperature', 'voice'],
  );
  assert.deepEqual(
    nextDef.outputs.map((port) => port.id),
    ['inference_settings', 'temperature', 'voice'],
  );
});

test('buildMergedInferenceSettings appends inference-node defaults without duplicating upstream settings', () => {
  const baseDef: NodeDefinition = {
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
  };

  const merged = buildMergedInferenceSettings(baseDef, settings);

  assert.deepEqual(
    merged.map((param) => param.key),
    ['temperature', 'voice', 'max_tokens'],
  );
  assert.equal(merged[0]?.default, 0.7);
  assert.equal(merged[2]?.pantograph_origin, 'inference-default');
});

test('buildExpandSettingsSchema unions downstream inference defaults while preserving upstream precedence', () => {
  const llamaDef: NodeDefinition = {
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
    outputs: [],
    execution_mode: 'stream',
  };
  const diffusionDef: NodeDefinition = {
    node_type: 'diffusion-inference',
    category: 'processing',
    label: 'Diffusion Inference',
    description: 'Generate images',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'steps', label: 'Steps', data_type: 'number', required: false, multiple: false, default_value: 25 },
      { id: 'seed', label: 'Seed', data_type: 'number', required: false, multiple: false, default_value: -1 },
      { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
    ],
    outputs: [],
    execution_mode: 'stream',
  };

  const merged = buildExpandSettingsSchema([llamaDef, diffusionDef], settings);

  assert.deepEqual(
    merged.map((param) => param.key),
    ['temperature', 'voice', 'max_tokens', 'steps', 'seed'],
  );
  assert.equal(merged[2]?.pantograph_owner_node_type, 'llamacpp-inference');
  assert.equal(merged[3]?.pantograph_owner_node_type, 'diffusion-inference');
});

test('buildDynamicInferenceDefinition promotes settings into the inference_settings surface', () => {
  const baseDef: NodeDefinition = {
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
  };

  const currentDef: NodeDefinition = {
    ...baseDef,
    inputs: [
      ...baseDef.inputs,
      { id: 'stale_setting', label: 'Stale', data_type: 'number', required: false, multiple: false },
    ],
  };

  const nextDef = buildDynamicInferenceDefinition(
    currentDef,
    baseDef,
    buildMergedInferenceSettings(baseDef, settings),
  );

  assert.deepEqual(
    nextDef.inputs.map((port) => port.id),
    ['model_path', 'prompt', 'inference_settings', 'temperature', 'voice', 'max_tokens'],
  );
  assert.equal(nextDef.outputs[0]?.id, 'response');
});
