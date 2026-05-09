import test from 'node:test';
import assert from 'node:assert/strict';

import {
  parseRuntimeSettingInput,
  resolveEffectiveSettingValue,
  runtimeSettingDraftValue,
  runtimeSettingOptions,
} from './expandSettingsDisplay.ts';

test('resolveEffectiveSettingValue falls back to the schema default', () => {
  const value = resolveEffectiveSettingValue(
    'expand-1',
    {},
    { key: 'temperature', default: 0.7 },
    [],
    [],
  );

  assert.equal(value, 0.7);
});

test('resolveEffectiveSettingValue prefers a connected source-node value', () => {
  const value = resolveEffectiveSettingValue(
    'expand-1',
    { temperature: 0.5 },
    { key: 'temperature', default: 0.7 },
    [
      {
        id: 'number-1',
        data: { value: 1.2 },
      },
    ],
    [
      {
        source: 'number-1',
        sourceHandle: 'value',
        target: 'expand-1',
        targetHandle: 'temperature',
      },
    ],
  );

  assert.equal(value, 1.2);
});

test('resolveEffectiveSettingValue uses runtime passthrough data when no live override source is available', () => {
  const value = resolveEffectiveSettingValue(
    'expand-1',
    { temperature: 0.9 },
    { key: 'temperature', default: 0.7 },
    [],
    [],
  );

  assert.equal(value, 0.9);
});

test('resolveEffectiveSettingValue unwraps option objects to their runtime values', () => {
  const value = resolveEffectiveSettingValue(
    'expand-1',
    {},
    {
      key: 'voice',
      default: { label: 'Leo', value: 'expr-voice-5-m' },
    },
    [],
    [],
  );

  assert.equal(value, 'expr-voice-5-m');
});

test('runtimeSettingOptions normalizes allowed option objects to runtime values', () => {
  const options = runtimeSettingOptions({
    key: 'device',
    default: 'auto',
    constraints: {
      allowed_values: [
        { label: 'Auto', value: 'auto' },
        { name: 'CUDA 0', value: 'CUDA0' },
      ],
    },
  });

  assert.deepEqual(options, [
    { label: 'Auto', value: 'auto' },
    { label: 'CUDA 0', value: 'CUDA0' },
  ]);
});

test('parseRuntimeSettingInput preserves typed runtime setting values', () => {
  assert.equal(parseRuntimeSettingInput({ key: 'gpu_layers', param_type: 'Integer', default: -1 }, '42'), 42);
  assert.equal(parseRuntimeSettingInput({ key: 'temperature', param_type: 'Number', default: 0.7 }, '0.25'), 0.25);
  assert.equal(parseRuntimeSettingInput({ key: 'stream', param_type: 'Boolean', default: false }, true), true);
  assert.equal(parseRuntimeSettingInput({ key: 'device', param_type: 'String', default: 'auto' }, 'CUDA0'), 'CUDA0');
});

test('runtimeSettingDraftValue unwraps option objects for editable controls', () => {
  assert.equal(runtimeSettingDraftValue({ label: 'All layers', value: -1 }), '-1');
});
