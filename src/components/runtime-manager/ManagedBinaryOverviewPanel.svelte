<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import {
    managedRuntimeService,
    type ManagedBinaryCategory,
    type ManagedBinaryStatus,
  } from '../../services/managedRuntime';

  let binaries: ManagedBinaryStatus[] = $state([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  const CATEGORY_LABELS: Record<ManagedBinaryCategory, string> = {
    runtime_sidecar: 'Runtime Sidecars',
    media_tool: 'Media Tools',
    native_artifact: 'Native Artifacts',
  };

  onMount(() => {
    void loadBinaries();
  });

  async function loadBinaries(): Promise<void> {
    loading = true;
    error = null;
    try {
      binaries = await managedRuntimeService.listManagedBinaries();
    } catch (loadError) {
      error = String(loadError);
    } finally {
      loading = false;
    }
  }

  function binariesForCategory(category: ManagedBinaryCategory): ManagedBinaryStatus[] {
    return binaries.filter((binary) => binary.category === category);
  }

  function statusLabel(binary: ManagedBinaryStatus): string {
    if (binary.readiness_state === 'ready') return 'Ready';
    if (binary.install_state === 'system_provided') return 'System';
    return binary.readiness_state.replace(/_/g, ' ');
  }

  function versionLabel(binary: ManagedBinaryStatus): string {
    return (
      binary.selected_version ??
      binary.active_version ??
      binary.default_version ??
      binary.versions[0]?.version ??
      'Unselected'
    );
  }
</script>

<section class="mt-4 border-t border-neutral-900 pt-4">
  <div class="mb-3 flex items-center justify-between gap-3">
    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wide text-neutral-400">
        Unified Managed Binary State
      </h3>
    </div>
    <button
      type="button"
      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-2.5 py-1 text-xs text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
      onclick={() => loadBinaries()}
      disabled={loading}
    >
      <RefreshCw size={13} aria-hidden="true" class={loading ? 'animate-spin' : ''} />
      Refresh
    </button>
  </div>

  {#if error}
    <div class="rounded border border-red-900 bg-red-950/40 px-3 py-2 text-sm text-red-200" role="alert">
      {error}
    </div>
  {:else}
    <div class="overflow-hidden rounded border border-neutral-800">
      {#each Object.keys(CATEGORY_LABELS) as categoryKey (categoryKey)}
        {@const category = categoryKey as ManagedBinaryCategory}
        {@const categoryBinaries = binariesForCategory(category)}
        <div class="border-b border-neutral-900 last:border-b-0">
          <div class="bg-neutral-950 px-3 py-2 text-xs font-medium text-neutral-300">
            {CATEGORY_LABELS[category]}
          </div>
          {#if categoryBinaries.length === 0}
            <div class="px-3 py-2 text-sm text-neutral-500">No backend entries reported.</div>
          {:else}
            <div class="divide-y divide-neutral-900">
              {#each categoryBinaries as binary (binary.key)}
                <div class="grid gap-2 px-3 py-2 text-sm text-neutral-300 md:grid-cols-[minmax(9rem,1fr)_7rem_minmax(8rem,1fr)_minmax(10rem,2fr)] md:items-center">
                  <div class="font-medium text-neutral-100">{binary.display_name}</div>
                  <div class="capitalize text-neutral-400">{statusLabel(binary)}</div>
                  <div class="text-neutral-400">{versionLabel(binary)}</div>
                  <div class="truncate text-neutral-500" title={binary.unavailable_reason ?? binary.missing_files.join(', ')}>
                    {binary.unavailable_reason ?? binary.missing_files.join(', ') ?? ''}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</section>
