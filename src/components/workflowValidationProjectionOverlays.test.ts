import test from 'node:test';
import assert from 'node:assert/strict';

import type { InferenceInterfaceNodeProjectionRecord } from '../services/workflow/types.ts';
import {
  INFERENCE_INTERFACE_DRIFT_REPORT_RUNTIME_KEY,
  INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY,
  INFERENCE_INTERFACE_UPDATE_PROPOSAL_RUNTIME_KEY,
  INFERENCE_INTERFACE_VALIDATION_SUMMARY_RUNTIME_KEY,
  workflowValidationProjectionOverlays,
} from './workflowValidationProjectionOverlays.ts';

test('workflowValidationProjectionOverlays projects backend inference validation facts for display overlays', () => {
  const projection = {
    node_id: 'infer-1',
    authored_snapshot: {
      contract_version: 1,
      descriptor_fingerprint: 'descriptor.image_generation.1',
      task_kind: 'image_generation',
      inputs: [],
      outputs: [],
    },
    descriptor: {
      contract_version: 1,
      model_ref: {
        model_id: 'diffusion/tiny',
      },
      task_kind: 'image_generation',
      descriptor_fingerprint: 'descriptor.image_generation.1',
      availability: { status: 'available' },
    },
    validation_summary: {
      status: 'executable',
      executable: true,
      diagnostics_count: 0,
      blocking_diagnostics_count: 0,
    },
    drift_report: {
      authored_fingerprint: 'descriptor.previous',
      current_fingerprint: 'descriptor.image_generation.1',
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
    update_proposal: {
      proposal_id: 'inference-interface-update/infer-1',
      node_id: 'infer-1',
      current_descriptor_fingerprint: 'descriptor.image_generation.1',
      drift_report: {
        authored_fingerprint: 'descriptor.previous',
        current_fingerprint: 'descriptor.image_generation.1',
        severity: 'blocking',
        blocking: true,
        diagnostics: [],
      },
      operations: [
        {
          operation: 'replace_authored_snapshot',
          value: {
            node_id: 'infer-1',
            snapshot: {
              contract_version: 1,
              descriptor_fingerprint: 'descriptor.image_generation.1',
              task_kind: 'image_generation',
              inputs: [],
              outputs: [],
            },
          },
        },
      ],
      requires_confirmation: true,
      destructive: false,
    },
  } satisfies InferenceInterfaceNodeProjectionRecord;

  assert.deepEqual(workflowValidationProjectionOverlays([projection]), [
    {
      nodeId: 'infer-1',
      data: {
        [INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY]: projection.authored_snapshot,
        [INFERENCE_INTERFACE_VALIDATION_SUMMARY_RUNTIME_KEY]: projection.validation_summary,
        [INFERENCE_INTERFACE_DRIFT_REPORT_RUNTIME_KEY]: projection.drift_report,
        [INFERENCE_INTERFACE_UPDATE_PROPOSAL_RUNTIME_KEY]: projection.update_proposal,
      },
    },
  ]);
});
