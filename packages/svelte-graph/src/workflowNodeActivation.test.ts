import test from 'node:test';
import assert from 'node:assert/strict';

import {
  isWorkflowGroupNode,
  resolveWorkflowGroupZoomTarget,
  resolveWorkflowNodeClick,
} from './workflowNodeActivation.ts';

test('resolveWorkflowNodeClick detects repeated node clicks within threshold', () => {
  const firstClick = resolveWorkflowNodeClick(
    {
      lastClickTime: 0,
      lastClickNodeId: null,
    },
    'node-a',
    1000,
  );

  assert.equal(firstClick.isDoubleClick, false);
  assert.deepEqual(firstClick.state, {
    lastClickTime: 1000,
    lastClickNodeId: 'node-a',
  });

  const secondClick = resolveWorkflowNodeClick(firstClick.state, 'node-a', 1250);
  assert.equal(secondClick.isDoubleClick, true);

  const otherNodeClick = resolveWorkflowNodeClick(firstClick.state, 'node-b', 1250);
  assert.equal(otherNodeClick.isDoubleClick, false);

  const lateClick = resolveWorkflowNodeClick(firstClick.state, 'node-a', 1400);
  assert.equal(lateClick.isDoubleClick, false);
});

test('resolveWorkflowGroupZoomTarget projects group nodes into zoom targets', () => {
  const target = resolveWorkflowGroupZoomTarget({
    id: 'group-a',
    type: 'node-group',
    position: { x: 10, y: 20 },
    measured: { width: 320, height: 180 },
    data: {},
  });

  assert.deepEqual(target, {
    nodeId: 'group-a',
    position: { x: 10, y: 20 },
    bounds: {
      width: 320,
      height: 180,
    },
  });
});

test('group activation helpers support data-backed group markers and defaults', () => {
  const groupNode = {
    id: 'group-b',
    position: { x: 0, y: 0 },
    data: { isGroup: true },
  };

  assert.equal(isWorkflowGroupNode(groupNode), true);
  assert.deepEqual(resolveWorkflowGroupZoomTarget(groupNode)?.bounds, {
    width: 200,
    height: 100,
  });

  assert.equal(
    resolveWorkflowGroupZoomTarget({
      id: 'plain-node',
      type: 'llm-inference',
      position: { x: 0, y: 0 },
      data: {},
    }),
    null,
  );
});
