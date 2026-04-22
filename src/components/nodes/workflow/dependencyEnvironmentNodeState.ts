import type {
  DependencyOverridePatchV1,
  EnvironmentRef,
  ModelDependencyRequirements,
  ModelDependencyStatus,
} from './dependencyEnvironmentTypes.ts';
import { formatDependencyActivityLine } from './dependencyEnvironmentDisplay.ts';

export type DependencyEnvironmentMode = 'auto' | 'manual';

export interface DependencyEnvironmentNodeDataState {
  mode: DependencyEnvironmentMode;
  selectedBindingIds: string[];
  dependencyRequirements: ModelDependencyRequirements | null;
  dependencyStatus: ModelDependencyStatus | null;
  environmentRef: EnvironmentRef | null;
  manualOverrides: DependencyOverridePatchV1[];
  activityLog: string[];
}

export interface DependencyEnvironmentNodeDataSource {
  mode?: DependencyEnvironmentMode;
  selected_binding_ids?: string[];
  dependency_requirements?: ModelDependencyRequirements;
  dependency_status?: ModelDependencyStatus;
  environment_ref?: EnvironmentRef;
  manual_overrides?: DependencyOverridePatchV1[];
  activity_log?: string[];
}

export function createDependencyEnvironmentNodeDataState(
  data: DependencyEnvironmentNodeDataSource,
): DependencyEnvironmentNodeDataState {
  return {
    mode: data.mode ?? 'auto',
    selectedBindingIds: Array.isArray(data.selected_binding_ids)
      ? data.selected_binding_ids
      : [],
    dependencyRequirements: data.dependency_requirements ?? null,
    dependencyStatus: data.dependency_status ?? null,
    environmentRef: data.environment_ref ?? null,
    manualOverrides: Array.isArray(data.manual_overrides) ? data.manual_overrides : [],
    activityLog: Array.isArray(data.activity_log) ? data.activity_log : [],
  };
}

export function buildDependencyEnvironmentNodeData(
  state: DependencyEnvironmentNodeDataState,
): Record<string, unknown> {
  return {
    mode: state.mode,
    selected_binding_ids: state.selectedBindingIds,
    dependency_requirements: state.dependencyRequirements,
    dependency_status: state.dependencyStatus,
    environment_ref: state.environmentRef,
    manual_overrides: state.manualOverrides,
    dependency_override_patches: state.manualOverrides,
    activity_log: state.activityLog,
  };
}

export function applyDependencyEnvironmentActionNodeData(
  state: DependencyEnvironmentNodeDataState,
  nodeData: Record<string, unknown>,
): DependencyEnvironmentNodeDataState {
  return {
    ...state,
    mode: (nodeData.mode as DependencyEnvironmentMode | undefined) ?? state.mode,
    selectedBindingIds: Array.isArray(nodeData.selected_binding_ids)
      ? (nodeData.selected_binding_ids as string[])
      : state.selectedBindingIds,
    dependencyRequirements:
      (nodeData.dependency_requirements as ModelDependencyRequirements | null | undefined) ??
      state.dependencyRequirements,
    dependencyStatus:
      (nodeData.dependency_status as ModelDependencyStatus | null | undefined) ??
      state.dependencyStatus,
    environmentRef:
      (nodeData.environment_ref as EnvironmentRef | null | undefined) ?? state.environmentRef,
  };
}

export function appendDependencyActivityLogLine(
  activityLog: string[],
  line: string,
  timestamp: string,
  maxLines: number,
): string[] {
  const formatted = formatDependencyActivityLine(line, timestamp);
  if (!formatted) return activityLog;

  const next = [...activityLog, formatted];
  return next.length > maxLines ? next.slice(next.length - maxLines) : next;
}
