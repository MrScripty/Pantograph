<script lang="ts">
  import type { WorkflowRunGraphProjection } from '../../services/workflow/types';
  import {
    buildRunGraphSnapshotLayout,
    clampRunGraphScale,
    fitRunGraphScale,
    type RunGraphNodeStatusByNodeId,
    type RunGraphSnapshotNode,
  } from './runGraphPresenters';
  import StatusGlyph from './ui/StatusGlyph.svelte';

  /**
   * Read-only surface for a run graph projection (ADR-014). It renders the
   * captured snapshot and the node status projection; it never reads or writes
   * the editable graph store. Selection is reported through `onSelectNode` and
   * owned by the caller.
   */
  let {
    runGraph,
    nodeStatuses = {},
    statusesAvailable = true,
    selectedNodeId = null,
    onSelectNode = undefined,
  }: {
    runGraph: WorkflowRunGraphProjection;
    nodeStatuses?: RunGraphNodeStatusByNodeId;
    /** False when the node status projection could not be read; nodes then claim no status. */
    statusesAvailable?: boolean;
    selectedNodeId?: string | null;
    /** Without it the cards are not interactive. */
    onSelectNode?: (nodeId: string) => void;
  } = $props();

  const ZOOM_STEP = 0.15;

  let layout = $derived(
    buildRunGraphSnapshotLayout(
      runGraph.graph,
      nodeStatuses,
      runGraph.graph_diagnostics ?? [],
      statusesAvailable,
    ),
  );

  let viewportWidth = $state(0);
  let viewportHeight = $state(0);
  // A manual zoom belongs to the run it was set on; another run starts fitted.
  let zoom = $state<{ runId: string; value: number } | null>(null);
  let fitScale = $derived(
    fitRunGraphScale({ width: viewportWidth, height: viewportHeight }, layout),
  );
  let manualZoom = $derived(zoom && zoom.runId === runGraph.workflow_run_id ? zoom.value : null);
  let scale = $derived(manualZoom ?? fitScale);

  function zoomBy(delta: number): void {
    zoom = { runId: runGraph.workflow_run_id, value: clampRunGraphScale(scale + delta) };
  }

  function fit(): void {
    zoom = null;
  }

  function nodeAccessibleLabel(node: RunGraphSnapshotNode): string {
    const parts = [node.label, node.nodeType, node.stateLabel];
    if (node.note) {
      parts.push(node.note);
    }
    return parts.join(', ');
  }
</script>

