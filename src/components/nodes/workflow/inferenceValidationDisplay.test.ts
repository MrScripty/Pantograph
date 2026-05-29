import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildInferenceDriftDisplay,
  buildInferenceValidationDisplay,
} from './inferenceValidationDisplay.ts';

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

test('buildInferenceDriftDisplay prefers backend update proposal counts', () => {
  assert.deepEqual(
    buildInferenceDriftDisplay(
      {
        authored_fingerprint: 'descriptor.previous',
        current_fingerprint: 'descriptor.current',
        severity: 'blocking',
        blocking: true,
        changes: [
          {
            kind: 'port_added',
            port_id: 'prompt',
            message: 'Current descriptor added input port prompt.',
          },
        ],
        diagnostics: [],
      },
      {
        proposal_id: 'inference-interface-update/infer-1',
        node_id: 'infer-1',
        current_descriptor_fingerprint: 'descriptor.current',
        drift_report: {
          authored_fingerprint: 'descriptor.previous',
          current_fingerprint: 'descriptor.current',
          severity: 'blocking',
          blocking: true,
        },
        operations: [
          {
            operation: 'replace_authored_snapshot',
            value: {
              node_id: 'infer-1',
              snapshot: {
                descriptor_fingerprint: 'descriptor.current',
                task_kind: 'image_generation',
              },
            },
          },
        ],
        requires_confirmation: true,
        destructive: false,
      },
    ),
    {
      label: 'Interface drift',
      detail: '1 proposed update',
      tone: 'error',
    },
  );
});

test('buildInferenceDriftDisplay falls back to drift change counts', () => {
  assert.deepEqual(
    buildInferenceDriftDisplay(
      {
        authored_fingerprint: 'descriptor.previous',
        current_fingerprint: 'descriptor.current',
        severity: 'non_blocking',
        blocking: false,
        changes: [
          {
            kind: 'availability_changed',
            port_id: 'prompt',
            message: 'Prompt availability changed.',
          },
          {
            kind: 'default_changed',
            port_id: 'steps',
            message: 'Steps default changed.',
          },
        ],
        diagnostics: [],
      },
      null,
    ),
    {
      label: 'Interface changed',
      detail: '2 interface changes',
      tone: 'warning',
    },
  );
});
