import test from 'node:test';
import assert from 'node:assert/strict';

import { buildInferenceValidationDisplay } from './inferenceValidationDisplay.ts';

test('buildInferenceValidationDisplay formats executable validation summary', () => {
  assert.deepEqual(
    buildInferenceValidationDisplay({
      status: 'executable',
      executable: true,
      diagnostics_count: 0,
      blocking_diagnostics_count: 0,
    }),
    {
      label: 'Executable',
      detail: null,
      tone: 'success',
    },
  );
});

test('buildInferenceValidationDisplay prioritizes blocking diagnostics', () => {
  assert.deepEqual(
    buildInferenceValidationDisplay({
      status: 'blocked',
      executable: false,
      enqueue_disabled_reasons: ['blocking_diagnostics', 'invalid_port_binding'],
      diagnostics_count: 3,
      blocking_diagnostics_count: 2,
    }),
    {
      label: 'Validation blocked',
      detail: '2 blocking diagnostics',
      tone: 'error',
    },
  );
});

test('buildInferenceValidationDisplay formats pending queue blocks without inferring submit authority', () => {
  assert.deepEqual(
    buildInferenceValidationDisplay({
      status: 'pending',
      executable: false,
      enqueue_disabled_reasons: ['validation_pending'],
      diagnostics_count: 0,
      blocking_diagnostics_count: 0,
    }),
    {
      label: 'Pending validation',
      detail: '1 queue block',
      tone: 'info',
    },
  );
});
