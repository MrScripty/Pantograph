import type { InferenceInterfaceUpdateProposal } from '../services/workflow/types.ts';

export const INFERENCE_INTERFACE_UPDATE_COORDINATOR_CONTEXT = Symbol(
  'pantograph.inferenceInterfaceUpdateCoordinator',
);

export interface InferenceInterfaceUpdateCoordinatorRequest {
  proposal: InferenceInterfaceUpdateProposal;
}

export type InferenceInterfaceUpdateCoordinator = (
  request: InferenceInterfaceUpdateCoordinatorRequest,
) => Promise<void>;
