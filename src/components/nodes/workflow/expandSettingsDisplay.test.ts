import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveEffectiveSettingValue } from './expandSettingsDisplay.ts';

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
