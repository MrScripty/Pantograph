import type { InferenceInterfaceNodeProjectionRecord } from '../services/workflow/types';

export const INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY = 'inference_interface_snapshot';
export const INFERENCE_INTERFACE_VALIDATION_SUMMARY_RUNTIME_KEY =
  'inference_interface_validation_summary';
export const INFERENCE_INTERFACE_DRIFT_REPORT_RUNTIME_KEY =
  'inference_interface_drift_report';
export const INFERENCE_INTERFACE_UPDATE_PROPOSAL_RUNTIME_KEY =
  'inference_interface_update_proposal';

export const INFERENCE_INTERFACE_VALIDATION_RUNTIME_KEYS = [
  INFERENCE_INTERFACE_SNAPSHOT_RUNTIME_KEY,
  INFERENCE_INTERFACE_VALIDATION_SUMMARY_RUNTIME_KEY,
  INFERENCE_INTERFACE_DRIFT_REPORT_RUNTIME_KEY,
  INFERENCE_INTERFACE_UPDATE_PROPOSAL_RUNTIME_KEY,
] as const;

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
      [INFERENCE_INTERFACE_VALIDATION_SUMMARY_RUNTIME_KEY]: projection.validation_summary,
      [INFERENCE_INTERFACE_DRIFT_REPORT_RUNTIME_KEY]: projection.drift_report ?? null,
      [INFERENCE_INTERFACE_UPDATE_PROPOSAL_RUNTIME_KEY]: projection.update_proposal ?? null,
    },
  }));
}
