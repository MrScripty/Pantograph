import test from 'node:test';
import assert from 'node:assert/strict';

import { computeWorkflowGraphSyncDecision } from './workflowGraphSync.ts';

test('computeWorkflowGraphSyncDecision skips node reassignment when requested', () => {
  const prevNodes = [{ id: 'a' }];
  const nextNodes = [{ id: 'a' }];
  const prevEdges = [{ id: 'e1' }];

  const decision = computeWorkflowGraphSyncDecision({
    storeNodes: nextNodes,
    storeEdges: prevEdges,
    prevNodesRef: prevNodes,
    prevEdgesRef: prevEdges,
    skipNextNodeSync: true,
  });

  assert.equal(decision.applyNodes, false);
  assert.equal(decision.applyEdges, false);
  assert.equal(decision.nextPrevNodesRef, nextNodes);
  assert.equal(decision.nextPrevEdgesRef, prevEdges);
  assert.equal(decision.nextSkipNextNodeSync, false);
});

test('computeWorkflowGraphSyncDecision still applies edge updates while node sync is skipped', () => {
  const prevNodes = [{ id: 'a' }];
  const nextNodes = [{ id: 'a' }];
  const prevEdges = [{ id: 'e1' }];
  const nextEdges = [{ id: 'e1' }, { id: 'e2' }];

  const decision = computeWorkflowGraphSyncDecision({
    storeNodes: nextNodes,
    storeEdges: nextEdges,
    prevNodesRef: prevNodes,
    prevEdgesRef: prevEdges,
    skipNextNodeSync: true,
  });

  assert.equal(decision.applyNodes, false);
  assert.equal(decision.applyEdges, true);
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
    skipNextNodeSync: false,
  });

  assert.equal(decision.applyNodes, true);
  assert.equal(decision.applyEdges, false);
});
