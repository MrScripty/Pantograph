import test from 'node:test';
import assert from 'node:assert/strict';

import type { NodeDefinition } from '../types/workflow.js';
import {
  buildDynamicExpandDefinition,
  buildDynamicInferenceDefinition,
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

test('buildDynamicInferenceDefinition preserves static inputs while replacing dynamic ones', () => {
  const baseDef: NodeDefinition = {
    node_type: 'llamacpp-inference',
    category: 'processing',
    label: 'LlamaCpp Inference',
    description: 'Run inference',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
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

  const nextDef = buildDynamicInferenceDefinition(currentDef, baseDef, settings);

  assert.deepEqual(
    nextDef.inputs.map((port) => port.id),
    ['model_path', 'prompt', 'inference_settings', 'temperature', 'voice'],
  );
  assert.equal(nextDef.outputs[0]?.id, 'response');
});
