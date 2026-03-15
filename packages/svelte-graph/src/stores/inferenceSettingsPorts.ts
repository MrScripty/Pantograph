import type {
  NodeDefinition,
  PortDefinition,
  PortDataType,
} from '../types/workflow.js';

/** Schema for a model-specific inference parameter (from pumas-library). */
export interface InferenceParamSchema {
  key: string;
  label: string;
  param_type: 'Number' | 'Integer' | 'String' | 'Boolean';
  default: unknown;
  description?: string;
  constraints?: {
    min?: number;
    max?: number;
    allowed_values?: unknown[];
  };
}

function paramTypeToPortDataType(paramType: string): PortDataType {
  switch (paramType) {
    case 'Number':
    case 'Integer':
      return 'number';
    case 'String':
      return 'string';
    case 'Boolean':
      return 'boolean';
    default:
      return 'any';
  }
}

export function inferenceParamToPortDefinition(
  param: InferenceParamSchema,
): PortDefinition {
  return {
    id: param.key,
    label: param.label,
    data_type: paramTypeToPortDataType(param.param_type),
    required: false,
    multiple: false,
    description: param.description,
    default_value: param.default,
    constraints: param.constraints
      ? {
          min: param.constraints.min,
          max: param.constraints.max,
          allowed_values: param.constraints.allowed_values,
        }
      : undefined,
  };
}

function buildDynamicPortMap(
  inferenceSettings: InferenceParamSchema[],
): Map<string, PortDefinition> {
  const dynamicPorts = new Map<string, PortDefinition>();

  for (const param of inferenceSettings) {
    dynamicPorts.set(param.key, inferenceParamToPortDefinition(param));
  }

  return dynamicPorts;
}

function mergeDynamicPorts(
  basePorts: PortDefinition[],
  dynamicPorts: Map<string, PortDefinition>,
): PortDefinition[] {
  return [
    ...basePorts.filter((port) => !dynamicPorts.has(port.id)),
    ...dynamicPorts.values(),
  ];
}

export function buildDynamicInferenceDefinition(
  currentDef: NodeDefinition,
  baseDef: NodeDefinition,
  inferenceSettings: InferenceParamSchema[],
): NodeDefinition {
  const dynamicPorts = buildDynamicPortMap(inferenceSettings);

  return {
    ...currentDef,
    inputs: mergeDynamicPorts(baseDef.inputs, dynamicPorts),
  };
}

export function buildDynamicExpandDefinition(
  currentDef: NodeDefinition,
  baseDef: NodeDefinition,
  inferenceSettings: InferenceParamSchema[],
): NodeDefinition {
  const dynamicPorts = buildDynamicPortMap(inferenceSettings);

  return {
    ...currentDef,
    inputs: mergeDynamicPorts(baseDef.inputs, dynamicPorts),
    outputs: mergeDynamicPorts(baseDef.outputs, dynamicPorts),
  };
}
