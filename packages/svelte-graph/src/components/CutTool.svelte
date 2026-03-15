<script lang="ts">
  import type { Edge } from '@xyflow/svelte';
  import { linesIntersect } from '../utils/geometry.js';
  import {
    applyMatrixToPoint,
    findRenderedEdgePath,
    isCutModifierPressed,
    shouldStartCutGesture,
    toContainerRelativePoint,
  } from '../cutInteraction.js';

  interface Props {
    edges: Edge[];
    enabled?: boolean;
    onEdgesCut?: (edgeIds: string[]) => Promise<void>;
    ctrlPressed?: boolean;
    isCutting?: boolean;
  }

  let {
    edges,
    enabled = true,
    onEdgesCut,
    ctrlPressed = $bindable(false),
    isCutting = $bindable(false),
  }: Props = $props();

  let cutStart = $state<{ x: number; y: number } | null>(null);
  let cutEnd = $state<{ x: number; y: number } | null>(null);
  let cutContainerRect = $state<DOMRect | null>(null);
  let isFinalizingCut = $state(false);

  function handleKeyDown(e: KeyboardEvent) {
    if (isCutModifierPressed(e)) {
      ctrlPressed = true;
    }
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (!isCutModifierPressed(e) && !ctrlPressed) {
      return;
    }

    ctrlPressed = e.ctrlKey || e.metaKey;
    if (!ctrlPressed && isCutting) {
      void finishCut();
    }
  }

  /** Call from parent's mousedown handler on the graph container */
  export function onPaneMouseDown(e: MouseEvent) {
    ctrlPressed = ctrlPressed || isCutModifierPressed(e);
    if (
      !shouldStartCutGesture({
        enabled,
        modifierPressed: ctrlPressed || isCutModifierPressed(e),
        target: e.target as HTMLElement | null,
      })
    ) {
      return;
    }

    isCutting = true;
    const container = (e.currentTarget as HTMLElement).querySelector('.svelte-flow');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    cutContainerRect = rect;
    cutStart = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    cutEnd = cutStart;
  }

  /** Call from parent's mousemove handler on the graph container */
  export function onPaneMouseMove(e: MouseEvent) {
    if (!isCutting || !cutStart) return;

    const container = (e.currentTarget as HTMLElement).querySelector('.svelte-flow');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    cutContainerRect = rect;
    cutEnd = { x: e.clientX - rect.left, y: e.clientY - rect.top };
  }

  /** Call from parent's mouseup handler on the graph container */
  export function onPaneMouseUp() {
    if (isCutting) {
      void finishCut();
    }
  }

  function lineIntersectsPath(
    p1: { x: number; y: number },
    p2: { x: number; y: number },
    path: SVGPathElement,
    containerRect: DOMRect | null,
  ): boolean {
    const screenMatrix = path.getScreenCTM();
    if (!screenMatrix || !containerRect) {
      return false;
    }

    const pathLength = path.getTotalLength();
    const samples = 20;

    for (let i = 0; i < samples; i++) {
      const t1 = (i / samples) * pathLength;
      const t2 = ((i + 1) / samples) * pathLength;

      const point1 = path.getPointAtLength(t1);
      const point2 = path.getPointAtLength(t2);
      const containerPoint1 = toContainerRelativePoint(
        applyMatrixToPoint(point1, screenMatrix),
        containerRect,
      );
      const containerPoint2 = toContainerRelativePoint(
        applyMatrixToPoint(point2, screenMatrix),
        containerRect,
      );

      if (linesIntersect(p1, p2, containerPoint1, containerPoint2)) {
        return true;
      }
    }
    return false;
  }

  async function finishCut() {
    if (isFinalizingCut) {
      return;
    }

    isFinalizingCut = true;
    try {
      if (!cutStart || !cutEnd) {
        isCutting = false;
        cutStart = null;
        cutEnd = null;
        cutContainerRect = null;
        return;
      }

      const edgesToRemove = edges.filter((edge) => {
        const edgeEl = findRenderedEdgePath(document, edge.id);
        if (!edgeEl) return false;
        return lineIntersectsPath(cutStart!, cutEnd!, edgeEl as SVGPathElement, cutContainerRect);
      });

      if (edgesToRemove.length > 0 && onEdgesCut) {
        await onEdgesCut(edgesToRemove.map((e) => e.id));
      }

      isCutting = false;
      cutStart = null;
      cutEnd = null;
      cutContainerRect = null;
    } finally {
      isFinalizingCut = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} onkeyup={handleKeyUp} />

{#if isCutting && cutStart && cutEnd}
  <svg class="cut-overlay">
    <line
      x1={cutStart.x}
      y1={cutStart.y}
      x2={cutEnd.x}
      y2={cutEnd.y}
      stroke="#ef4444"
      stroke-width="2"
      stroke-dasharray="5,5"
    />
  </svg>
{/if}

<style>
  .cut-overlay {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 1000;
  }
</style>
