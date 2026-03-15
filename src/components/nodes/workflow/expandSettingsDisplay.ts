interface ParamSchema {
  key: string;
  default: unknown;
}

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

function normalizeSettingValue(value: unknown): unknown {
  if (!value || typeof value !== 'object') return value;
  const record = value as Record<string, unknown>;
  return record.value ?? value;
}

function readConnectedSettingValue(
  nodeId: string,
  key: string,
  graphNodes: WorkflowNodeLike[],
  graphEdges: WorkflowEdgeLike[],
): unknown {
  const edge = graphEdges.find(
    (candidate) =>
      candidate.target === nodeId && (candidate.targetHandle ?? null) === key,
  );
  if (!edge || !edge.sourceHandle) return undefined;

  const sourceNode = graphNodes.find((node) => node.id === edge.source);
  return sourceNode?.data?.[edge.sourceHandle] ?? undefined;
}

export function resolveEffectiveSettingValue(
  nodeId: string,
  nodeData: Record<string, unknown>,
  parameter: ParamSchema,
  graphNodes: WorkflowNodeLike[],
  graphEdges: WorkflowEdgeLike[],
): unknown {
  const connectedValue = normalizeSettingValue(
    readConnectedSettingValue(nodeId, parameter.key, graphNodes, graphEdges),
  );
  if (connectedValue !== undefined && connectedValue !== null) {
    return connectedValue;
  }

  const runtimeValue = normalizeSettingValue(nodeData[parameter.key]);
  if (runtimeValue !== undefined && runtimeValue !== null) {
    return runtimeValue;
  }

  return normalizeSettingValue(parameter.default);
}
