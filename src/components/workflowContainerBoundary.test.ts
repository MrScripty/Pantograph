import test from 'node:test';
import assert from 'node:assert/strict';

import {
  isWorkflowContainerFullyVisible,
  resolveWorkflowContainerBounds,
  resolveWorkflowContainerTransitionDecision,
} from './workflowContainerBoundary.ts';

test('resolveWorkflowContainerBounds expands measured node extents by the orchestration margin', () => {
  const bounds = resolveWorkflowContainerBounds([
    {
      position: { x: 20, y: 40 },
      measured: { width: 120, height: 80 },
    },
    {
      position: { x: 300, y: 200 },
      width: 160,
      height: 140,
    },
  ]);

  assert.deepEqual(bounds, {
    x: -80,
    y: -60,
    width: 640,
    height: 500,
  });
});

test('resolveWorkflowContainerBounds returns null for empty graphs', () => {
  assert.equal(resolveWorkflowContainerBounds([]), null);
});

test('isWorkflowContainerFullyVisible applies zoom, pan, and visibility margin', () => {
  const bounds = { x: 0, y: 0, width: 400, height: 300 };

  assert.equal(
    isWorkflowContainerFullyVisible(bounds, { x: 100, y: 100, zoom: 1 }, 800, 700),
    true,
  );
  assert.equal(
    isWorkflowContainerFullyVisible(bounds, { x: 10, y: 100, zoom: 1 }, 800, 700),
    false,
  );
});

test('resolveWorkflowContainerTransitionDecision triggers once when bounds become visible', () => {
  const bounds = { x: 0, y: 0, width: 400, height: 300 };

  assert.deepEqual(
    resolveWorkflowContainerTransitionDecision({
      bounds,
      viewport: { x: 100, y: 100, zoom: 1 },
      screenWidth: 800,
      screenHeight: 700,
      hasCurrentOrchestration: true,
      transitionTriggered: false,
    }),
    {
      transitionTriggered: true,
      shouldZoomToOrchestration: true,
    },
  );

  assert.deepEqual(
    resolveWorkflowContainerTransitionDecision({
      bounds,
      viewport: { x: 100, y: 100, zoom: 1 },
      screenWidth: 800,
      screenHeight: 700,
      hasCurrentOrchestration: true,
      transitionTriggered: true,
    }),
    {
      transitionTriggered: true,
      shouldZoomToOrchestration: false,
    },
  );
});

test('resolveWorkflowContainerTransitionDecision resets when bounds are not visible', () => {
  assert.deepEqual(
    resolveWorkflowContainerTransitionDecision({
      bounds: { x: 0, y: 0, width: 400, height: 300 },
      viewport: { x: 10, y: 100, zoom: 1 },
      screenWidth: 800,
      screenHeight: 700,
      hasCurrentOrchestration: true,
      transitionTriggered: true,
    }),
    {
      transitionTriggered: false,
      shouldZoomToOrchestration: false,
    },
  );
});

test('resolveWorkflowContainerTransitionDecision preserves state without orchestration context', () => {
  assert.deepEqual(
    resolveWorkflowContainerTransitionDecision({
      bounds: { x: 0, y: 0, width: 400, height: 300 },
      viewport: { x: 100, y: 100, zoom: 1 },
      screenWidth: 800,
      screenHeight: 700,
      hasCurrentOrchestration: false,
      transitionTriggered: true,
    }),
    {
      transitionTriggered: true,
      shouldZoomToOrchestration: false,
    },
  );
});
