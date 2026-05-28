import type { InferenceInterfaceNodeProjectionRecord } from '../services/workflow/types';

export const INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY = 'inference_interface_snapshot';

export interface WorkflowValidationProjectionOverlay {
  nodeId: string;
  data: Record<string, unknown>;
}

export function workflowValidationProjectionOverlays(
  nodeProjections: readonly InferenceInterfaceNodeProjectionRecord[],
): WorkflowValidationProjectionOverlay[] {
  return nodeProjections.map((projection) => ({
    nodeId: projection.node_id,
    data: {
      [INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY]: projection.authored_snapshot,
    },
  }));
}
