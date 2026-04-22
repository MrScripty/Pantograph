import { parseOverridePatches } from './dependencyEnvironmentOverrides.ts';
import type {
  DependencyOverridePatchV1,
  ModelDependencyRequirements,
} from './dependencyEnvironmentTypes.ts';

interface WorkflowEdgeLike {
  source: string;
  sourceHandle?: string | null;
  target: string;
  targetHandle?: string | null;
}

interface WorkflowNodeLike {
  id: string;
  data?: Record<string, unknown>;
}

export interface DependencyEnvironmentUpstreamState {
  modelPath: string | null;
  modelId: string | null;
  modelType: string | null;
  taskType: string | null;
  backendKey: string | null;
  platformContext: Record<string, string> | null;
  requirements: ModelDependencyRequirements | null;
  manualOverrides: DependencyOverridePatchV1[];
}

function findSourceNode(
  nodeId: string,
  targetHandle: string,
  graphNodes: WorkflowNodeLike[],
  graphEdges: WorkflowEdgeLike[]
): WorkflowNodeLike | null {
  const edge = graphEdges.find((candidate) => candidate.target === nodeId && candidate.targetHandle === targetHandle);
  if (!edge) return null;
  return graphNodes.find((node) => node.id === edge.source) ?? null;
}

function readStringField(data: Record<string, unknown> | undefined, snakeName: string, camelName: string): string | null {
  const value = data?.[snakeName] ?? data?.[camelName] ?? null;
  return typeof value === 'string' ? value : null;
}

function readPlatformContext(data: Record<string, unknown> | undefined): Record<string, string> | null {
  const value = data?.platform_context ?? data?.platformContext ?? null;
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  return value as Record<string, string>;
}

function readRequirements(data: Record<string, unknown> | undefined): ModelDependencyRequirements | null {
  return (data?.dependency_requirements as ModelDependencyRequirements | undefined) ?? null;
}

function parseManualOverridesFromSource(
  sourceNode: WorkflowNodeLike | null,
  sourceHandle: string | null
): DependencyOverridePatchV1[] {
  if (!sourceNode) return [];
  const sourceData = sourceNode.data ?? {};
  const candidates: unknown[] = [];
  if (sourceHandle && sourceHandle.length > 0) candidates.push(sourceData[sourceHandle]);
  candidates.push(
    sourceData.manual_overrides,
    sourceData.manualOverrides,
    sourceData.dependency_override_patches,
    sourceData.dependencyOverridePatches,
    sourceData.output,
    sourceData.value,
    sourceData.json
  );

  for (const candidate of candidates) {
    const parsed = parseOverridePatches(candidate);
    if (parsed.length > 0) return parsed;
  }
  return [];
}

export function resolveDependencyEnvironmentUpstreamState(
  nodeId: string,
  graphNodes: WorkflowNodeLike[],
  graphEdges: WorkflowEdgeLike[]
): DependencyEnvironmentUpstreamState {
  const modelSourceNode = findSourceNode(nodeId, 'model_path', graphNodes, graphEdges);
  const requirementsSourceNode = findSourceNode(nodeId, 'dependency_requirements', graphNodes, graphEdges);
  const manualOverridesEdge =
    graphEdges.find((edge) => edge.target === nodeId && edge.targetHandle === 'manual_overrides') ?? null;
  const manualOverridesSourceNode = manualOverridesEdge
    ? graphNodes.find((node) => node.id === manualOverridesEdge.source) ?? null
    : null;
  const modelData = modelSourceNode?.data;

  return {
    modelPath: readStringField(modelData, 'model_path', 'modelPath'),
    modelId: readStringField(modelData, 'model_id', 'modelId'),
    modelType: readStringField(modelData, 'model_type', 'modelType'),
    taskType: readStringField(modelData, 'task_type_primary', 'taskTypePrimary'),
    backendKey: readStringField(modelData, 'backend_key', 'backendKey'),
    platformContext: readPlatformContext(modelData),
    requirements: readRequirements(requirementsSourceNode?.data) ?? readRequirements(modelData),
    manualOverrides: parseManualOverridesFromSource(manualOverridesSourceNode, manualOverridesEdge?.sourceHandle ?? null),
  };
}
