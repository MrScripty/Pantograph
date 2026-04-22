import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildConnectionIntentState,
  edgeToGraphEdge,
  isWorkflowConnectionValid,
  preserveConnectionIntentState,
  resolveConnectionCommitGraphRevision,
  resolveWorkflowConnectionAnchors,
} from './workflowConnections.ts';
import type { NodeDefinition } from './types/workflow.ts';

const sourceDefinition: NodeDefinition = {
  node_type: 'source',
  category: 'input',
  label: 'Source',
  description: 'Source node',
  io_binding_origin: 'client_session',
  execution_mode: 'manual',
  inputs: [],
  outputs: [
    {
      id: 'out',
      label: 'Out',
      data_type: 'number',
      required: false,
      multiple: false,
    },
  ],
};

const targetDefinition: NodeDefinition = {
  node_type: 'target',
  category: 'processing',
  label: 'Target',
  description: 'Target node',
  io_binding_origin: 'client_session',
  execution_mode: 'manual',
  inputs: [
    {
      id: 'in',
      label: 'In',
      data_type: 'boolean',
      required: false,
      multiple: false,
    },
    {
      id: 'text',
      label: 'Text',
      data_type: 'string',
      required: false,
      multiple: false,
    },
  ],
  outputs: [],
};

test('edgeToGraphEdge normalizes missing handles for backend graph edges', () => {
  assert.deepEqual(
    edgeToGraphEdge({
      id: 'edge-a',
      source: 'source-a',
      target: 'target-a',
    }),
    {
      id: 'edge-a',
      source: 'source-a',
      source_handle: 'output',
      target: 'target-a',
      target_handle: 'input',
    },
  );
});

test('buildConnectionIntentState projects backend candidates into UI state', () => {
  const intent = buildConnectionIntentState({
    graph_revision: 'rev-a',
    revision_matches: true,
    source_anchor: { node_id: 'source-a', port_id: 'out' },
    compatible_nodes: [
      {
        node_id: 'target-a',
        node_type: 'target',
        node_label: 'Target',
        position: { x: 0, y: 0 },
        anchors: [
          {
            port_id: 'in',
            port_label: 'In',
            data_type: 'number',
            multiple: false,
          },
        ],
      },
    ],
    insertable_node_types: [],
  });

  assert.equal(intent.graphRevision, 'rev-a');
  assert.deepEqual(intent.compatibleNodeIds, ['target-a']);
  assert.deepEqual(intent.compatibleTargetKeys, ['target-a:in']);
});

test('preserveConnectionIntentState keeps existing candidates while adding rejection state', () => {
  assert.deepEqual(
    preserveConnectionIntentState({
      sourceAnchor: { node_id: 'source-b', port_id: 'out' },
      graphRevision: 'rev-b',
      currentIntent: {
        sourceAnchor: { node_id: 'source-a', port_id: 'out' },
        graphRevision: 'rev-a',
        compatibleNodeIds: ['target-a'],
        compatibleTargetKeys: ['target-a:in'],
        insertableNodeTypes: [
          {
            node_type: 'processor',
            label: 'Processor',
            category: 'processing',
            description: 'Processor node',
            matching_input_port_ids: ['in'],
          },
        ],
      },
      rejection: {
        reason: 'stale_revision',
        message: 'Graph changed',
      },
    }),
    {
      sourceAnchor: { node_id: 'source-b', port_id: 'out' },
      graphRevision: 'rev-b',
      compatibleNodeIds: ['target-a'],
      compatibleTargetKeys: ['target-a:in'],
      insertableNodeTypes: [
        {
          node_type: 'processor',
          label: 'Processor',
          category: 'processing',
          description: 'Processor node',
          matching_input_port_ids: ['in'],
        },
      ],
      rejection: {
        reason: 'stale_revision',
        message: 'Graph changed',
      },
    },
  );
});

test('preserveConnectionIntentState builds an empty fallback without current intent', () => {
  assert.deepEqual(
    preserveConnectionIntentState({
      sourceAnchor: { node_id: 'source-a', port_id: 'out' },
      graphRevision: 'rev-a',
      currentIntent: null,
    }),
    {
      sourceAnchor: { node_id: 'source-a', port_id: 'out' },
      graphRevision: 'rev-a',
      compatibleNodeIds: [],
      compatibleTargetKeys: [],
      insertableNodeTypes: [],
      rejection: undefined,
    },
  );
});

test('resolveWorkflowConnectionAnchors returns backend anchors for complete connections', () => {
  assert.deepEqual(
    resolveWorkflowConnectionAnchors({
      source: 'source-a',
      sourceHandle: 'out',
      target: 'target-a',
      targetHandle: 'in',
    }),
    {
      sourceAnchor: { node_id: 'source-a', port_id: 'out' },
      targetAnchor: { node_id: 'target-a', port_id: 'in' },
    },
  );

  assert.equal(
    resolveWorkflowConnectionAnchors({
      source: 'source-a',
      sourceHandle: 'out',
      target: null,
      targetHandle: 'in',
    }),
    null,
  );
});

test('resolveConnectionCommitGraphRevision prefers matching active intent revision', () => {
  const currentIntent = {
    sourceAnchor: { node_id: 'source-a', port_id: 'out' },
    graphRevision: 'intent-rev',
    compatibleNodeIds: [],
    compatibleTargetKeys: [],
    insertableNodeTypes: [],
  };

  assert.equal(
    resolveConnectionCommitGraphRevision({
      sourceAnchor: { node_id: 'source-a', port_id: 'out' },
      currentIntent,
      currentGraphRevision: 'current-rev',
    }),
    'intent-rev',
  );
  assert.equal(
    resolveConnectionCommitGraphRevision({
      sourceAnchor: { node_id: 'source-b', port_id: 'out' },
      currentIntent,
      currentGraphRevision: 'current-rev',
    }),
    'current-rev',
  );
});

test('isWorkflowConnectionValid uses connection intent when it matches the active source', () => {
  const intent = {
    sourceAnchor: { node_id: 'source-a', port_id: 'out' },
    graphRevision: 'rev-a',
    compatibleNodeIds: ['target-a'],
    compatibleTargetKeys: ['target-a:in'],
    insertableNodeTypes: [],
  };

  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'in',
      },
      [],
      intent,
    ),
    true,
  );
  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'other',
      },
      [],
      intent,
    ),
    false,
  );
});

test('isWorkflowConnectionValid falls back to port compatibility when no intent matches', () => {
  const graphNodes = [
    { id: 'source-a', data: { definition: sourceDefinition } },
    { id: 'target-a', data: { definition: targetDefinition } },
  ];

  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'text',
      },
      graphNodes,
      null,
    ),
    true,
  );
  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'in',
      },
      graphNodes,
      null,
    ),
    false,
  );
});
