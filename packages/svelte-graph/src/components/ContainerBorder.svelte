<script lang="ts">
  import type { Node } from '@xyflow/svelte';

  interface Props {
    nodes: Node[];
    currentViewport: { x: number; y: number; zoom: number } | null;
    showBorder?: boolean;
    onZoomOut?: () => void;
    containerWidth: number;
    containerHeight: number;
  }

  let {
    nodes,
    currentViewport,
    showBorder = false,
    onZoomOut,
    containerWidth,
    containerHeight,
  }: Props = $props();

  const CONTAINER_MARGIN = 100;
  const VISIBILITY_MARGIN = 50;
  const EDGE_WIDTH = 12;

  let selected = $state(false);
  let transitionTriggered = $state(false);

  let bounds = $derived.by(() => {
    if (nodes.length === 0) return null;

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;

    for (const node of nodes) {
      const width = (node.measured?.width || node.width || 200) as number;
      const height = (node.measured?.height || node.height || 100) as number;

      minX = Math.min(minX, node.position.x);
      minY = Math.min(minY, node.position.y);
      maxX = Math.max(maxX, node.position.x + width);
      maxY = Math.max(maxY, node.position.y + height);
    }

    return {
      x: minX - CONTAINER_MARGIN,
      y: minY - CONTAINER_MARGIN,
      width: (maxX - minX) + (CONTAINER_MARGIN * 2),
      height: (maxY - minY) + (CONTAINER_MARGIN * 2),
    };
  });

  function isContainerFullyVisible(
    b: { x: number; y: number; width: number; height: number },
    vp: { x: number; y: number; zoom: number },
    screenW: number,
    screenH: number,
  ): boolean {
    const screenX = b.x * vp.zoom + vp.x;
    const screenY = b.y * vp.zoom + vp.y;
    const sw = b.width * vp.zoom;
    const sh = b.height * vp.zoom;

    return (
      screenX >= VISIBILITY_MARGIN &&
      screenY >= VISIBILITY_MARGIN &&
      screenX + sw <= screenW - VISIBILITY_MARGIN &&
      screenY + sh <= screenH - VISIBILITY_MARGIN
    );
  }

  /** Call from parent's onmoveend to check if container is fully visible */
  export function checkVisibility() {
    if (!bounds || !currentViewport || !showBorder) return;

    const fullyVisible = isContainerFullyVisible(bounds, currentViewport, containerWidth, containerHeight);

    if (fullyVisible && !transitionTriggered) {
      transitionTriggered = true;
      onZoomOut?.();
    }

    if (!fullyVisible) {
      transitionTriggered = false;
    }
  }

  /** Reset transition state when returning to data-graph view */
  export function resetTransition() {
    transitionTriggered = false;
  }

  /** Deselect the container border */
  export function deselect() {
    selected = false;
  }

  function handleContainerClick(event: MouseEvent) {
    event.stopPropagation();
    selected = !selected;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Tab' && selected) {
      e.preventDefault();
      selected = false;
      onZoomOut?.();
    }
    if (e.key === 'Escape' && selected) {
      e.preventDefault();
      selected = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if showBorder && bounds && currentViewport}
  {@const x = bounds.x * currentViewport.zoom + currentViewport.x}
  {@const y = bounds.y * currentViewport.zoom + currentViewport.y}
  {@const w = bounds.width * currentViewport.zoom}
  {@const h = bounds.height * currentViewport.zoom}

  <!-- Visual border (pointer-events: none) -->
  <div
    class="container-border-visual"
    style="
      position: absolute;
      left: {x}px;
      top: {y}px;
      width: {w}px;
      height: {h}px;
      border: 3px solid {selected ? '#93c5fd' : '#60a5fa'};
      border-radius: 8px;
      pointer-events: none;
      z-index: 1;
      box-shadow:
        0 0 15px rgba(96, 165, 250, 0.4),
        0 0 30px rgba(96, 165, 250, 0.2),
        inset 0 0 15px rgba(96, 165, 250, 0.05);
      transition: border-color 0.15s ease, box-shadow 0.15s ease;
    "
  ></div>

  <!-- Clickable edge zones (invisible, for click detection) -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="container-edge top" onclick={handleContainerClick}
    style="position:absolute; left:{x}px; top:{y - EDGE_WIDTH/2}px; width:{w}px; height:{EDGE_WIDTH}px; cursor:pointer; pointer-events:auto; z-index:2;"
  ></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="container-edge bottom" onclick={handleContainerClick}
    style="position:absolute; left:{x}px; top:{y + h - EDGE_WIDTH/2}px; width:{w}px; height:{EDGE_WIDTH}px; cursor:pointer; pointer-events:auto; z-index:2;"
  ></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="container-edge left" onclick={handleContainerClick}
    style="position:absolute; left:{x - EDGE_WIDTH/2}px; top:{y}px; width:{EDGE_WIDTH}px; height:{h}px; cursor:pointer; pointer-events:auto; z-index:2;"
  ></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="container-edge right" onclick={handleContainerClick}
    style="position:absolute; left:{x + w - EDGE_WIDTH/2}px; top:{y}px; width:{EDGE_WIDTH}px; height:{h}px; cursor:pointer; pointer-events:auto; z-index:2;"
  ></div>

  <!-- Input anchor (left side) -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="container-anchor input"
    style="position:absolute; left:{x - 8}px; top:{y + h/2 - 8}px; width:16px; height:16px; background:#3b82f6; border:2px solid #1e3a5f; border-radius:50%; pointer-events:auto; z-index:3; box-shadow:0 0 8px rgba(59,130,246,0.6);"
  ></div>
  <!-- Output anchor (right side) -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="container-anchor output"
    style="position:absolute; left:{x + w - 8}px; top:{y + h/2 - 8}px; width:16px; height:16px; background:#3b82f6; border:2px solid #1e3a5f; border-radius:50%; pointer-events:auto; z-index:3; box-shadow:0 0 8px rgba(59,130,246,0.6);"
  ></div>
{/if}
