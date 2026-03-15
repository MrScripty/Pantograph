import type { NodeDefinition } from '../types/workflow.ts';

export function resolveNodeDefinitionOverlay(
  nodeType: string,
  nodeData: Record<string, unknown>,
  definitions: NodeDefinition[],
): NodeDefinition | undefined {
  const baseDefinition = definitions.find((d) => d.node_type === nodeType);
  const overlay = nodeData.definition;

  if (!baseDefinition || !overlay || typeof overlay !== 'object') {
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
