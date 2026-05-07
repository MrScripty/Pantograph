<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import {
    managedRuntimeService,
    type ManagedDependencyCategory,
    type ManagedDependencyKey,
    type ManagedDependencyStatus,
  } from '../../services/managedRuntime';

  let dependencies: ManagedDependencyStatus[] = $state([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  const CATEGORY_LABELS: Record<ManagedDependencyCategory, string> = {
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
      dependencies = await managedRuntimeService.listManagedDependencies();
    } catch (loadError) {
      error = String(loadError);
    } finally {
      loading = false;
    }
  }

  function dependenciesForCategory(
    category: ManagedDependencyCategory
  ): ManagedDependencyStatus[] {
    return dependencies.filter((dependency) => dependency.category === category);
  }

  function statusLabel(dependency: ManagedDependencyStatus): string {
    if (dependency.readiness_state === 'ready') return 'Ready';
    if (dependency.install_state === 'system_provided') return 'System';
    return dependency.readiness_state.replace(/_/g, ' ');
  }

  function versionLabel(dependency: ManagedDependencyStatus): string {
    return (
      dependency.selection.selected_version ??
      dependency.selection.active_version ??
      dependency.selection.default_version ??
      dependency.versions[0]?.version ??
      'Unselected'
    );
  }

  function dependencyStableKey(key: ManagedDependencyKey): string {
    if ('runtime_sidecar' in key) return `runtime_sidecar:${key.runtime_sidecar}`;
    if ('media_tool' in key) return `media_tool:${key.media_tool}`;
    return `native_artifact:${key.native_artifact}`;
  }
</script>

<section class="mt-4 border-t border-neutral-900 pt-4">
  <div class="mb-3 flex items-center justify-between gap-3">
    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wide text-neutral-400">
        Managed Dependency State
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
        {@const category = categoryKey as ManagedDependencyCategory}
        {@const categoryDependencies = dependenciesForCategory(category)}
        <div class="border-b border-neutral-900 last:border-b-0">
          <div class="bg-neutral-950 px-3 py-2 text-xs font-medium text-neutral-300">
            {CATEGORY_LABELS[category]}
          </div>
          {#if categoryDependencies.length === 0}
            <div class="px-3 py-2 text-sm text-neutral-500">No backend entries reported.</div>
          {:else}
            <div class="divide-y divide-neutral-900">
              {#each categoryDependencies as dependency (dependencyStableKey(dependency.key))}
                <div class="grid gap-2 px-3 py-2 text-sm text-neutral-300 md:grid-cols-[minmax(9rem,1fr)_7rem_minmax(8rem,1fr)_minmax(10rem,2fr)] md:items-center">
                  <div class="font-medium text-neutral-100">{dependency.display_name}</div>
                  <div class="capitalize text-neutral-400">{statusLabel(dependency)}</div>
                  <div class="text-neutral-400">{versionLabel(dependency)}</div>
                  <div class="truncate text-neutral-500" title={dependency.unavailable_reason ?? dependency.missing_files.join(', ')}>
                    {dependency.unavailable_reason ?? dependency.missing_files.join(', ') ?? ''}
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
