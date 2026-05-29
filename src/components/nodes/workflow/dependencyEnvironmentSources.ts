import { parseOverridePatches } from './dependencyEnvironmentOverrides.ts';
import type { DependencyOverridePatchV1 } from './dependencyEnvironmentTypes.ts';

const DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID = 'dependency_environment_sidecar';

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
  hasSidecarAssociation: boolean;
  manualOverrides: DependencyOverridePatchV1[];
}

function hasDependencyEnvironmentSidecarAssociation(
  nodeId: string,
  graphEdges: WorkflowEdgeLike[]
): boolean {
  return graphEdges.some(
    (edge) =>
      edge.source === nodeId &&
      edge.sourceHandle === DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID &&
      edge.targetHandle === DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID
  );
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
  const manualOverridesEdge =
    graphEdges.find((edge) => edge.target === nodeId && edge.targetHandle === 'manual_overrides') ?? null;
  const manualOverridesSourceNode = manualOverridesEdge
    ? graphNodes.find((node) => node.id === manualOverridesEdge.source) ?? null
    : null;

  return {
    hasSidecarAssociation: hasDependencyEnvironmentSidecarAssociation(nodeId, graphEdges),
    manualOverrides: parseManualOverridesFromSource(manualOverridesSourceNode, manualOverridesEdge?.sourceHandle ?? null),
  };
}
