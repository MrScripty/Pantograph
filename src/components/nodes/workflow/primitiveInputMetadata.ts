import type { NodeDefinition, PortDefinition } from '../../../services/workflow/types';

interface WorkflowEdgeLike {
  source: string;
  sourceHandle?: string | null;
  target: string;
  targetHandle?: string | null;
}

interface WorkflowNodeLike {
  id: string;
  data?: {
    definition?: NodeDefinition;
  } & Record<string, unknown>;
}

export function findConnectedTargetPort(
  nodeId: string,
  sourceHandle: string,
  graphNodes: WorkflowNodeLike[],
  graphEdges: WorkflowEdgeLike[]
): PortDefinition | null {
  const edge = graphEdges.find(
    (candidate) =>
      candidate.source === nodeId && (candidate.sourceHandle ?? null) === sourceHandle
  );
  if (!edge) return null;

  const targetNode = graphNodes.find((node) => node.id === edge.target);
  const definition = targetNode?.data?.definition;
  if (!definition) return null;

  return (
    definition.inputs.find((port) => port.id === (edge.targetHandle ?? null)) ?? null
  );
}

export function normalizePortDefaultValue(value: unknown): unknown {
  if (!value || typeof value !== 'object') return value;
  const record = value as Record<string, unknown>;
  return record.value ?? value;
}

export function parseNumberNodeValue(value: unknown): number | null {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : null;
  }

  if (typeof value === 'string' && value.trim().length > 0) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }

  return null;
}

export function parseBooleanNodeValue(value: unknown): boolean | null {
  if (typeof value === 'boolean') return value;
  if (value === 'true') return true;
  if (value === 'false') return false;
  return null;
}
