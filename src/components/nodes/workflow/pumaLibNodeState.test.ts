import test from 'node:test';
import assert from 'node:assert/strict';
import type { PortOption } from '../../../services/workflow/types.ts';
import {
  buildPumaLibSelectionNodeData,
  findPumasModelOptionById,
  isSelectablePumasModelOption,
  pumasModelIdFromOption,
  pumasModelRefFromOption,
} from './pumaLibNodeState.ts';

function option(overrides: Partial<PortOption> = {}): PortOption {
  return {
    value: {
      model_id: 'image/stable-diffusion/tiny-sd',
      revision: 'main',
      selected_artifact_id: 'artifact-diffusers',
    },
    label: 'Tiny SD',
    metadata: {
      id: 'image/stable-diffusion/tiny-sd',
      pumas_model_ref: {
        model_id: 'image/stable-diffusion/tiny-sd',
        revision: 'main',
        selected_artifact_id: 'artifact-diffusers',
      },
    },
    ...overrides,
  };
}

test('buildPumaLibSelectionNodeData persists only canonical Pumas identity', () => {
  const nodeData = buildPumaLibSelectionNodeData(option());

  assert.deepEqual(nodeData, {
    modelName: 'Tiny SD',
    model_id: 'image/stable-diffusion/tiny-sd',
    pumas_model_ref: {
      model_id: 'image/stable-diffusion/tiny-sd',
      revision: 'main',
      selected_artifact_id: 'artifact-diffusers',
    },
  });
  assert.equal(Object.hasOwn(nodeData, 'modelPath'), false);
  assert.equal(Object.hasOwn(nodeData, 'model_path'), false);
  assert.equal(Object.hasOwn(nodeData, 'dependency_requirements'), false);
});

test('Puma-Lib option helpers reject path-shaped legacy values', () => {
  const legacyPathOption = option({
    value: '/models/tiny-sd',
    metadata: { id: 'image/stable-diffusion/tiny-sd' },
  });

  assert.equal(pumasModelRefFromOption(legacyPathOption), null);
  assert.equal(isSelectablePumasModelOption(legacyPathOption), false);
  assert.throws(
    () => buildPumaLibSelectionNodeData(legacyPathOption),
    /canonical pumas_model_ref identity/,
  );
});

test('Puma-Lib option lookup uses model id instead of display value', () => {
  const selected = option();
  const other = option({
    label: 'Other',
    metadata: {
      id: 'llm/imported/other',
      pumas_model_ref: { model_id: 'llm/imported/other' },
    },
    value: { model_id: 'llm/imported/other' },
  });

  assert.equal(
    pumasModelIdFromOption(selected),
    'image/stable-diffusion/tiny-sd',
  );
  assert.equal(
    findPumasModelOptionById([other, selected], 'image/stable-diffusion/tiny-sd'),
    selected,
  );
});
