import type {
  DependencyEnvironmentActionPayloadInput,
  DependencyEnvironmentActionRequest,
  DependencyEnvironmentActionResponse,
} from './dependencyEnvironmentTypes.ts';

export type DependencyEnvironmentActionInvoker = (
  request: DependencyEnvironmentActionRequest,
) => Promise<DependencyEnvironmentActionResponse>;

export interface DependencyEnvironmentActionRunnerInput {
  action: DependencyEnvironmentActionRequest['action'];
  payload: DependencyEnvironmentActionRequest | null;
  invokeAction: DependencyEnvironmentActionInvoker;
  applyNodeData: (nodeData: Record<string, unknown>) => void;
  appendActivityLine: (line: string) => void;
  setBusy: (busy: boolean) => void;
}

export function buildDependencyEnvironmentActionPayload({
  action,
  mode,
  upstreamModelPath,
  upstreamModelId,
  upstreamModelType,
  upstreamTaskType,
  upstreamBackendKey,
  upstreamPlatformContext,
  selectedBindingIds,
  upstreamRequirements,
  dependencyRequirements,
  effectiveManualOverrides,
}: DependencyEnvironmentActionPayloadInput): DependencyEnvironmentActionRequest | null {
  const modelPath = (upstreamModelPath ?? '').trim();
  if (!modelPath) return null;
  return {
    action,
    mode,
    modelPath,
    modelId: upstreamModelId ?? dependencyRequirements?.model_id ?? undefined,
    modelType: upstreamModelType ?? undefined,
    taskTypePrimary: upstreamTaskType ?? undefined,
    backendKey: upstreamBackendKey ?? dependencyRequirements?.backend_key ?? undefined,
    platformContext: upstreamPlatformContext ?? undefined,
    selectedBindingIds,
    dependencyRequirements: upstreamRequirements ?? dependencyRequirements ?? undefined,
    dependencyOverridePatches: effectiveManualOverrides.length > 0 ? effectiveManualOverrides : undefined,
  };
}

export function formatDependencyEnvironmentActionError(
  action: DependencyEnvironmentActionRequest['action'],
  error: unknown,
): string {
  const message = error instanceof Error ? error.message : String(error);
  return `${action}: error="${message}"`;
}

export async function runDependencyEnvironmentActionRequest({
  action,
  payload,
  invokeAction,
  applyNodeData,
  appendActivityLine,
  setBusy,
}: DependencyEnvironmentActionRunnerInput): Promise<boolean> {
  if (!payload) return false;

  setBusy(true);
  try {
    const response = await invokeAction(payload);
    applyNodeData(response.nodeData);
    return true;
  } catch (error) {
    appendActivityLine(formatDependencyEnvironmentActionError(action, error));
    throw error;
  } finally {
    setBusy(false);
  }
}
