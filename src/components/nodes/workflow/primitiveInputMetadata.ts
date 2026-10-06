import type { NodeDefinition, PortDefinition } from '../../../services/workflow/types';

interface WorkflowEdgeLike {
  source: string;
  sourceHandle?: string | null;
  source_handle?: string | null;
  target: string;
  targetHandle?: string | null;
  target_handle?: string | null;
}

interface WorkflowNodeLike {
  id: string;
  type?: string;
  node_type?: string;
  data?: {
    definition?: NodeDefinition;
  } & Record<string, unknown>;
}

export const DESKTOP_SEED_MAX = Number.MAX_SAFE_INTEGER;
export const DESKTOP_SEED_ERROR = `Seed must be a whole number between 0 and ${DESKTOP_SEED_MAX}.`;

function nodeType(node: WorkflowNodeLike): string | undefined {
  return node.type ?? node.node_type ?? node.data?.definition?.node_type;
}

export function hasInferenceSeedTarget(
  nodeId: string,
  graphNodes: WorkflowNodeLike[],
  graphEdges: WorkflowEdgeLike[],
): boolean {
  return graphEdges.some((edge) =>
    edge.source === nodeId
      && (edge.sourceHandle ?? edge.source_handle) === 'value'
      && (edge.targetHandle ?? edge.target_handle) === 'seed'
      && graphNodes.some((node) => node.id === edge.target && nodeType(node) === 'llm-inference')
  );
}

export interface NumberInputState {
  value: number | null;
  error: string | null;
}

export function parseDesktopSeedValue(value: unknown): NumberInputState {
  if (value === undefined || value === null || (typeof value === 'string' && value.trim() === '')) {
    return { value: null, error: null };
  }
  // Check authored text before conversion: large fractions can round to a safe integer.
  if (typeof value === 'string' && !/^\d+$/.test(value.trim())) {
    return { value: null, error: DESKTOP_SEED_ERROR };
  }
  const parsed = typeof value === 'string' ? Number(value) : value;
  if (typeof parsed !== 'number' || !Number.isSafeInteger(parsed) || parsed < 0) {
    return { value: null, error: DESKTOP_SEED_ERROR };
  }
  return { value: parsed, error: null };
}

export function numberInputState(value: unknown, seed: boolean): NumberInputState {
  return seed ? parseDesktopSeedValue(value) : { value: parseNumberNodeValue(value), error: null };
}

export function numberInputPersistedValue(text: string, seed: boolean): number | string | null {
  const state = numberInputState(text, seed);
  // Invalid seed text remains authored data, so saving cannot round or omit it.
  return state.error === null ? state.value : text;
}

export function numberInputDisplayValue(value: unknown, seed: boolean): string {
  const state = numberInputState(value, seed);
  if (state.error !== null) return String(value);
  return state.value === null ? '' : String(state.value);
}

export function desktopSeedInputError(graph: {
  nodes: WorkflowNodeLike[];
  edges: WorkflowEdgeLike[];
}): string | null {
  for (const node of graph.nodes) {
    if (nodeType(node) !== 'number-input' || !hasInferenceSeedTarget(node.id, graph.nodes, graph.edges)) {
      continue;
    }
    const state = parseDesktopSeedValue(node.data?.value);
    if (state.error !== null) return `Number input '${node.id}': ${state.error}`;
  }
  return null;
}

export function assertDesktopSeedInputs(graph: Parameters<typeof desktopSeedInputError>[0]): void {
  const error = desktopSeedInputError(graph);
  if (error !== null) throw new Error(error);
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
