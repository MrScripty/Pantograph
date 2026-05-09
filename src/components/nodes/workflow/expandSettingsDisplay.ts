interface ParamSchema {
  key: string;
  param_type?: string;
  default: unknown;
  constraints?: { allowed_values?: unknown[] };
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

export function normalizeSettingValue(value: unknown): unknown {
  if (!value || typeof value !== 'object') return value;
  const record = value as Record<string, unknown>;
  return record.value ?? value;
}

export interface RuntimeSettingOption {
  label: string;
  value: unknown;
}

function optionLabel(value: unknown): string {
  if (!value || typeof value !== 'object') return String(value ?? '');
  const record = value as Record<string, unknown>;
  const label = record.label ?? record.name ?? record.key ?? record.value;
  return String(label ?? '');
}

export function runtimeSettingOptions(parameter: ParamSchema): RuntimeSettingOption[] {
  const allowedValues = parameter.constraints?.allowed_values;
  if (!Array.isArray(allowedValues)) return [];

  return allowedValues.map((option) => ({
    label: optionLabel(option),
    value: normalizeSettingValue(option),
  }));
}

export function runtimeSettingDraftValue(value: unknown): string {
  const normalized = normalizeSettingValue(value);
  if (normalized === null || normalized === undefined) return '';
  return String(normalized);
}

export function parseRuntimeSettingInput(parameter: ParamSchema, value: string | boolean): unknown {
  if (typeof value === 'boolean') return value;

  const trimmed = value.trim();
  switch (parameter.param_type) {
    case 'Number': {
      if (trimmed === '') return null;
      const parsed = Number(trimmed);
      return Number.isFinite(parsed) ? parsed : null;
    }
    case 'Integer': {
      if (trimmed === '') return null;
      const parsed = Number.parseInt(trimmed, 10);
      return Number.isFinite(parsed) ? parsed : null;
    }
    case 'Boolean':
      return ['true', '1', 'yes', 'on'].includes(trimmed.toLowerCase());
    default:
      return trimmed;
  }
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