{#snippet card(node: RunGraphSnapshotNode)}
  <span class="rg-head">
    <StatusGlyph state={node.glyph} small />
    <span class="rg-label" title={node.label}>{node.label}</span>
    <span class="rg-type" title={node.nodeType}>{node.nodeType}</span>
  </span>
  <span class="rg-body">
    {#each node.inputs as port (port.id)}
      <span class="rg-port rg-in" title={port.dataType ?? undefined}>{port.label}</span>
    {/each}
    {#each node.outputs as port (port.id)}
      <span class="rg-port rg-out" title={port.dataType ?? undefined}>{port.label}</span>
    {/each}
    {#if node.hiddenPortCount > 0}
      <span class="rg-more">{node.hiddenPortCount === 1 ? '1 more port' : `${node.hiddenPortCount} more ports`}</span>
    {/if}
    {#if node.note}
      <span class="rg-note" class:rg-note-danger={node.noteDanger} title={node.note}>{node.note}</span>
    {/if}
  </span>
{/snippet}

<div class="rg">
  <div
    class="rg-canvas"
    role="group"
    aria-label={`Graph of ${runGraph.workflow_id} v${runGraph.workflow_semantic_version} as run ${runGraph.workflow_run_id} used it, read-only`}
    bind:clientWidth={viewportWidth}
    bind:clientHeight={viewportHeight}
  >
    {#if layout.nodes.length === 0}
      <p class="rg-empty">This run captured no graph nodes.</p>
    {:else}
      <div class="rg-stage" style:width="{layout.width * scale}px" style:height="{layout.height * scale}px">
        <div
          class="rg-plane"
          style:width="{layout.width}px"
          style:height="{layout.height}px"
          style:transform="scale({scale})"
        >
          <svg class="rg-wires" width={layout.width} height={layout.height} aria-hidden="true">
            {#each layout.wires as wire (wire.id)}
              <path
                class="rg-wire"
                class:rg-wire-live={wire.kind === 'live'}
                class:rg-wire-failed={wire.kind === 'failed'}
                class:rg-wire-stale={wire.kind === 'stale'}
                d={wire.d}
              />
            {/each}
          </svg>
          {#each layout.nodes as node (node.id)}
            {#if onSelectNode}
              <button
                type="button"
                class="rg-node"
                style:left="{node.x}px"
                style:top="{node.y}px"
                style:width="{node.width}px"
                style:min-height="{node.height}px"
                aria-current={node.id === selectedNodeId ? 'true' : undefined}
                aria-label={nodeAccessibleLabel(node)}
                onclick={() => onSelectNode?.(node.id)}
              >
                {@render card(node)}
              </button>
            {:else}
              <div
                class="rg-node"
                style:left="{node.x}px"
                style:top="{node.y}px"
                style:width="{node.width}px"
                style:min-height="{node.height}px"
              >
                {@render card(node)}
              </div>
            {/if}
          {/each}
        </div>
      </div>
    {/if}
  </div>

  {#if layout.nodes.length > 0}
    <div class="rg-fab rg-legend">
      <span><StatusGlyph state="running" small /> running now</span>
      <span><StatusGlyph state="succeeded" small /> done in this run</span>
      <span><StatusGlyph state="failed" small /> failed</span>
      <span><StatusGlyph state="queued" small /> not reached</span>
    </div>
  {/if}

  {#if layout.nodes.length > 0}
    <div class="rg-fab rg-zoom" role="group" aria-label="Zoom">
      <button class="rg-pill" type="button" aria-label="Zoom in" title="Zoom in" onclick={() => zoomBy(ZOOM_STEP)}>+</button>
      <button class="rg-pill" type="button" aria-label="Zoom out" title="Zoom out" onclick={() => zoomBy(-ZOOM_STEP)}>−</button>
      <button
        class="rg-pill"
        type="button"
        aria-pressed={manualZoom === null}
        title="Fit the whole graph"
        onclick={fit}
      >
        Fit
      </button>
      <span class="rg-scale" aria-live="polite">{Math.round(scale * 100)}%</span>
    </div>
  {/if}
</div>

<style>
  .rg {
    position: relative;
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    height: 100%;
  }

  .rg-canvas {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: auto;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background-color: var(--pg-panel-head);
    background-image: radial-gradient(var(--pg-border) 1px, transparent 1px);
    background-size: 22px 22px;
  }

  .rg-empty {
    margin: 0;
    padding: 32px 24px;
    color: var(--pg-text-muted);
  }

  .rg-stage {
    position: relative;
  }

  .rg-plane {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }

  .rg-wires {
    position: absolute;
    inset: 0;
    overflow: visible;
  }

  .rg-wire {
    fill: none;
    stroke: var(--pg-border-strong);
    stroke-width: 1.5;
  }

  .rg-wire-live {
    stroke: var(--pg-accent);
    stroke-width: 2;
  }

  .rg-wire-failed {
    stroke: var(--pg-danger);
    stroke-dasharray: 4 3;
  }

  .rg-wire-stale {
    stroke-dasharray: 2 3;
  }

  .rg-node {
    position: absolute;
    display: flex;
    flex-direction: column;
    padding: 0;
    border: 1px solid var(--pg-scroll-thumb);
    border-radius: 4px;
    background: var(--pg-panel);
    color: var(--pg-text);
    text-align: left;
    font: inherit;
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.25);
  }

  button.rg-node {
    cursor: pointer;
  }

  button.rg-node:hover {
    border-color: var(--pg-border-strong);
  }

  .rg-node[aria-current='true'] {
    border-color: var(--pg-text);
    box-shadow: 0 0 0 1px var(--pg-text);
  }

  .rg-head {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 9px;
    border-bottom: 1px solid var(--pg-border);
    font-size: 12.5px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .rg-label {
    min-width: 0;
    flex: 0 1 auto;
    /* The name keeps its room; the type truncates first. */
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .rg-type {
    margin-left: auto;
    min-width: 0;
    flex: 0 1000 auto;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-family: var(--pg-font-mono);
    font-size: 11px;
    font-weight: 400;
    color: var(--pg-text-muted);
  }

  .rg-body {
    display: flex;
    flex-direction: column;
    padding: 6px 0;
  }

  .rg-port,
  .rg-more,
  .rg-note {
    position: relative;
    display: block;
    height: 22px;
    padding: 0 9px;
    font-size: 11.5px;
    line-height: 22px;
    color: var(--pg-text-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rg-port {
    overflow: visible;
  }

  .rg-out {
    text-align: right;
  }

  .rg-in::before,
  .rg-out::after {
    content: '';
    position: absolute;
    top: 50%;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--pg-panel-head);
    border: 1.5px solid var(--pg-border-strong);
    transform: translateY(-50%);
  }

  .rg-in::before {
    left: -5px;
  }

  .rg-out::after {
    right: -5px;
  }

  .rg-more {
    color: var(--pg-text-muted);
  }

  .rg-note {
    border-top: 1px solid var(--pg-border);
    color: var(--pg-text-soft);
  }

  .rg-note-danger {
    color: var(--pg-danger);
  }

  .rg-fab {
    position: absolute;
    display: flex;
    gap: 6px;
    padding: 6px;
    background: var(--pg-panel-sunken);
    border: 1px solid var(--pg-border);
    border-radius: 4px;
  }

  /* Bottom right: graphs flow left to right from the top, so this corner is the emptiest. */
  .rg-legend {
    right: 12px;
    bottom: 12px;
    flex-direction: column;
    gap: 4px;
    padding: 8px 10px;
    font-size: 11.5px;
    color: var(--pg-text-muted);
  }

  .rg-legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .rg-zoom {
    left: 12px;
    bottom: 12px;
    align-items: center;
  }

  .rg-pill {
    height: 26px;
    padding: 0 9px;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background: var(--pg-panel);
    color: var(--pg-text-soft);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    font-size: 12px;
  }

  .rg-pill:hover {
    border-color: var(--pg-border-strong);
    color: var(--pg-text-bright);
  }

  .rg-pill[aria-pressed='true'] {
    background: var(--pg-text);
    color: var(--pg-ink);
    border-color: var(--pg-text);
  }

  .rg-scale {
    padding: 0 4px;
    font-family: var(--pg-font-mono);
    font-size: 11px;
    color: var(--pg-text-muted);
  }
</style>
