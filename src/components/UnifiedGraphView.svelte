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
    tabOutOfGroup,
    navigateBack,
    currentDataGraphId,
    groupStack,
    type ViewLevel,
  } from '../stores/viewStore';
  import {
    currentOrchestration,
    selectedOrchestrationNodeId,
    loadOrchestrationNodeTypes,
  } from '../stores/orchestrationStore';
  import { nodeGroups, expandedGroupId } from '../stores/workflowStore';

  import ZoomTransition from './ZoomTransition.svelte';
  import NavigationBreadcrumb from './NavigationBreadcrumb.svelte';
  import WorkflowGraph from './WorkflowGraph.svelte';
  import OrchestrationGraph from './orchestration/OrchestrationGraph.svelte';

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

  // Restore view state on mount and load orchestration node types
  onMount(() => {
    restoreViewState();
    loadOrchestrationNodeTypes().catch(console.error);
  });

  // Handle double-click on DataGraph node in orchestration to zoom in
  function handleOrchestrationNodeDoubleClick(nodeId: string) {
    const orch = $currentOrchestration;
    if (!orch) return;

    const node = orch.nodes.find((n) => n.id === nodeId);
    if (!node) return;

    // Only zoom into DataGraph nodes
    if (node.nodeType === 'data_graph') {
      const dataGraphId = orch.dataGraphs[nodeId];
      if (dataGraphId) {
        zoomToDataGraph(nodeId, dataGraphId);
      }
    }
  }

  // Handle double-click on group node in data graph to tab in
  function handleGroupDoubleClick(groupId: string) {
    tabIntoGroup(groupId);
  }

  // Get current group being edited (if in group view)
  let currentGroup = $derived.by(() => {
    if ($viewLevel !== 'group' || $groupStack.length === 0) return null;
    const groupId = $groupStack[$groupStack.length - 1];
    return $nodeGroups.get(groupId) ?? null;
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

    // Ctrl+] to zoom into selected node (DataGraph or Group)
    if (event.ctrlKey && event.key === ']') {
      if ($viewLevel === 'orchestration' && $selectedOrchestrationNodeId) {
        handleOrchestrationNodeDoubleClick($selectedOrchestrationNodeId);
      } else if ($viewLevel === 'data-graph' && $expandedGroupId === null) {
        // If a group node is selected, tab into it
        // (selection would come from WorkflowGraph component)
      }
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
    <!-- Orchestration view -->
    {#if $viewLevel === 'orchestration'}
      <div
        class="view-layer orchestration-layer"
        in:scale={{ duration: $animationConfig.duration, start: 0.5, easing: cubicOut }}
        out:scale={{ duration: $animationConfig.duration, start: 2, easing: cubicOut }}
      >
        {#if $currentOrchestration}
          <OrchestrationGraph
            onNodeDoubleClick={handleOrchestrationNodeDoubleClick}
          />
        {:else}
          <div class="placeholder-view">
            <div class="placeholder-icon">⚙️</div>
            <h3>No Orchestration Loaded</h3>
            <p>Create or load an orchestration to see the control flow graph</p>
            <p class="hint">Use the orchestration panel to manage workflows</p>
          </div>
        {/if}
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

    <!-- Group view - shows the internal nodes of a group -->
    {#if $viewLevel === 'group'}
      <div
        class="view-layer group-layer"
        in:scale={{ duration: $animationConfig.duration, start: 2, easing: cubicOut }}
        out:scale={{ duration: $animationConfig.duration, start: 0.5, easing: cubicOut }}
      >
        {#if currentGroup}
          <div class="group-editor">
            <div class="group-header">
              <span class="group-icon">📦</span>
              <h3>{currentGroup.name}</h3>
              {#if currentGroup.description}
                <p class="group-description">{currentGroup.description}</p>
              {/if}
              <button class="exit-group-btn" onclick={() => tabOutOfGroup()}>
                Exit Group
              </button>
            </div>
            <!-- Group editing view - displays group info and node preview -->
            <div class="group-graph-container">
              <div class="group-info">
                <div class="info-row">
                  <span class="info-label">Nodes:</span>
                  <span class="info-value">{currentGroup.nodes.length}</span>
                </div>
                <div class="info-row">
                  <span class="info-label">Edges:</span>
                  <span class="info-value">{currentGroup.edges.length}</span>
                </div>
                <div class="info-row">
                  <span class="info-label">Exposed Inputs:</span>
                  <span class="info-value">{currentGroup.exposed_inputs.length}</span>
                </div>
                <div class="info-row">
                  <span class="info-label">Exposed Outputs:</span>
                  <span class="info-value">{currentGroup.exposed_outputs.length}</span>
                </div>
              </div>
              <div class="group-nodes-preview">
                <h4>Nodes in Group</h4>
                {#each currentGroup.nodes as node}
                  <div class="preview-node">
                    <span class="node-type">{node.node_type}</span>
                    <span class="node-id">{node.id}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {:else}
          <div class="placeholder-view">
            <div class="placeholder-icon">📦</div>
            <h3>Group Not Found</h3>
            <p>The selected group could not be loaded</p>
            <p class="hint">Press Escape to go back</p>
          </div>
        {/if}
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

  /* Group editor styles */
  .group-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1a1a1a;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid #333;
    background: linear-gradient(to right, rgba(245, 158, 11, 0.1), transparent);
  }

  .group-icon {
    font-size: 24px;
  }

  .group-header h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: #f59e0b;
  }

  .group-description {
    margin: 0;
    font-size: 12px;
    color: #888;
    margin-left: auto;
  }

  .group-graph-container {
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 16px;
    overflow-y: auto;
  }

  .exit-group-btn {
    margin-left: auto;
    padding: 6px 12px;
    background: rgba(245, 158, 11, 0.2);
    border: 1px solid #f59e0b;
    border-radius: 4px;
    color: #f59e0b;
    font-size: 12px;
    cursor: pointer;
    transition: all 150ms ease;
  }

  .exit-group-btn:hover {
    background: rgba(245, 158, 11, 0.3);
  }

  .group-info {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    padding: 16px;
    background: #252525;
    border-radius: 8px;
    border: 1px solid #333;
  }

  .info-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .info-label {
    color: #888;
    font-size: 12px;
  }

  .info-value {
    color: #f59e0b;
    font-weight: 600;
  }

  .group-nodes-preview {
    background: #252525;
    border-radius: 8px;
    border: 1px solid #333;
    padding: 16px;
  }

  .group-nodes-preview h4 {
    margin: 0 0 12px 0;
    font-size: 14px;
    color: #a3a3a3;
  }

  .preview-node {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    background: #1a1a1a;
    border-radius: 4px;
    margin-bottom: 8px;
    border: 1px solid #333;
  }

  .preview-node:last-child {
    margin-bottom: 0;
  }

  .node-type {
    padding: 2px 8px;
    background: rgba(59, 130, 246, 0.2);
    border-radius: 4px;
    color: #3b82f6;
    font-size: 11px;
    font-family: monospace;
  }

  .node-id {
    color: #666;
    font-size: 11px;
    font-family: monospace;
  }
</style>
