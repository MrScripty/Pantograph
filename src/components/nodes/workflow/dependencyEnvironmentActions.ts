import type {
  DependencyEnvironmentActionPayloadInput,
  DependencyEnvironmentActionRequest,
} from './dependencyEnvironmentTypes.ts';

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
