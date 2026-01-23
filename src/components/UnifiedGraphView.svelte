<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import {
    viewLevel,
    isAnimating,
    animationConfig,
    restoreViewState,
    zoomToOrchestration,
    zoomToDataGraph,
    tabIntoGroup,
    navigateBack,
    type ViewLevel,
  } from '../stores/viewStore';

  import ZoomTransition from './ZoomTransition.svelte';
  import NavigationBreadcrumb from './NavigationBreadcrumb.svelte';
  import WorkflowGraph from './WorkflowGraph.svelte';

  // Props for customization
  interface Props {
    /** Show the navigation breadcrumb */
    showBreadcrumb?: boolean;
    /** Show zoom level indicator */
    showLevelIndicator?: boolean;
    /** Custom class for the container */
    class?: string;
  }

  let {
    showBreadcrumb = true,
    showLevelIndicator = true,
    class: className = '',
  }: Props = $props();

  // Track previous level for transition direction
  let previousLevel = $state<ViewLevel>('data-graph');

  // Determine animation direction based on level change
  let transitionDirection = $derived.by(() => {
    const current = $viewLevel;
    const prev = previousLevel;

    if (current === 'orchestration' && prev !== 'orchestration') {
      return 'zoom-out'; // Going up to orchestration
    } else if (current === 'data-graph' && prev === 'orchestration') {
      return 'zoom-in'; // Going down to data-graph
    } else if (current === 'group') {
      return 'zoom-in'; // Going into group
    } else if (prev === 'group' && current === 'data-graph') {
      return 'zoom-out'; // Coming out of group
    }
    return 'none';
  });

  // Track level changes
  $effect(() => {
    const level = $viewLevel;
    // Update previous level after a tick
    setTimeout(() => {
      previousLevel = level;
    }, 0);
  });

  // Restore view state on mount
  onMount(() => {
    restoreViewState();
  });

  // Keyboard shortcuts
  function handleKeyDown(event: KeyboardEvent) {
    // Escape to go back
    if (event.key === 'Escape' && !event.ctrlKey && !event.altKey) {
      navigateBack();
      event.preventDefault();
    }

    // Ctrl+[ to zoom out to orchestration
    if (event.ctrlKey && event.key === '[') {
      zoomToOrchestration();
      event.preventDefault();
    }
  }

  // Level indicator text
  const levelText: Record<ViewLevel, string> = {
    orchestration: 'Orchestration',
    'data-graph': 'Data Graph',
    group: 'Node Group',
  };
</script>

<svelte:window onkeydown={handleKeyDown} />

<div
  class="unified-graph-view {className}"
  class:animating={$isAnimating}
  class:zoom-in={transitionDirection === 'zoom-in'}
  class:zoom-out={transitionDirection === 'zoom-out'}
