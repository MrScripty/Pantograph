<script lang="ts">
  // Read-only view of a saved workflow's graph and its backend stale facts,
  // reached from the I/O Inspector's no-run state ("Inspect a saved workflow
  // instead"). Nodes are native buttons laid over a decorative edge drawing at
  // the graph's own coordinates, so keyboard and screen-reader users select
  // them like any other control.
  import {
    formatSavedGraphNodeAccessibleLabel,
    type SavedGraphInspectionDisplayModel,
  } from './graphInspectionPresenters';
  import { buildStaleGraphFactCards, parseGraphCanvasViewBox } from './ioInspectorPresenters';

  let {
    model,
    selectedNodeId = null,
    onSelectNode,
  }: {
    model: SavedGraphInspectionDisplayModel;
    selectedNodeId?: string | null;
    onSelectNode?: (nodeId: string) => void;
  } = $props();

  let box = $derived(parseGraphCanvasViewBox(model.canvas.viewBox));
  let selectedCards = $derived(buildStaleGraphFactCards(model.selectedNodeDiagnostics));
</script>

<div class="saved-graph">
  <div class="canvas" aria-label="Saved workflow graph" role="group">
    <div class="plane" style:width="{box.width}px" style:height="{box.height}px">
      <svg
        class="edges"
        width={box.width}
        height={box.height}
        viewBox="{box.x} {box.y} {box.width} {box.height}"
        aria-hidden="true"
      >
        {#each model.canvas.edges as edge (edge.id)}
          <line
            x1={edge.sourceX}
            y1={edge.sourceY}
            x2={edge.targetX}
            y2={edge.targetY}
            class:stale={edge.staleDiagnosticCount > 0}
          />
        {/each}
      </svg>
      {#each model.canvas.nodes as node (node.id)}
        <button
          type="button"
          class="gnode"
          style:left="{node.x - box.x}px"
          style:top="{node.y - box.y}px"
          style:width="{node.width}px"
          style:height="{node.height}px"
          aria-current={node.id === selectedNodeId ? 'true' : undefined}
          aria-label={formatSavedGraphNodeAccessibleLabel(node)}
          onclick={() => onSelectNode?.(node.id)}
        >
          <span class="gname">{node.id}</span>
          <span class="pg-mono pg-muted gtype">{node.nodeType}</span>
          {#if node.staleBadgeLabel}
            <span class="pg-small gstale">{node.staleBadgeLabel}</span>
          {/if}
        </button>
      {/each}
    </div>
  </div>

  <aside class="facts" aria-label="Stale graph facts">
    <h3 class="facts-title">Stale graph facts <span class="pg-n">{model.diagnostics.length}</span></h3>
    {#if model.selectedNodeId}
      <div class="pg-mono facts-node">{model.selectedNodeId}</div>
      {#if selectedCards.length === 0}
        <div class="pg-small pg-muted">The engine reports no stale facts for this node.</div>
      {:else}
        <div class="cards">
          {#each selectedCards as card (card.key)}
            <div class="fact">
              <div><b class="fact-title">{card.title}</b></div>
              <div class="pg-small pg-muted">{card.code}</div>
              <div class="pg-small fact-text">{card.explanation}</div>
              {#if card.suggestion}
                <div class="pg-small pg-muted fact-hint">{card.suggestion}</div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {:else}
      <div class="pg-small pg-muted">Select a node to see the engine's stale facts for it.</div>
    {/if}
  </aside>
</div>

<style>
  .saved-graph {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 300px);
    height: 100%;
    min-height: 0;
  }

  .canvas {
    min-height: 0;
    overflow: auto;
    background: var(--pg-panel-sunken);
  }

  .plane {
    position: relative;
  }

  .edges {
    position: absolute;
    inset: 0;
  }

  .edges line {
    stroke: var(--pg-border-strong);
    stroke-width: 1.5;
  }

  .edges line.stale {
    stroke-dasharray: 5 4;
    stroke: var(--pg-text-soft);
  }

  .gnode {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 10px 12px;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background: var(--pg-panel);
    color: var(--pg-text);
    text-align: left;
    cursor: pointer;
    overflow: hidden;
  }

  .gnode:hover {
    border-color: var(--pg-border-strong);
  }

  .gnode[aria-current='true'] {
    background: var(--pg-panel-selected);
    box-shadow: inset 3px 0 0 var(--pg-text);
  }

  .gname {
    font-weight: 700;
    color: var(--pg-text-bright);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .gtype {
    font-size: 11px;
  }

  .gstale {
    margin-top: auto;
    color: var(--pg-text-soft);
  }

  .facts {
    min-height: 0;
    overflow: auto;
    padding: 12px 14px;
    border-left: 1px solid var(--pg-border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .facts-title {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .facts-node {
    color: var(--pg-text-soft);
  }

  .cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* ErrorCard shape, muted: these describe the saved graph, not a failure. */
  .fact {
    padding: 8px 10px;
    border: 1px solid var(--pg-border);
    border-left: 3px solid var(--pg-border-strong);
    border-radius: 4px;
    background: var(--pg-panel-head);
  }

  .fact-title {
    color: var(--pg-text-bright);
  }

  .fact-text {
    margin-top: 2px;
  }

  .fact-hint {
    margin-top: 4px;
  }
</style>
