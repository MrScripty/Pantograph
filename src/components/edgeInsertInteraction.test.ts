import test from 'node:test';
import assert from 'node:assert/strict';

import {
  EDGE_INSERT_HIT_RADIUS_PX,
  applyEdgeInsertPreviewActiveFlag,
  clearEdgeInsertPreviewState,
  createEdgeInsertPreviewState,
  findEdgeInsertHitTarget,
  getCommittableEdgeInsertPreview,
  sampleClosestEdgeDistance,
  setEdgeInsertHoverTarget,
  updateEdgeInsertHitPoint,
  setEdgeInsertPreviewPending,
  setEdgeInsertPreviewRejected,
  setEdgeInsertPreviewResolved,
  shouldRefreshEdgeInsertPreview,
  type Point,
} from './edgeInsertInteraction.ts';

type Matrix2D = {
  a: number;
  b: number;
  c: number;
  d: number;
  e: number;
  f: number;
};

const IDENTITY_MATRIX: Matrix2D = {
  a: 1,
  b: 0,
  c: 0,
  d: 1,
  e: 0,
  f: 0,
};

function createLinePath(y: number, length: number) {
  return {
    getTotalLength() {
      return length;
    },
    getPointAtLength(distance: number): Point {
      return {
        x: Math.max(0, Math.min(length, distance)),
        y,
      };
    },
    getScreenCTM() {
      return IDENTITY_MATRIX;
    },
  };
}

test('edge insert preview state tracks hover, pending, success, and rejection', () => {
  const hoverState = setEdgeInsertHoverTarget(
    createEdgeInsertPreviewState(),
    {
      edgeId: 'edge-1',
      hitPoint: { x: 120, y: 40 },
      distance: 12,
    },
    'embedding',
    'rev-1',
  );

  assert.deepEqual(hoverState, {
    edgeId: 'edge-1',
    nodeType: 'embedding',
    graphRevision: 'rev-1',
    hitPoint: { x: 120, y: 40 },
    pending: false,
    bridge: null,
    rejection: null,
  });

  const pendingState = setEdgeInsertPreviewPending(hoverState);
  assert.equal(pendingState.pending, true);

  const resolvedState = setEdgeInsertPreviewResolved(pendingState, {
    input_port_id: 'prompt',
    output_port_id: 'document',
  });
  assert.equal(resolvedState.pending, false);
  assert.deepEqual(resolvedState.bridge, {
    input_port_id: 'prompt',
    output_port_id: 'document',
  });

  const movedState = updateEdgeInsertHitPoint(resolvedState, { x: 132, y: 42 });
  assert.deepEqual(movedState.hitPoint, { x: 132, y: 42 });
  assert.deepEqual(movedState.bridge, resolvedState.bridge);

  const rejectedState = setEdgeInsertPreviewRejected(movedState, {
    reason: 'no_compatible_insert_path',
    message: 'No compatible bridge',
  });
  assert.equal(rejectedState.bridge, null);
  assert.equal(rejectedState.rejection?.reason, 'no_compatible_insert_path');

  assert.deepEqual(clearEdgeInsertPreviewState(), createEdgeInsertPreviewState());
});

test('shouldRefreshEdgeInsertPreview only invalidates when preview inputs change', () => {
  const state = setEdgeInsertPreviewResolved(
    setEdgeInsertPreviewPending(
      setEdgeInsertHoverTarget(
        createEdgeInsertPreviewState(),
        {
          edgeId: 'edge-1',
          hitPoint: { x: 120, y: 40 },
          distance: 12,
        },
        'embedding',
        'rev-1',
      ),
    ),
    {
      input_port_id: 'prompt',
      output_port_id: 'document',
    },
  );

  assert.equal(shouldRefreshEdgeInsertPreview(state, 'edge-1', 'embedding', 'rev-1'), false);
  assert.equal(shouldRefreshEdgeInsertPreview(state, 'edge-2', 'embedding', 'rev-1'), true);
  assert.equal(shouldRefreshEdgeInsertPreview(state, 'edge-1', 'llm-inference', 'rev-1'), true);
  assert.equal(shouldRefreshEdgeInsertPreview(state, 'edge-1', 'embedding', 'rev-2'), true);
});

test('getCommittableEdgeInsertPreview requires a matching resolved preview', () => {
  const resolvedState = setEdgeInsertPreviewResolved(
    setEdgeInsertPreviewPending(
      setEdgeInsertHoverTarget(
        createEdgeInsertPreviewState(),
        {
          edgeId: 'edge-1',
          hitPoint: { x: 120, y: 40 },
          distance: 12,
        },
        'embedding',
        'rev-1',
      ),
    ),
    {
      input_port_id: 'prompt',
      output_port_id: 'document',
    },
  );

  assert.deepEqual(getCommittableEdgeInsertPreview(resolvedState, 'embedding'), resolvedState);
  assert.equal(getCommittableEdgeInsertPreview(resolvedState, 'llm-inference'), null);
  assert.equal(
    getCommittableEdgeInsertPreview(setEdgeInsertPreviewRejected(resolvedState), 'embedding'),
    null,
  );
});

test('applyEdgeInsertPreviewActiveFlag toggles only changed edge data', () => {
  const edges = [
    { id: 'edge-1', data: { label: 'existing' } },
    { id: 'edge-2', data: { edgeInsertPreviewActive: true } },
    { id: 'edge-3' },
  ];

  const activated = applyEdgeInsertPreviewActiveFlag(edges, 'edge-1');

  assert.equal(activated.changed, true);
  assert.equal(activated.edges[0].data?.edgeInsertPreviewActive, true);
  assert.equal(activated.edges[0].data?.label, 'existing');
  assert.equal(activated.edges[1].data?.edgeInsertPreviewActive, undefined);
  assert.equal(activated.edges[2], edges[2]);

  const unchanged = applyEdgeInsertPreviewActiveFlag(activated.edges, 'edge-1');

  assert.equal(unchanged.changed, false);
  assert.equal(unchanged.edges[0], activated.edges[0]);
});

test('sampleClosestEdgeDistance measures from the cursor to the rendered path', () => {
  const distance = sampleClosestEdgeDistance({
    path: createLinePath(40, 100) as never,
    hitPoint: { x: 60, y: 46 },
    containerRect: { left: 0, top: 0 },
    sampleStepPx: 10,
  });

  assert.equal(distance, 6);
});

test('findEdgeInsertHitTarget picks the nearest edge within the threshold', () => {
  const root = {
    querySelectorAll() {
      return [
        {
          dataset: { id: 'edge-a' },
          querySelector() {
            return createLinePath(10, 100) as never;
          },
        },
        {
          dataset: { id: 'edge-b' },
          querySelector() {
            return createLinePath(40, 100) as never;
          },
        },
      ];
    },
  };

  assert.deepEqual(
    findEdgeInsertHitTarget({
      root: root as never,
      hitPoint: { x: 60, y: 36 },
      containerRect: { left: 0, top: 0 },
      thresholdPx: EDGE_INSERT_HIT_RADIUS_PX,
    }),
    {
      edgeId: 'edge-b',
      hitPoint: { x: 60, y: 36 },
      distance: 4,
    },
  );

  assert.equal(
    findEdgeInsertHitTarget({
      root: root as never,
      hitPoint: { x: 60, y: 90 },
      containerRect: { left: 0, top: 0 },
      thresholdPx: 8,
    }),
    null,
  );
});