>
  <!-- Navigation breadcrumb -->
  {#if showBreadcrumb}
    <div class="breadcrumb-container" transition:fade={{ duration: 150 }}>
      <NavigationBreadcrumb />
    </div>
  {/if}

  <!-- Main graph container with transitions -->
  <div class="graph-container">
    <!-- Orchestration view (placeholder - will be implemented by Workstream C) -->
    {#if $viewLevel === 'orchestration'}
      <div
        class="view-layer orchestration-layer"
        in:scale={{ duration: $animationConfig.duration, start: 0.5, easing: cubicOut }}
        out:scale={{ duration: $animationConfig.duration, start: 2, easing: cubicOut }}
      >
        <div class="placeholder-view">
          <div class="placeholder-icon">⚙️</div>
          <h3>Orchestration View</h3>
          <p>Control flow graph - Coming from Workstream C</p>
          <p class="hint">Double-click a node to zoom into its data graph</p>
        </div>
      </div>
    {/if}

    <!-- Data graph view (current WorkflowGraph) -->
    {#if $viewLevel === 'data-graph'}
      <div
        class="view-layer data-graph-layer"
        in:scale={{ duration: $animationConfig.duration, start: 2, easing: cubicOut }}
        out:scale={{ duration: $animationConfig.duration, start: 0.5, easing: cubicOut }}
      >
        <WorkflowGraph />
      </div>
    {/if}

    <!-- Group view (placeholder - will be implemented by Workstream B) -->
    {#if $viewLevel === 'group'}
      <div
        class="view-layer group-layer"
        in:scale={{ duration: $animationConfig.duration, start: 2, easing: cubicOut }}
        out:scale={{ duration: $animationConfig.duration, start: 0.5, easing: cubicOut }}
      >
        <div class="placeholder-view">
          <div class="placeholder-icon">📦</div>
          <h3>Node Group View</h3>
          <p>Internal group nodes - Coming from Workstream B</p>
          <p class="hint">Press Escape or click back to exit group</p>
        </div>
      </div>
    {/if}
  </div>

  <!-- Level indicator -->
  {#if showLevelIndicator}
    <div class="level-indicator" transition:fade={{ duration: 150 }}>
      <span class="level-dot" class:orchestration={$viewLevel === 'orchestration'}
        class:data-graph={$viewLevel === 'data-graph'}
        class:group={$viewLevel === 'group'}
      ></span>
      <span class="level-text">{levelText[$viewLevel]}</span>
    </div>
  {/if}

  <!-- Zoom controls -->
  <div class="zoom-controls">
    <button
      class="zoom-button"
      onclick={() => zoomToOrchestration()}
      disabled={$viewLevel === 'orchestration' || $isAnimating}
      title="Zoom out to Orchestration (Ctrl+[)"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <circle cx="11" cy="11" r="8" />
        <path d="M21 21l-4.35-4.35M8 11h6" />
      </svg>
    </button>
    <button
      class="zoom-button"
      onclick={() => navigateBack()}
      disabled={$viewLevel === 'orchestration' || $isAnimating}
      title="Navigate back (Escape)"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <path d="M19 12H5M12 19l-7-7 7-7" />
      </svg>
    </button>
  </div>
</div>

<style>
  .unified-graph-view {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: transparent;
  }

  .animating {
    pointer-events: none;
  }

  .breadcrumb-container {
    position: absolute;
    top: 12px;
    left: 12px;
    z-index: 100;
  }

  .graph-container {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }

  .view-layer {
    position: absolute;
    inset: 0;
    transform-origin: center center;
    will-change: transform, opacity;
  }

  /* Placeholder styles for views not yet implemented */
  .placeholder-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #737373;
    text-align: center;
  }

  .placeholder-icon {
    font-size: 64px;
    margin-bottom: 16px;
    opacity: 0.5;
  }

  .placeholder-view h3 {
    margin: 0 0 8px;
    font-size: 24px;
    font-weight: 600;
    color: #a3a3a3;
  }

  .placeholder-view p {
    margin: 4px 0;
    font-size: 14px;
  }

  .placeholder-view .hint {
    margin-top: 24px;
    padding: 8px 16px;
    background: rgba(38, 38, 38, 0.8);
    border-radius: 6px;
    font-size: 12px;
    color: #525252;
  }

  /* Level indicator */
  .level-indicator {
    position: absolute;
    bottom: 12px;
    left: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: rgba(23, 23, 23, 0.9);
    backdrop-filter: blur(8px);
    border: 1px solid #404040;
    border-radius: 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #737373;
    z-index: 100;
  }

  .level-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #525252;
    transition: background 200ms ease;
  }

  .level-dot.orchestration {
    background: #8b5cf6;
  }

  .level-dot.data-graph {
    background: #10b981;
  }

  .level-dot.group {
    background: #f59e0b;
  }

  /* Zoom controls */
  .zoom-controls {
    position: absolute;
    bottom: 12px;
    right: 12px;
    display: flex;
    gap: 4px;
    z-index: 100;
  }

  .zoom-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: rgba(23, 23, 23, 0.9);
    backdrop-filter: blur(8px);
    border: 1px solid #404040;
    border-radius: 6px;
    color: #a3a3a3;
    cursor: pointer;
    transition: all 150ms ease;
  }

  .zoom-button:not(:disabled):hover {
    background: #262626;
    border-color: #525252;
    color: #e5e5e5;
  }

  .zoom-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .zoom-button:not(:disabled):active {
    transform: scale(0.95);
  }
</style>
