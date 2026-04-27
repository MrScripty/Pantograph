<script lang="ts">
  import type { WorkflowRunGraphProjection } from '../../services/workflow/types';
  import {
    buildRunGraphCanvasModel,
    buildRunGraphEdgeRows,
    buildRunGraphNodeRows,
    formatRunGraphCountLabel,
    formatRunGraphTimestamp,
    resolveRunGraphCounts,
    resolveRunGraphPresentationLabel,
    type RunGraphNodeArtifactSummaryByNodeId,
  } from './runGraphPresenters';

  let {
    runGraph,
    artifactSummaries = {},
  }: {
    runGraph: WorkflowRunGraphProjection;
    artifactSummaries?: RunGraphNodeArtifactSummaryByNodeId;
  } = $props();

  let canvas = $derived(buildRunGraphCanvasModel(runGraph.graph, artifactSummaries));
  let counts = $derived(resolveRunGraphCounts(runGraph.graph));
  let nodeRows = $derived(buildRunGraphNodeRows(runGraph, artifactSummaries));
  let edgeRows = $derived(buildRunGraphEdgeRows(runGraph));
  let presentationLabel = $derived(resolveRunGraphPresentationLabel(runGraph));

  function compactValue(value: string, maxLength = 18): string {
    if (value.length <= maxLength) {
      return value;
    }
    return `${value.slice(0, maxLength - 3)}...`;
  }
</script>

