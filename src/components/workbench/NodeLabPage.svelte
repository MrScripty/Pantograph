<script lang="ts">
  import { onMount } from 'svelte';
  import { nodeLabService } from '../../services/workflow/customNodeAuthoring';
  import { NodeLabController, type NodeLabView } from './nodeLabController';
  import { buildNodeLabStatusRows, nodeLabTrustMessage } from './nodeLabPresenters';

  let view = $state<NodeLabView>({ catalog: null, draft: null, savedPath: '', busy: false, error: null, notice: '', pendingCleanup: 0 });
  let name = $state('custom-node-workflow');
  let path = $state('');
  let controller: NodeLabController;
  onMount(() => {
    controller = new NodeLabController(nodeLabService, state => { view = state; });
    void controller.loadCatalog();
    return () => { void controller.dispose(); };
  });
  const statusRows = buildNodeLabStatusRows();
</script>

<section class="h-full overflow-auto bg-neutral-950 p-6 text-sm" aria-label="NodeLab authoring" aria-busy={view.busy}>
  <h1 class="text-base font-semibold text-neutral-100">NodeLab</h1>
  <p class="mt-2 max-w-3xl text-neutral-400">{nodeLabTrustMessage()}</p>
  <dl class="mt-4 grid gap-3 sm:grid-cols-3">
    {#each statusRows as row (row.label)}
      <div class="border border-neutral-800 bg-neutral-900/40 p-3">
        <dt class="text-xs uppercase tracking-[0.18em] text-neutral-500">{row.label}</dt>
        <dd class="mt-2 text-neutral-300">{row.value}</dd>
      </div>
    {/each}
  </dl>
  {#if view.error}<p role="alert" class="mt-4 text-red-300">{view.error}</p>{/if}
  <p role="status" class="mt-3 text-cyan-200">{view.busy ? 'Working…' : view.notice}</p>
  {#if view.pendingCleanup}
    <p role="alert" class="mt-3 text-amber-200">{view.pendingCleanup} previous NodeLab sessions could not be closed. Close them before creating or reopening a draft.</p>
    <button type="button" class="mt-2 rounded border border-neutral-600 px-3 py-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-40" disabled={view.busy} onclick={() => controller.retryCleanup()}>Retry session cleanup</button>
  {/if}
  <h2 class="mt-5 font-medium text-neutral-200">Admitted nodes</h2>
  <button type="button" class="mt-2 rounded border border-neutral-600 px-3 py-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-40" disabled={view.busy} onclick={() => controller.loadCatalog()}>Refresh catalog</button>
  {#if view.catalog?.length === 0}
    <p class="mt-3 text-neutral-400">This host has no admitted custom nodes. A host maintainer must register a trusted implementation before creating a new draft.</p>
  {:else if view.catalog}
    <ul class="mt-3 grid gap-3 lg:grid-cols-2">
      {#each view.catalog as entry (entry.identity.node_type)}
        <li class="border border-neutral-700 p-4">
          <h3 class="font-medium text-neutral-100">{entry.label}</h3>
          <p class="mt-1 text-neutral-400">{entry.description}</p>
          <p class="mt-2 text-xs text-neutral-400">{entry.inputs.length} string inputs · {entry.outputs.length} string outputs · Trusted in-process</p>
          <details class="mt-2 break-all text-xs text-neutral-500">
            <summary class="cursor-pointer focus-visible:outline focus-visible:outline-cyan-400">Implementation identity</summary>
            <pre class="mt-2 whitespace-pre-wrap">{JSON.stringify(entry.identity, null, 2)}</pre>
          </details>
          <button type="button" class="mt-3 rounded border border-cyan-700 px-3 py-2 text-cyan-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-40" disabled={view.busy} onclick={() => controller.create(entry)}>Create {entry.label} draft</button>
        </li>
      {/each}
    </ul>
  {/if}
  <form class="mt-6 flex flex-wrap items-end gap-2" onsubmit={event => { event.preventDefault(); void controller.reopen(path); }}>
    <label class="grid flex-1 gap-1 text-neutral-300">Saved workflow path
      <input class="min-w-64 rounded border border-neutral-600 bg-neutral-900 p-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400" bind:value={path} required disabled={view.busy} />
    </label>
    <button class="rounded border border-neutral-600 px-3 py-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-40" disabled={view.busy || !path.trim()}>Reopen saved workflow</button>
  </form>
  {#if view.savedPath}<p class="mt-3 break-all text-neutral-300">Saved: <code>{view.savedPath}</code></p>{/if}
  {#if view.draft}
    <div class="mt-6 border border-neutral-700 p-4">
      <h2 class="font-medium text-neutral-100">Workflow draft</h2>
      <p class="mt-2 text-neutral-400">{view.draft.graph.nodes.length} nodes · {view.draft.graph.edges.length} connections. Saving preserves the selected implementation identity. Run saved workflows from Scheduler.</p>
      {#if view.draft.graph_diagnostics.length}
        <ul class="mt-3 text-amber-200" aria-label="Graph diagnostics">
          {#each view.draft.graph_diagnostics as diagnostic}
            <li>{diagnostic.blocking_submission ? 'Blocks execution: ' : ''}{diagnostic.message}</li>
          {/each}
        </ul>
      {:else}<p class="mt-3 text-emerald-200">Graph contract valid</p>{/if}
      {#each view.draft.graph.nodes.filter(node => node.node_type === 'text-input') as node (node.id)}
        <form class="mt-4" onsubmit={event => { event.preventDefault(); const input = new FormData(event.currentTarget).get('text'); if (typeof input === 'string') void controller.update(node, input); }}>
          <label class="grid gap-1 text-neutral-300">{typeof node.data.label === 'string' ? node.data.label : node.id}
            <textarea name="text" class="rounded border border-neutral-600 bg-neutral-900 p-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400" disabled={view.busy} value={typeof node.data.text === 'string' ? node.data.text : ''}></textarea>
          </label>
          <button class="mt-2 rounded border border-neutral-600 px-3 py-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-40" disabled={view.busy}>Apply input</button>
        </form>
      {/each}
      <form class="mt-5 flex flex-wrap items-end gap-2" onsubmit={event => { event.preventDefault(); void controller.save(name); }}>
        <label class="grid flex-1 gap-1 text-neutral-300">Workflow name
          <input class="rounded border border-neutral-600 bg-neutral-900 p-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400" bind:value={name} required disabled={view.busy} />
        </label>
        <button class="rounded border border-cyan-700 px-3 py-2 text-cyan-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-40" disabled={view.busy}>Save workflow</button>
      </form>
    </div>
  {/if}
</section>
