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
  pantograph_origin?: 'inference-default';
  pantograph_owner_node_type?: string;
}

const PROMOTED_INFERENCE_SETTING_PORT_IDS = new Map<string, Set<string>>([
  ['audio-generation', new Set(['duration', 'num_inference_steps', 'guidance_scale', 'seed'])],
  ['diffusion-inference', new Set(['steps', 'cfg_scale', 'seed', 'width', 'height'])],
  ['llamacpp-inference', new Set(['temperature', 'max_tokens'])],
  ['ollama-inference', new Set(['temperature', 'max_tokens'])],
  ['pytorch-inference', new Set(['temperature', 'max_tokens', 'device', 'model_type'])],
  ['reranker', new Set(['top_k', 'return_documents'])],
]);

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

function portDataTypeToParamType(dataType: PortDataType): InferenceParamSchema['param_type'] {
  switch (dataType) {
    case 'boolean':
      return 'Boolean';
    case 'number':
      return 'Number';
    case 'string':
      return 'String';
    default:
      return 'String';
  }
}

function promotedInferenceSettingPortIds(nodeType: string): Set<string> {
  return PROMOTED_INFERENCE_SETTING_PORT_IDS.get(nodeType) ?? new Set();
}

function inferenceDefaultPortToSchema(
  nodeType: string,
  port: PortDefinition,
): InferenceParamSchema {
  return {
    key: port.id,
    label: port.label,
    param_type: portDataTypeToParamType(port.data_type),
    default: port.default_value ?? null,
    description: port.description,
    constraints: port.constraints
      ? {
          min: port.constraints.min,
          max: port.constraints.max,
          allowed_values: port.constraints.allowed_values,
        }
      : undefined,
    pantograph_origin: 'inference-default',
    pantograph_owner_node_type: nodeType,
  };
}

function mergeInferenceSettings(
  upstreamSettings: InferenceParamSchema[],
  appendedSettings: InferenceParamSchema[],
): InferenceParamSchema[] {
  const merged = [...upstreamSettings];
  const seenKeys = new Set(upstreamSettings.map((param) => param.key));

  for (const param of appendedSettings) {
    if (seenKeys.has(param.key)) continue;
    merged.push(param);
    seenKeys.add(param.key);
  }

  return merged;
}

function stripForeignInferenceDefaults(
  nodeType: string,
  inferenceSettings: InferenceParamSchema[],
): InferenceParamSchema[] {
  return inferenceSettings.filter((param) => {
    if (param.pantograph_origin !== 'inference-default') {
      return true;
    }
    return param.pantograph_owner_node_type === nodeType;
  });
}

export function buildMergedInferenceSettings(
  baseDef: NodeDefinition,
  inferenceSettings: InferenceParamSchema[],
): InferenceParamSchema[] {
  const promotedPortIds = promotedInferenceSettingPortIds(baseDef.node_type);
  if (promotedPortIds.size === 0) {
    return stripForeignInferenceDefaults(baseDef.node_type, inferenceSettings);
  }

  const upstreamSettings = stripForeignInferenceDefaults(baseDef.node_type, inferenceSettings);
  const appendedSettings = baseDef.inputs
    .filter((port) => promotedPortIds.has(port.id))
    .map((port) => inferenceDefaultPortToSchema(baseDef.node_type, port));

  return mergeInferenceSettings(upstreamSettings, appendedSettings);
}

export function buildExpandSettingsSchema(
  baseDefs: NodeDefinition[],
  inferenceSettings: InferenceParamSchema[],
): InferenceParamSchema[] {
  return baseDefs.reduce(
    (currentSettings, baseDef) => {
      const promotedPortIds = promotedInferenceSettingPortIds(baseDef.node_type);
      const appendedSettings = baseDef.inputs
        .filter((port) => promotedPortIds.has(port.id))
        .map((port) => inferenceDefaultPortToSchema(baseDef.node_type, port));
      return mergeInferenceSettings(currentSettings, appendedSettings);
    },
    inferenceSettings,
  );
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
  const promotedPortIds = promotedInferenceSettingPortIds(baseDef.node_type);
  const staticInputs = baseDef.inputs.filter((port) => !promotedPortIds.has(port.id));

  return {
    ...currentDef,
    inputs: mergeDynamicPorts(staticInputs, dynamicPorts),
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
