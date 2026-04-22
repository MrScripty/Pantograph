import test from 'node:test';
import assert from 'node:assert/strict';

import type { NodeDefinition } from '../types/workflow.ts';
import { buildDefaultWorkflowGraphState } from './defaultWorkflowGraph.ts';

function definition(nodeType: string, label: string): NodeDefinition {
  return {
    node_type: nodeType,
    category: nodeType === 'text-output' ? 'output' : 'input',
    label,
    description: label,
    io_binding_origin: 'client_session',
    inputs: [],
    outputs: [],
    execution_mode: 'manual',
  };
}

test('buildDefaultWorkflowGraphState builds the default starter graph', () => {
  const textInput = definition('text-input', 'Text Input');
  const llm = definition('llm-inference', 'LLM Inference');
  const output = definition('text-output', 'Text Output');

  const state = buildDefaultWorkflowGraphState([textInput, llm, output]);

  assert.deepEqual(
    state.nodes.map((node) => [node.id, node.type, node.position]),
    [
      ['user-input', 'text-input', { x: 50, y: 150 }],
      ['llm', 'llm-inference', { x: 350, y: 150 }],
      ['output', 'text-output', { x: 650, y: 150 }],
    ],
  );
  assert.deepEqual(
    state.edges.map((edge) => [edge.id, edge.source, edge.sourceHandle, edge.target, edge.targetHandle]),
    [
      ['input-to-llm', 'user-input', 'text', 'llm', 'prompt'],
      ['llm-to-output', 'llm', 'response', 'output', 'text'],
    ],
  );
  assert.equal(state.nodes[0].data.definition, textInput);
  assert.equal(state.nodes[1].data.definition, llm);
  assert.equal(state.nodes[2].data.definition, output);
  assert.deepEqual(
    state.graph.edges.map((edge) => [
      edge.id,
      edge.source_handle,
      edge.target_handle,
    ]),
    [
      ['input-to-llm', 'text', 'prompt'],
      ['llm-to-output', 'response', 'text'],
    ],
  );
});
