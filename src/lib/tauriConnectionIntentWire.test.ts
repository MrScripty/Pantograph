import test from 'node:test';
import assert from 'node:assert/strict';

import {
  normalizeConnectionCandidatesResponse,
  normalizeConnectionCommitResponse,
  normalizeEdgeInsertionPreviewResponse,
  normalizeInsertNodeConnectionResponse,
  normalizeInsertNodeOnEdgeResponse,
  serializeConnectionAnchor,
} from './tauriConnectionIntentWire.ts';

test('serializeConnectionAnchor converts snake_case to camelCase', () => {
  assert.deepEqual(
    serializeConnectionAnchor({ node_id: 'source-node', port_id: 'output' }),
    { nodeId: 'source-node', portId: 'output' },
  );
});

test('normalizeConnectionCandidatesResponse converts camelCase payloads', () => {
  const response = normalizeConnectionCandidatesResponse({
    graphRevision: 'rev-1',
    revisionMatches: true,
    sourceAnchor: { nodeId: 'source-node', portId: 'output' },
    compatibleNodes: [
      {
        nodeId: 'target-node',
        nodeType: 'text-output',
        nodeLabel: 'Target',
        position: { x: 10, y: 20 },
        anchors: [
          {
            portId: 'input',
            portLabel: 'Input',
            dataType: 'string',
            multiple: false,
          },
        ],
      },
    ],
    insertableNodeTypes: [
      {
        nodeType: 'text-output',
        category: 'output',
        label: 'Text Output',
        description: 'Display text',
        matchingInputPortIds: ['input'],
      },
    ],
  });

  assert.deepEqual(response, {
    graph_revision: 'rev-1',
    revision_matches: true,
    source_anchor: { node_id: 'source-node', port_id: 'output' },
    compatible_nodes: [
      {
        node_id: 'target-node',
        node_type: 'text-output',
        node_label: 'Target',
        position: { x: 10, y: 20 },
        anchors: [
          {
            port_id: 'input',
            port_label: 'Input',
            data_type: 'string',
            multiple: false,
          },
        ],
      },
    ],
    insertable_node_types: [
      {
        node_type: 'text-output',
        category: 'output',
        label: 'Text Output',
        description: 'Display text',
        matching_input_port_ids: ['input'],
      },
    ],
  });
});

test('normalizeConnectionCommitResponse converts camelCase payloads', () => {
  assert.deepEqual(
    normalizeConnectionCommitResponse({
      accepted: false,
      graphRevision: 'rev-2',
      workflowEvent: {
        type: 'GraphModified',
        data: {
          workflow_id: 'session-1',
          execution_id: 'session-1',
          dirty_tasks: ['target-node'],
        },
      },
      workflowSessionState: {
        contract_version: 1,
        residency: 'active',
      },
      rejection: {
        reason: 'incompatible_types',
        message: 'Types are incompatible',
      },
    }),
    {
      accepted: false,
      graph_revision: 'rev-2',
      graph: undefined,
      workflow_event: {
        type: 'GraphModified',
        data: {
          workflow_id: 'session-1',
          execution_id: 'session-1',
          dirty_tasks: ['target-node'],
        },
      },
      workflow_session_state: {
        contract_version: 1,
        residency: 'active',
      },
      rejection: {
        reason: 'incompatible_types',
        message: 'Types are incompatible',
      },
    },
  );
});

test('normalizeInsertNodeConnectionResponse converts camelCase payloads', () => {
  assert.deepEqual(
    normalizeInsertNodeConnectionResponse({
      accepted: true,
      graphRevision: 'rev-3',
      insertedNodeId: 'new-node',
      workflowSessionState: {
        contract_version: 1,
        residency: 'warm',
      },
    }),
    {
      accepted: true,
      graph_revision: 'rev-3',
      inserted_node_id: 'new-node',
      graph: undefined,
      workflow_event: undefined,
      workflow_session_state: {
        contract_version: 1,
        residency: 'warm',
      },
      rejection: undefined,
    },
  );
});

test('normalizeEdgeInsertionPreviewResponse converts camelCase payloads', () => {
  assert.deepEqual(
    normalizeEdgeInsertionPreviewResponse({
      accepted: true,
      graphRevision: 'rev-4',
      bridge: {
        inputPortId: 'prompt',
        outputPortId: 'response',
      },
    }),
    {
      accepted: true,
      graph_revision: 'rev-4',
      bridge: {
        input_port_id: 'prompt',
        output_port_id: 'response',
      },
      rejection: undefined,
    },
  );
});

test('normalizeInsertNodeOnEdgeResponse converts camelCase payloads', () => {
  assert.deepEqual(
    normalizeInsertNodeOnEdgeResponse({
      accepted: true,
      graphRevision: 'rev-5',
      insertedNodeId: 'inserted-node',
      workflowEvent: {
        type: 'GraphModified',
        data: {
          workflow_id: 'session-2',
          execution_id: 'session-2',
          dirty_tasks: ['inserted-node', 'target'],
        },
      },
      bridge: {
        inputPortId: 'prompt',
        outputPortId: 'response',
      },
    }),
    {
      accepted: true,
      graph_revision: 'rev-5',
      inserted_node_id: 'inserted-node',
      bridge: {
        input_port_id: 'prompt',
        output_port_id: 'response',
      },
      graph: undefined,
      workflow_event: {
        type: 'GraphModified',
        data: {
          workflow_id: 'session-2',
          execution_id: 'session-2',
          dirty_tasks: ['inserted-node', 'target'],
        },
      },
      workflow_session_state: undefined,
      rejection: undefined,
    },
  );
});
