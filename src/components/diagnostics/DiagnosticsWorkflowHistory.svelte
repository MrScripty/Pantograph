<script lang="ts">
  import type { DiagnosticsWorkflowTimingHistory } from '../../services/diagnostics/types';
  import DiagnosticsTimingExpectation from './DiagnosticsTimingExpectation.svelte';

  type Props = {
    history?: DiagnosticsWorkflowTimingHistory | null;
  };

  let { history = null }: Props = $props();
  let nodes = $derived.by(() => {
    return Object.values(history?.nodes ?? {}).sort((left, right) =>
      left.nodeId.localeCompare(right.nodeId),
    );
  });
</script>

{#if history}
  <div class="h-full overflow-auto p-4">
    <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
      <div>
        <div class="text-sm font-medium text-neutral-100">
          {history.workflowId}
        </div>
        <div class="mt-1 text-xs text-neutral-500">
          Graph Fingerprint: {history.graphFingerprint ?? 'Unavailable'}
        </div>
      </div>
      <DiagnosticsTimingExpectation expectation={history.timingExpectation ?? null} align="right" />
    </div>

    {#if nodes.length > 0}
      <div class="overflow-hidden rounded border border-neutral-800">
        <table class="w-full table-fixed text-left text-xs">
          <thead class="bg-neutral-900/80 text-neutral-400">
            <tr>
              <th class="w-2/5 px-3 py-2 font-medium">Node</th>
              <th class="w-1/5 px-3 py-2 font-medium">Type</th>
              <th class="w-2/5 px-3 py-2 text-right font-medium">Duration</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-neutral-900">
            {#each nodes as node (node.nodeId)}
              <tr>
                <td class="break-all px-3 py-3 font-medium text-neutral-100">{node.nodeId}</td>
                <td class="break-all px-3 py-3 text-neutral-400">{node.nodeType ?? 'Unknown'}</td>
                <td class="px-3 py-3">
                  <DiagnosticsTimingExpectation expectation={node.timingExpectation ?? null} align="right" />
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <div class="flex h-40 items-center justify-center border border-neutral-800 text-sm text-neutral-500">
        No timing history is available for this workflow graph.
      </div>
    {/if}
  </div>
{:else}
  <div class="flex h-full items-center justify-center px-6 text-center text-sm text-neutral-500">
    No timing history is available for this workflow graph.
  </div>
{/if}
