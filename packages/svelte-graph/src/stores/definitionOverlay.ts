import type { NodeDefinition, PortDataType, PortDefinition } from '../types/workflow.ts';

const GENERIC_INFERENCE_NODE_TYPE = 'llm-inference';
const INFERENCE_INTERFACE_SNAPSHOT_FIELD = 'inference_interface_snapshot';

export function resolveNodeDefinitionOverlay(
  nodeType: string,
  nodeData: Record<string, unknown>,
  definitions: NodeDefinition[],
): NodeDefinition | undefined {
  const baseDefinition = definitions.find((d) => d.node_type === nodeType);
  if (!baseDefinition) {
    return undefined;
  }

  if (nodeType === GENERIC_INFERENCE_NODE_TYPE) {
    const snapshot = readAuthoredInferenceInterfaceSnapshot(
      nodeData[INFERENCE_INTERFACE_SNAPSHOT_FIELD],
    );
    if (!snapshot) {
      return baseDefinition;
    }

    return {
      ...baseDefinition,
      inputs: snapshot.inputs,
      outputs: snapshot.outputs,
    };
  }

  const overlay = nodeData.definition;

  if (!overlay || typeof overlay !== 'object') {
    return baseDefinition;
  }

  const overlayDef = overlay as Partial<NodeDefinition>;
  if (overlayDef.node_type !== nodeType) {
    return baseDefinition;
  }

  return {
    ...baseDefinition,
    ...(Array.isArray(overlayDef.inputs) ? { inputs: overlayDef.inputs } : {}),
    ...(Array.isArray(overlayDef.outputs) ? { outputs: overlayDef.outputs } : {}),
  };
}

interface AuthoredInferenceInterfacePorts {
  inputs: PortDefinition[];
  outputs: PortDefinition[];
}

interface AuthoredInferencePortRecord {
  port_id?: unknown;
  label?: unknown;
  direction?: unknown;
  requirement?: unknown;
  value_type?: unknown;
  default?: unknown;
}

function readAuthoredInferenceInterfaceSnapshot(
  value: unknown,
): AuthoredInferenceInterfacePorts | null {
  if (!value || typeof value !== 'object') {
    return null;
  }

  const record = value as Record<string, unknown>;
  return {
    inputs: readAuthoredInferencePorts(record.inputs, 'input'),
    outputs: readAuthoredInferencePorts(record.outputs, 'output'),
  };
}

function readAuthoredInferencePorts(
  value: unknown,
  direction: 'input' | 'output',
): PortDefinition[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .map((port) => authoredInferencePortToDefinition(port, direction))
    .filter((port): port is PortDefinition => port !== null);
}

function authoredInferencePortToDefinition(
  value: unknown,
  direction: 'input' | 'output',
): PortDefinition | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  const port = value as AuthoredInferencePortRecord;
  if (port.direction !== direction || typeof port.port_id !== 'string') {
    return null;
  }

  return {
    id: port.port_id,
    label: typeof port.label === 'string' ? port.label : port.port_id,
    data_type: inferenceValueTypeToPortDataType(port.value_type),
    required: port.requirement === 'required',
    multiple: false,
    ...(typeof port.default === 'undefined' ? {} : { default_value: port.default }),
  };
}

function inferenceValueTypeToPortDataType(value: unknown): PortDataType {
  if (!value || typeof value !== 'object') {
    return 'json';
  }

  const record = value as Record<string, unknown>;
  switch (record.category) {
    case 'scalar':
      return scalarInferenceTypeToPortDataType(record.kind);
    case 'artifact':
      return artifactInferenceTypeToPortDataType(record.kind);
    case 'reference':
      return referenceInferenceTypeToPortDataType(record.kind);
    case 'constraint':
      return 'string';
    default:
      return 'json';
  }
}

function scalarInferenceTypeToPortDataType(kind: unknown): PortDataType {
  if (kind === 'bool') {
    return 'boolean';
  }
  if (kind === 'i64' || kind === 'u64' || kind === 'f64') {
    return 'number';
  }
  return 'string';
}

function artifactInferenceTypeToPortDataType(kind: unknown): PortDataType {
  if (kind === 'image') {
    return 'image';
  }
  if (kind === 'audio') {
    return 'audio';
  }
  if (kind === 'document') {
    return 'document';
  }
  return 'json';
}

function referenceInferenceTypeToPortDataType(kind: unknown): PortDataType {
  if (kind === 'scheduler_task_result') {
    return 'any';
  }
  return 'json';
}
