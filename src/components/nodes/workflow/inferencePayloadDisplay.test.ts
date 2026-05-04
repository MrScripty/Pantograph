import test from 'node:test';
import assert from 'node:assert/strict';

import { buildInferencePayloadDisplay } from './inferencePayloadDisplay.ts';
import type {
  InferencePortPayloadContract,
  NodeDefinition,
} from '../../../services/workflow/types.ts';

function llmDefinition(overrides: Partial<NodeDefinition> = {}): NodeDefinition {
  return {
    node_type: 'llm-inference',
    category: 'processing',
    label: 'LLM Inference',
    description: 'Canonical inference node',
    io_binding_origin: 'integrated',
    execution_mode: 'stream',
    inputs: [
      {
        id: 'pumas_model_ref',
        label: 'Pumas Model Ref',
        data_type: 'json',
        required: false,
        multiple: false,
        inference_payloads: [
          { task_id: 'text_generation', role: 'model_reference' },
          { task_id: 'chat_completion', role: 'model_reference' },
        ],
      },
      {
        id: 'generation_options',
        label: 'Generation Options',
        data_type: 'json',
        required: false,
        multiple: false,
        inference_payloads: [{ task_id: 'text_generation', role: 'options' }],
      },
    ],
    outputs: [
      {
        id: 'response',
        label: 'Response',
        data_type: 'string',
        required: false,
        multiple: false,
        inference_payloads: [{ task_id: 'text_generation', role: 'task_output' }],
      },
      {
        id: 'metadata',
        label: 'Metadata',
        data_type: 'json',
        required: false,
        multiple: false,
        inference_payloads: [{ task_id: 'text_generation', role: 'diagnostics' }],
      },
      {
        id: 'diagnostics',
        label: 'Diagnostics',
        data_type: 'json',
        required: false,
        multiple: false,
        inference_payloads: [{ task_id: 'text_generation', role: 'diagnostics' }],
      },
      {
        id: 'usage',
        label: 'Usage',
        data_type: 'json',
        required: false,
        multiple: false,
        inference_payloads: [{ task_id: 'chat_completion', role: 'usage' }],
      },
    ],
    ...overrides,
  };
}

test('buildInferencePayloadDisplay formats task and role labels from inference payloads', () => {
  const display = buildInferencePayloadDisplay(llmDefinition());

  assert.deepEqual(display?.tasks, ['Chat', 'Text']);
  assert.deepEqual(display?.rows, [
    { label: 'Model facts', value: 'Pumas Model Ref' },
    { label: 'Options', value: 'Generation Options' },
    { label: 'Diagnostics', value: 'Diagnostics, Metadata' },
    { label: 'Usage', value: 'Usage' },
  ]);
});

test('buildInferencePayloadDisplay ignores additive backend and runtime payload fields', () => {
  const definition = llmDefinition({
    inputs: [],
    outputs: [
      {
        id: 'diagnostics',
        label: 'Diagnostics',
        data_type: 'json',
        required: false,
        multiple: false,
        inference_payloads: [
          {
            task_id: 'text_generation',
            role: 'diagnostics',
            backend_key: 'pytorch',
            runtime_id: 'runtime-a',
            scheduler_policy: 'reservation-first',
          } as unknown as InferencePortPayloadContract,
        ],
      },
    ],
  });

  const display = buildInferencePayloadDisplay(definition);
  const displayText = JSON.stringify(display);

  assert.equal(displayText.includes('pytorch'), false);
  assert.equal(displayText.includes('runtime-a'), false);
  assert.equal(displayText.includes('reservation-first'), false);
  assert.deepEqual(display?.rows, [{ label: 'Diagnostics', value: 'Diagnostics' }]);
});

test('buildInferencePayloadDisplay ignores unknown payload tasks and empty metadata', () => {
  const definition = llmDefinition({
    inputs: [],
    outputs: [
      {
        id: 'diagnostics',
        label: 'Diagnostics',
        data_type: 'json',
        required: false,
        multiple: false,
        inference_payloads: [{ task_id: 'unknown', role: 'diagnostics' }],
      },
      {
        id: 'plain_json',
        label: 'Plain JSON',
        data_type: 'json',
        required: false,
        multiple: false,
      },
    ],
  });

  const display = buildInferencePayloadDisplay(definition);

  assert.deepEqual(display?.tasks, []);
  assert.deepEqual(display?.rows, [{ label: 'Diagnostics', value: 'Diagnostics' }]);
  assert.equal(buildInferencePayloadDisplay(llmDefinition({ inputs: [], outputs: [] })), null);
  assert.equal(buildInferencePayloadDisplay(undefined), null);
});
