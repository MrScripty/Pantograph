<script lang="ts">
  interface ContainerBounds {
    x: number;
    y: number;
    width: number;
    height: number;
  }

  interface ViewportState {
    x: number;
    y: number;
    zoom: number;
  }

  interface Props {
    bounds: ContainerBounds | null;
    viewport: ViewportState | null;
    selected: boolean;
    onToggleSelected: () => void;
  }

  let { bounds, viewport, selected, onToggleSelected }: Props = $props();

  function handleEdgeClick(event: MouseEvent) {
    event.stopPropagation();
    onToggleSelected();
  }
</script>

{#if bounds && viewport}
  {@const x = bounds.x * viewport.zoom + viewport.x}
  {@const y = bounds.y * viewport.zoom + viewport.y}
  {@const w = bounds.width * viewport.zoom}
  {@const h = bounds.height * viewport.zoom}
  {@const edgeWidth = 12}

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

  <button
    type="button"
    class="container-edge top"
    onclick={handleEdgeClick}
    aria-label="Select orchestration boundary"
    style="
      position: absolute;
      left: {x}px;
      top: {y - edgeWidth / 2}px;
      width: {w}px;
      height: {edgeWidth}px;
      border: 0;
      padding: 0;
      background: transparent;
      cursor: pointer;
      pointer-events: auto;
      z-index: 2;
    "
  ></button>
  <button
    type="button"
    class="container-edge bottom"
    onclick={handleEdgeClick}
    aria-label="Select orchestration boundary"
    style="
      position: absolute;
      left: {x}px;
      top: {y + h - edgeWidth / 2}px;
      width: {w}px;
      height: {edgeWidth}px;
      border: 0;
      padding: 0;
      background: transparent;
      cursor: pointer;
      pointer-events: auto;
      z-index: 2;
    "
  ></button>
  <button
    type="button"
    class="container-edge left"
    onclick={handleEdgeClick}
    aria-label="Select orchestration boundary"
    style="
      position: absolute;
      left: {x - edgeWidth / 2}px;
      top: {y}px;
      width: {edgeWidth}px;
      height: {h}px;
      border: 0;
      padding: 0;
      background: transparent;
      cursor: pointer;
      pointer-events: auto;
      z-index: 2;
    "
  ></button>
  <button
    type="button"
    class="container-edge right"
    onclick={handleEdgeClick}
    aria-label="Select orchestration boundary"
    style="
      position: absolute;
      left: {x + w - edgeWidth / 2}px;
      top: {y}px;
      width: {edgeWidth}px;
      height: {h}px;
      border: 0;
      padding: 0;
      background: transparent;
      cursor: pointer;
      pointer-events: auto;
      z-index: 2;
    "
  ></button>

  <div
    class="container-anchor input"
    style="
      position: absolute;
      left: {x - 8}px;
      top: {y + h / 2 - 8}px;
      width: 16px;
      height: 16px;
      background: #3b82f6;
      border: 2px solid #1e3a5f;
      border-radius: 50%;
      pointer-events: auto;
      z-index: 3;
      box-shadow: 0 0 8px rgba(59, 130, 246, 0.6);
    "
  ></div>
  <div
    class="container-anchor output"
    style="
      position: absolute;
      left: {x + w - 8}px;
      top: {y + h / 2 - 8}px;
      width: 16px;
      height: 16px;
      background: #3b82f6;
      border: 2px solid #1e3a5f;
      border-radius: 50%;
      pointer-events: auto;
      z-index: 3;
      box-shadow: 0 0 8px rgba(59, 130, 246, 0.6);
    "
  ></div>
{/if}
