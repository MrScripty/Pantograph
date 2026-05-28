import test from 'node:test';
import assert from 'node:assert/strict';

import type { InferenceInterfaceNodeProjectionRecord } from '../services/workflow/types.ts';
import {
  INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY,
  workflowValidationProjectionOverlays,
} from './workflowValidationProjectionOverlays.ts';

test('workflowValidationProjectionOverlays projects authored snapshots for display overlays', () => {
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
  } satisfies InferenceInterfaceNodeProjectionRecord;

  assert.deepEqual(workflowValidationProjectionOverlays([projection]), [
    {
      nodeId: 'infer-1',
      data: {
        [INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY]: projection.authored_snapshot,
      },
    },
  ]);
});
