import type {
  DependencyEnvironmentAction,
  DependencyEnvironmentActionIntentResult,
} from '../services/workflow/dependencyEnvironmentActionIntent.ts';

export const DEPENDENCY_ENVIRONMENT_ACTION_COORDINATOR_CONTEXT =
  Symbol('pantograph.dependencyEnvironmentActionCoordinator');

export interface DependencyEnvironmentActionCoordinatorRequest {
  targetNodeId: string;
  action: DependencyEnvironmentAction;
}

export type DependencyEnvironmentActionCoordinator = (
  request: DependencyEnvironmentActionCoordinatorRequest,
) => Promise<DependencyEnvironmentActionIntentResult>;
