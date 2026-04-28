<script lang="ts">
  import {
    buildNodeLabStatusRows,
    nodeLabUnavailableMessage,
  } from './nodeLabPresenters';

  const statusRows = buildNodeLabStatusRows();
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950 p-6">
  <h1 class="text-base font-semibold text-neutral-100">Node Editor</h1>
  <div class="mt-4 border border-neutral-800 bg-neutral-900/60 p-4 text-sm text-neutral-500">
    {nodeLabUnavailableMessage()}
  </div>
  <dl class="mt-4 grid gap-3 text-sm sm:grid-cols-3">
    {#each statusRows as row (row.label)}
      <div class="border border-neutral-800 bg-neutral-900/40 p-3">
        <dt class="text-xs uppercase tracking-[0.18em] text-neutral-600">{row.label}</dt>
        <dd class="mt-2 text-neutral-300">{row.value}</dd>
      </div>
    {/each}
  </dl>
</section>
