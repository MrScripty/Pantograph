<script lang="ts">
  import {
    getHorseshoeItemPosition,
    getHorseshoeWindow,
  } from '../horseshoeSelector.js';
  import type { HorseshoeDisplayState } from '../horseshoeDragSession.js';
  import type { InsertableNodeTypeCandidate } from '../types/workflow.js';

  interface Props {
    displayState: HorseshoeDisplayState;
    anchorPosition: { x: number; y: number } | null;
    items: InsertableNodeTypeCandidate[];
    selectedIndex: number;
    query?: string;
    pending?: boolean;
    statusLabel?: string | null;
    onSelect?: (candidate: InsertableNodeTypeCandidate) => void;
    onRotate?: (delta: number) => void;
    onCancel?: () => void;
  }

  let {
    displayState,
    anchorPosition,
    items,
    selectedIndex,
    query = '',
    pending = false,
    statusLabel = null,
    onSelect,
    onRotate,
    onCancel,
  }: Props = $props();

  const windowData = $derived(getHorseshoeWindow(items, selectedIndex));
  const visibleItems = $derived(windowData.visibleItems);
  const hiddenBefore = $derived(windowData.hiddenBefore);
  const hiddenAfter = $derived(windowData.hiddenAfter);

  function handleWheel(event: WheelEvent) {
    if (displayState !== 'open' || pending) return;
    event.preventDefault();
    event.stopPropagation();
    onRotate?.(event.deltaY > 0 ? 1 : -1);
  }

  function handleCancel(event: PointerEvent) {
    event.preventDefault();
    event.stopPropagation();
    onCancel?.();
  }

  function handleSelect(event: PointerEvent, candidate: InsertableNodeTypeCandidate) {
    event.preventDefault();
    event.stopPropagation();
    if (pending) return;
    onSelect?.(candidate);
  }
</script>

{#if displayState !== 'hidden' && (anchorPosition || displayState !== 'open')}
  <div
    class="horseshoe-root"
    class:fallback-anchor={!anchorPosition}
    style:left={anchorPosition ? `${anchorPosition.x}px` : undefined}
    style:top={anchorPosition ? `${anchorPosition.y}px` : undefined}
    onwheel={handleWheel}
  >
    {#if displayState === 'open' && hiddenBefore > 0}
      <div class="horseshoe-counter left">+{hiddenBefore}</div>
    {/if}

    {#if displayState === 'open' && hiddenAfter > 0}
      <div class="horseshoe-counter right">+{hiddenAfter}</div>
    {/if}

    {#if displayState === 'open' && anchorPosition && items.length > 0}
      {#each visibleItems as entry}
        {@const position = getHorseshoeItemPosition(entry.slot, visibleItems.length)}
        <button
          type="button"
          class="horseshoe-item"
          class:selected={entry.index === selectedIndex}
          class:pending
          style="left: {position.x}px; top: {position.y}px;"
          onpointerdown={(event) => handleSelect(event, entry.item)}
        >
          <span class="label">{entry.item.label}</span>
        </button>
      {/each}
    {/if}

    {#if displayState !== 'open' && statusLabel}
      <div class="horseshoe-status">{statusLabel}</div>
    {/if}

    <button type="button" class="horseshoe-center" onpointerdown={handleCancel}>
      {#if pending}
        <span>...</span>
      {:else if query}
        <span>{query}</span>
      {:else if displayState === 'pending'}
        <span>Wait</span>
      {:else if displayState === 'blocked'}
        <span>Close</span>
      {:else}
        <span>Esc</span>
      {/if}
    </button>
  </div>
{/if}

<style>
  .horseshoe-root {
    position: absolute;
    transform: translate(-50%, -50%);
    width: 0;
    height: 0;
    pointer-events: none;
    z-index: 1200;
  }

  .horseshoe-root.fallback-anchor {
    left: 50%;
    top: 3rem;
    transform: translateX(-50%);
  }

  .horseshoe-item,
  .horseshoe-center,
  .horseshoe-counter {
    position: absolute;
    transform: translate(-50%, -50%);
    pointer-events: auto;
  }

  .horseshoe-item {
    min-width: 104px;
    max-width: 148px;
    border: 1px solid rgba(82, 82, 91, 0.95);
    border-radius: 999px;
    background: rgba(23, 23, 23, 0.94);
    color: #e5e7eb;
    padding: 0.55rem 0.85rem;
    font-size: 0.72rem;
    line-height: 1;
    text-align: center;
    box-shadow:
      0 8px 20px rgba(0, 0, 0, 0.35),
      0 0 24px rgba(34, 197, 94, 0.08);
    transition:
      transform 120ms ease,
      border-color 120ms ease,
      background-color 120ms ease;
  }

  .horseshoe-item.selected {
    border-color: rgba(52, 211, 153, 0.92);
    background: rgba(6, 78, 59, 0.92);
    transform: translate(-50%, -50%) scale(1.08);
  }

  .horseshoe-item.pending {
    opacity: 0.72;
  }

  .horseshoe-item .label {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .horseshoe-center {
    width: 52px;
    height: 52px;
    border-radius: 999px;
    border: 1px solid rgba(82, 82, 91, 0.95);
    background: rgba(10, 10, 10, 0.94);
    color: #d4d4d8;
    font-size: 0.7rem;
    font-weight: 600;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
  }

  .horseshoe-counter {
    min-width: 36px;
    padding: 0.25rem 0.5rem;
    border-radius: 999px;
    border: 1px solid rgba(82, 82, 91, 0.85);
    background: rgba(23, 23, 23, 0.88);
    color: #a1a1aa;
    font-size: 0.68rem;
    text-align: center;
  }

  .horseshoe-counter.left {
    left: -158px;
    top: -32px;
  }

  .horseshoe-counter.right {
    left: 158px;
    top: -32px;
  }

  .horseshoe-status {
    position: absolute;
    transform: translate(-50%, -50%);
    left: 0;
    top: -78px;
    min-width: 180px;
    max-width: 240px;
    padding: 0.45rem 0.7rem;
    border-radius: 999px;
    border: 1px solid rgba(82, 82, 91, 0.9);
    background: rgba(23, 23, 23, 0.95);
    color: #e5e7eb;
    font-size: 0.72rem;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
    pointer-events: none;
  }
</style>
