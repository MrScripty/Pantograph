import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildInferenceDriftDisplay,
  buildInferenceUpdateApplyDisplay,
  buildInferenceUpdatePreviewDisplay,
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

test('buildInferenceUpdateApplyDisplay enables only backend replacement proposals', () => {
  assert.deepEqual(
    buildInferenceUpdateApplyDisplay({
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
    }),
    {
      detail: null,
      enabled: true,
      label: 'Apply',
    },
  );
});

test('buildInferenceUpdateApplyDisplay disables destructive proposal operations', () => {
  assert.deepEqual(
    buildInferenceUpdateApplyDisplay({
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
          operation: 'remove_invalid_edge',
          value: {
            edge: {
              edge_id: 'edge-1',
              source_node_id: 'source',
              source_port_id: 'image',
              target_node_id: 'infer-1',
              target_port_id: 'prompt',
            },
            reason: 'target_port_removed',
          },
        },
      ],
      requires_confirmation: true,
      destructive: true,
    }),
    {
      detail: 'Requires preview',
      enabled: false,
      label: 'Review',
    },
  );
});

test('buildInferenceUpdatePreviewDisplay lists backend drift changes without patch construction', () => {
  assert.deepEqual(
    buildInferenceUpdatePreviewDisplay(
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
          {
            kind: 'port_type_changed',
            port_id: 'image',
            message: 'Input image changed type.',
          },
          {
            kind: 'default_changed',
            port_id: 'steps',
            message: 'Steps default changed.',
          },
          {
            kind: 'availability_changed',
            port_id: 'sampler',
            message: 'Sampler availability changed.',
          },
        ],
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
      extraCount: 1,
      operationSummary: '1 backend operation',
      rows: [
        'prompt: Current descriptor added input port prompt.',
        'image: Input image changed type.',
        'steps: Steps default changed.',
      ],
      title: 'Review interface changes',
    },
  );
});
