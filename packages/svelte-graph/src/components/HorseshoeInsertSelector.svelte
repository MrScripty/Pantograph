<script lang="ts">
  import { getHorseshoeWindow } from '../horseshoeSelector.js';
  import type { InsertableNodeTypeCandidate } from '../types/workflow.js';

  interface Props {
    visible: boolean;
    anchorPosition: { x: number; y: number } | null;
    items: InsertableNodeTypeCandidate[];
    selectedIndex: number;
    query?: string;
    pending?: boolean;
    onSelect?: (candidate: InsertableNodeTypeCandidate) => void;
    onRotate?: (delta: number) => void;
    onCancel?: () => void;
  }

  let {
    visible,
    anchorPosition,
    items,
    selectedIndex,
    query = '',
    pending = false,
    onSelect,
    onRotate,
    onCancel,
  }: Props = $props();

  const OUTER_RADIUS = 126;
  const ITEM_RADIUS = 24;
  const START_ANGLE = -150;
  const END_ANGLE = -30;

  const windowData = $derived(getHorseshoeWindow(items, selectedIndex));
  const visibleItems = $derived(windowData.visibleItems);
  const hiddenBefore = $derived(windowData.hiddenBefore);
  const hiddenAfter = $derived(windowData.hiddenAfter);

  function itemPosition(slot: number, itemCount: number) {
    if (itemCount <= 1) {
      return {
        x: 0,
        y: -OUTER_RADIUS,
        angle: -90,
      };
    }

    const step = (END_ANGLE - START_ANGLE) / (itemCount - 1);
    const angle = START_ANGLE + step * slot;
    const radians = (angle * Math.PI) / 180;
    return {
      x: Math.cos(radians) * OUTER_RADIUS,
      y: Math.sin(radians) * OUTER_RADIUS,
      angle,
    };
  }

  function handleWheel(event: WheelEvent) {
    if (!visible || pending) return;
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

{#if visible && anchorPosition && items.length > 0}
  <div
    class="horseshoe-root"
    style="left: {anchorPosition.x}px; top: {anchorPosition.y}px;"
    onwheel={handleWheel}
  >
    {#if hiddenBefore > 0}
      <div class="horseshoe-counter left">+{hiddenBefore}</div>
    {/if}

    {#if hiddenAfter > 0}
      <div class="horseshoe-counter right">+{hiddenAfter}</div>
    {/if}

    {#each visibleItems as entry}
      {@const position = itemPosition(entry.slot, visibleItems.length)}
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

    <button type="button" class="horseshoe-center" onpointerdown={handleCancel}>
      {#if query}
        <span>{query}</span>
      {:else if pending}
        <span>...</span>
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
</style>