<div class="grid min-h-0 flex-1 grid-cols-1 overflow-hidden xl:grid-cols-[minmax(0,1fr)_28rem]">
  <div class="min-h-0 overflow-auto">
    <div class="border-b border-neutral-900 px-4 py-3">
      <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-neutral-500">
        <span>{formatRunGraphCountLabel(counts)}</span>
        <span>{presentationLabel}</span>
        <span>Snapshot {formatRunGraphTimestamp(runGraph.snapshot_created_at_ms)}</span>
      </div>
    </div>

    <div class="p-4">
      <div class="h-[30rem] overflow-hidden rounded border border-neutral-800 bg-neutral-950">
        {#if canvas.nodes.length === 0}
          <div class="flex h-full items-center justify-center text-sm text-neutral-500">No graph nodes captured</div>
        {:else}
          <svg
            role="img"
            aria-label="Captured workflow graph"
            class="h-full w-full"
            viewBox={canvas.viewBox}
          >
            <defs>
              <marker
                id="run-graph-arrow"
                markerHeight="8"
                markerWidth="8"
                orient="auto"
                refX="7"
                refY="4"
              >
                <path d="M 0 0 L 8 4 L 0 8 z" fill="#38bdf8" opacity="0.85" />
              </marker>
            </defs>
            {#each canvas.edges as edge (edge.id)}
              <line
                x1={edge.sourceX}
                y1={edge.sourceY}
                x2={edge.targetX}
                y2={edge.targetY}
                stroke="#38bdf8"
                stroke-width="2"
                stroke-opacity="0.65"
                marker-end="url(#run-graph-arrow)"
              />
            {/each}
            {#each canvas.nodes as node (node.id)}
              <g transform={`translate(${node.x}, ${node.y})`}>
                <rect
                  width={node.width}
                  height={node.height}
                  rx="6"
                  fill="#171717"
                  stroke={node.hasOutputArtifacts ? '#22c55e' : '#404040'}
                  stroke-width={node.hasOutputArtifacts ? '2' : '1'}
                />
                <text x="14" y="26" fill="#f5f5f5" font-size="13" font-family="ui-monospace, monospace">
                  {compactValue(node.id)}
                </text>
                <text x="14" y="48" fill="#a3a3a3" font-size="12" font-family="ui-sans-serif, system-ui">
                  {compactValue(node.nodeType, 22)}
                </text>
                {#if node.artifactCount > 0}
                  <rect
                    x="14"
                    y="58"
                    width="118"
                    height="16"
                    rx="4"
                    fill={node.hasOutputArtifacts ? '#052e16' : '#262626'}
                    stroke={node.hasOutputArtifacts ? '#16a34a' : '#525252'}
                    stroke-width="1"
                  />
                  <text x="20" y="70" fill="#d4d4d4" font-size="10" font-family="ui-sans-serif, system-ui">
                    {compactValue(node.artifactSummaryLabel, 18)}
                  </text>
                {/if}
              </g>
            {/each}
          </svg>
        {/if}
      </div>
    </div>
  </div>

  <aside class="min-h-0 overflow-auto border-l border-neutral-800 bg-neutral-950/80">
    <div class="border-b border-neutral-900 p-4">
      <h2 class="text-sm font-semibold text-neutral-100">Version</h2>
      <dl class="mt-3 space-y-3 text-xs">
        <div>
          <dt class="text-neutral-500">Workflow</dt>
          <dd class="mt-1 truncate font-mono text-neutral-200" title={runGraph.workflow_id}>
            {runGraph.workflow_id}
          </dd>
        </div>
        <div>
          <dt class="text-neutral-500">Semantic Version</dt>
          <dd class="mt-1 text-neutral-200">{runGraph.workflow_semantic_version}</dd>
        </div>
        <div>
          <dt class="text-neutral-500">Version ID</dt>
          <dd class="mt-1 truncate font-mono text-neutral-200" title={runGraph.workflow_version_id}>
            {runGraph.workflow_version_id}
          </dd>
        </div>
        <div>
          <dt class="text-neutral-500">Execution Fingerprint</dt>
          <dd class="mt-1 truncate font-mono text-neutral-200" title={runGraph.workflow_execution_fingerprint}>
            {runGraph.workflow_execution_fingerprint}
          </dd>
        </div>
        <div>
          <dt class="text-neutral-500">Presentation Revision</dt>
          <dd class="mt-1 truncate font-mono text-neutral-200" title={runGraph.workflow_presentation_revision_id}>
            {runGraph.workflow_presentation_revision_id}
          </dd>
        </div>
      </dl>
    </div>

    <div class="border-b border-neutral-900 p-4">
      <h2 class="text-sm font-semibold text-neutral-100">Nodes</h2>
      <div class="mt-3 max-h-72 overflow-auto rounded border border-neutral-800">
        <table class="w-full text-left text-xs">
          <thead class="sticky top-0 bg-neutral-950 text-neutral-500">
            <tr>
              <th class="px-3 py-2 font-medium">Node</th>
              <th class="px-3 py-2 font-medium">Contract</th>
              <th class="px-3 py-2 font-medium">I/O</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-neutral-900">
            {#each nodeRows as node (node.nodeId)}
              <tr>
                <td class="max-w-[12rem] px-3 py-2">
                  <div class="truncate font-mono text-neutral-200" title={node.nodeId}>{node.nodeId}</div>
                  <div class="truncate text-neutral-500" title={node.nodeType}>{node.nodeType}</div>
                  <div class="truncate text-neutral-600" title={node.positionLabel}>{node.positionLabel}</div>
                </td>
                <td class="max-w-[10rem] px-3 py-2">
                  <div class="truncate text-neutral-300" title={node.contractVersion}>{node.contractVersion}</div>
                  <div class="truncate font-mono text-neutral-600" title={node.behaviorDigest}>
                    {node.behaviorDigest}
                  </div>
                  <div class="truncate text-neutral-600" title={node.settingsState}>{node.settingsState}</div>
                </td>
                <td class="max-w-[9rem] px-3 py-2">
                  <div
                    class={node.hasOutputArtifacts ? 'truncate text-green-300' : 'truncate text-neutral-400'}
                    title={node.artifactSummaryLabel}
                  >
                    {node.artifactSummaryLabel}
                  </div>
                  <div class="truncate text-neutral-600" title={node.artifactDetailLabel}>
                    {node.artifactDetailLabel}
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>

    <div class="p-4">
      <h2 class="text-sm font-semibold text-neutral-100">Edges</h2>
      {#if edgeRows.length === 0}
        <div class="mt-3 text-xs text-neutral-500">No graph edges captured</div>
      {:else}
        <div class="mt-3 space-y-2">
          {#each edgeRows as edge (edge.edgeId)}
            <div class="rounded border border-neutral-800 bg-neutral-900/50 p-3 text-xs">
              <div class="truncate font-mono text-neutral-300" title={edge.edgeId}>{edge.edgeId}</div>
              <div class="mt-2 truncate text-neutral-500" title={`${edge.source} -> ${edge.target}`}>
                {edge.source} -> {edge.target}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </aside>
</div>
