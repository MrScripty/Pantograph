import test from 'node:test';
import assert from 'node:assert/strict';

import { computeWorkflowGraphSyncDecision } from './workflowGraphSync.ts';

test('computeWorkflowGraphSyncDecision skips node reassignment for a matching graph key', () => {
  const prevNodes = [{ id: 'a' }];
  const nextNodes = [{ id: 'a' }];
  const prevEdges = [{ id: 'e1' }];

  const decision = computeWorkflowGraphSyncDecision({
    storeNodes: nextNodes,
    storeEdges: prevEdges,
    prevNodesRef: prevNodes,
    prevEdgesRef: prevEdges,
    graphSyncKey: 'graph-a',
    prevGraphSyncKey: 'graph-a',
    skipNodeSyncGraphKey: 'graph-a',
  });

  assert.equal(decision.applyNodes, false);
  assert.equal(decision.applyEdges, false);
  assert.equal(decision.nextPrevNodesRef, nextNodes);
  assert.equal(decision.nextPrevEdgesRef, prevEdges);
  assert.equal(decision.nextPrevGraphSyncKey, 'graph-a');
  assert.equal(decision.nextSkipNodeSyncGraphKey, null);
});

test('computeWorkflowGraphSyncDecision still applies same-graph edge updates while node sync is skipped', () => {
  const prevNodes = [{ id: 'a' }];
  const nextNodes = [{ id: 'a' }];
  const prevEdges = [{ id: 'e1' }];
  const nextEdges = [{ id: 'e1' }, { id: 'e2' }];

  const decision = computeWorkflowGraphSyncDecision({
    storeNodes: nextNodes,
    storeEdges: nextEdges,
    prevNodesRef: prevNodes,
    prevEdgesRef: prevEdges,
    graphSyncKey: 'graph-a',
    prevGraphSyncKey: 'graph-a',
    skipNodeSyncGraphKey: 'graph-a',
  });

  assert.equal(decision.applyNodes, false);
  assert.equal(decision.applyEdges, true);
});

test('computeWorkflowGraphSyncDecision applies nodes and edges when the graph key changes', () => {
  const prevNodes = [{ id: 'a' }];
  const nextNodes = [{ id: 'b' }];
  const prevEdges = [{ id: 'e1', source: 'a', target: 'a' }];
  const nextEdges = [{ id: 'e2', source: 'b', target: 'b' }];

  const decision = computeWorkflowGraphSyncDecision({
    storeNodes: nextNodes,
    storeEdges: nextEdges,
    prevNodesRef: prevNodes,
    prevEdgesRef: prevEdges,
    graphSyncKey: 'graph-b',
    prevGraphSyncKey: 'graph-a',
    skipNodeSyncGraphKey: 'graph-a',
  });

  assert.equal(decision.applyNodes, true);
  assert.equal(decision.applyEdges, true);
  assert.equal(decision.nextPrevNodesRef, nextNodes);
  assert.equal(decision.nextPrevEdgesRef, nextEdges);
  assert.equal(decision.nextPrevGraphSyncKey, 'graph-b');
  assert.equal(decision.nextSkipNodeSyncGraphKey, null);
});

test('computeWorkflowGraphSyncDecision does not treat an unrelated skip key as active', () => {
  const prevNodes = [{ id: 'a' }];
  const nextNodes = [{ id: 'b' }];
  const prevEdges = [{ id: 'e1' }];

  const decision = computeWorkflowGraphSyncDecision({
    storeNodes: nextNodes,
    storeEdges: prevEdges,
    prevNodesRef: prevNodes,
    prevEdgesRef: prevEdges,
    graphSyncKey: 'graph-b',
    prevGraphSyncKey: 'graph-b',
    skipNodeSyncGraphKey: 'graph-a',
  });

  assert.equal(decision.applyNodes, true);
  assert.equal(decision.applyEdges, false);
});

test('computeWorkflowGraphSyncDecision applies node sync when references change and no skip is set', () => {
  const prevNodes = [{ id: 'a' }];
  const nextNodes = [{ id: 'a' }, { id: 'b' }];
  const edges = [{ id: 'e1' }];

  const decision = computeWorkflowGraphSyncDecision({
    storeNodes: nextNodes,
    storeEdges: edges,
    prevNodesRef: prevNodes,
    prevEdgesRef: edges,
    graphSyncKey: 'graph-a',
    prevGraphSyncKey: 'graph-a',
    skipNodeSyncGraphKey: null,
  });

  assert.equal(decision.applyNodes, true);
  assert.equal(decision.applyEdges, false);
});
