<script lang="ts">
  import { tick } from 'svelte';
  import {
    formatSavedGraphNodeAccessibleLabel,
    isSavedGraphNodeSelectionKey,
    savedGraphNodeFocusDomId,
    type SavedGraphInspectionDisplayModel,
  } from './graphInspectionPresenters';
  import { runGraphStaleDiagnosticClass } from './runGraphPresenters';

  let {
    model,
    selectedNodeId = null,
    onSelectNode,
  }: {
    model: SavedGraphInspectionDisplayModel;
    selectedNodeId?: string | null;
    onSelectNode?: (nodeId: string) => void;
  } = $props();

  function selectNode(nodeId: string): void {
    onSelectNode?.(nodeId);
    void restoreSelectedNodeFocus(nodeId);
  }

  async function restoreSelectedNodeFocus(nodeId: string): Promise<void> {
    await tick();
    document.getElementById(savedGraphNodeFocusDomId(nodeId))?.focus();
  }

  function handleNodeKeydown(event: KeyboardEvent, nodeId: string): void {
    if (!isSavedGraphNodeSelectionKey(event.key)) {
      return;
    }
    event.preventDefault();
    selectNode(nodeId);
  }

  function nodeStroke(node: (typeof model.canvas.nodes)[number]): string {
    if (node.id === selectedNodeId) {
      return '#22d3ee';
    }
    if (node.staleSeverity === 'error') {
      return '#f97316';
    }
    if (node.staleSeverity === 'warning') {
      return '#eab308';
    }
    if (node.staleSeverity === 'info') {
      return '#38bdf8';
    }
    return '#525252';
  }
</script>

<div class="grid h-full min-h-0 grid-cols-[minmax(0,1fr)_minmax(16rem,22rem)] overflow-hidden">
  <div class="min-h-0 overflow-auto bg-neutral-950">
    <svg
      class="h-full min-h-[18rem] w-full"
      viewBox={`${model.canvas.viewBox.x} ${model.canvas.viewBox.y} ${model.canvas.viewBox.width} ${model.canvas.viewBox.height}`}
      role="img"
      aria-label="Saved workflow graph inspection"
    >
      {#each model.canvas.edges as edge (edge.id)}
        <line
          x1={edge.sourceX}
          y1={edge.sourceY}
          x2={edge.targetX}
          y2={edge.targetY}
          stroke={edge.staleSeverity === 'error' ? '#fb923c' : edge.staleSeverity === 'warning' ? '#facc15' : '#38bdf8'}
          stroke-width={edge.staleDiagnosticCount > 0 ? '3' : '2'}
          stroke-opacity={edge.staleDiagnosticCount > 0 ? '0.9' : '0.65'}
        />
      {/each}

      {#each model.canvas.nodes as node (node.id)}
        <g
          transform={`translate(${node.x}, ${node.y})`}
          id={savedGraphNodeFocusDomId(node.id)}
          role="button"
          tabindex="0"
          aria-label={formatSavedGraphNodeAccessibleLabel(node)}
          class="cursor-pointer outline-none"
          onclick={() => selectNode(node.id)}
          onkeydown={(event) => handleNodeKeydown(event, node.id)}
        >
          <rect
            width={node.width}
            height={node.height}
            rx="8"
            fill="#171717"
            stroke={nodeStroke(node)}
            stroke-width={node.id === selectedNodeId || node.staleDiagnosticCount > 0 ? '2.5' : '1.5'}
          />
          <text
            x="14"
            y="26"
            fill="#f5f5f5"
            font-size="13"
            font-family="ui-sans-serif, system-ui"
          >
            {node.id}
          </text>
          <text
            x="14"
            y="48"
            fill="#a3a3a3"
            font-size="11"
            font-family="ui-monospace, SFMono-Regular, Menlo, monospace"
          >
            {node.nodeType}
          </text>
          {#if node.staleDiagnosticCount > 0}
            <circle
              cx={node.width - 18}
              cy="22"
              r="7"
              fill={node.staleSeverity === 'error' ? '#7c2d12' : node.staleSeverity === 'warning' ? '#713f12' : '#164e63'}
              stroke={node.staleSeverity === 'error' ? '#fb923c' : node.staleSeverity === 'warning' ? '#facc15' : '#67e8f9'}
              stroke-width="1.5"
            />
            <text
              x={node.width - 18}
              y="26"
              text-anchor="middle"
              fill="#fff7ed"
              font-size="11"
              font-family="ui-sans-serif, system-ui"
            >
              !
            </text>
          {/if}
        </g>
      {/each}
    </svg>
  </div>

  <aside class="min-h-0 overflow-auto border-l border-neutral-800 bg-neutral-950 p-4 text-sm">
    <h2 class="text-sm font-semibold text-neutral-100">Stale Graph Facts</h2>
    <div class="mt-1 text-xs text-neutral-500">{model.diagnostics.length} backend diagnostics</div>

    {#if model.selectedNodeId}
      <div class="mt-4 rounded border border-neutral-800 bg-neutral-900/60 p-3">
        <div class="font-mono text-xs text-neutral-100">{model.selectedNodeId}</div>
        {#if model.selectedNodeDiagnostics.length === 0}
          <div class="mt-2 text-xs text-neutral-500">No backend stale facts for this node.</div>
        {:else}
          <div class="mt-3 space-y-2">
            {#each model.selectedNodeDiagnostics as diagnostic (`${diagnostic.code}:${diagnostic.message}`)}
              <div class="rounded border border-neutral-800 bg-neutral-950 p-2">
                <span
                  class={`inline-flex rounded border px-2 py-0.5 text-[11px] ${runGraphStaleDiagnosticClass(diagnostic.severity) === 'error' ? 'border-orange-800 bg-orange-950 text-orange-100' : runGraphStaleDiagnosticClass(diagnostic.severity) === 'warning' ? 'border-yellow-800 bg-yellow-950 text-yellow-100' : 'border-cyan-800 bg-cyan-950 text-cyan-100'}`}
                >
                  {diagnostic.code}
                </span>
                <div class="mt-2 text-xs text-neutral-300">{diagnostic.message}</div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div class="mt-4 text-xs text-neutral-500">Select a graph node to inspect backend stale facts.</div>
    {/if}
  </aside>
</div>
