<script lang="ts">
  import { onMount } from 'svelte';
  import {
    managedRuntimeService,
    type ManagedDependencyCategory,
    type ManagedDependencyKey,
    type ManagedDependencyStatus,
  } from '../../services/managedRuntime';
  import { StatusGlyph } from '../workbench/ui';
  import { managedDependencyGlyph, sentenceCaseState } from './managedRuntimePresenters';

  let dependencies: ManagedDependencyStatus[] = $state([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  const CATEGORY_LABELS: Record<ManagedDependencyCategory, string> = {
    runtime_sidecar: 'Runtime sidecars',
    media_tool: 'Media tools',
    native_artifact: 'Native artifacts',
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

<section class="flex flex-col gap-3 border-t border-pg-border-soft pt-4" aria-labelledby="managed-dependency-state-title">
  <div class="flex items-center justify-between gap-3">
    <h3 id="managed-dependency-state-title" class="m-0 text-[13px] font-bold text-pg-text-bright">
      Managed dependency state
    </h3>
    <button type="button" class="pg-btn" onclick={() => loadBinaries()} disabled={loading}>
      {loading ? 'Refreshing dependency state' : 'Refresh dependency state'}
    </button>
  </div>

  {#if error}
    <p class="m-0 pg-muted pg-small" role="alert">{error}</p>
  {:else}
    <div class="overflow-hidden rounded border border-pg-border">
      {#each Object.keys(CATEGORY_LABELS) as categoryKey (categoryKey)}
        {@const category = categoryKey as ManagedDependencyCategory}
        {@const categoryDependencies = dependenciesForCategory(category)}
        <div class="border-t border-pg-border-soft first:border-t-0">
          <h4 class="m-0 bg-pg-panel-head px-3 py-2 text-[12.5px] font-bold text-pg-text-soft">
            {CATEGORY_LABELS[category]}
          </h4>
          {#if categoryDependencies.length === 0}
            <p class="m-0 border-t border-pg-border-soft px-3 py-2 pg-muted pg-small">No backend entries reported.</p>
          {:else}
            <ul class="m-0 list-none p-0">
              {#each categoryDependencies as dependency (dependencyStableKey(dependency.key))}
                {@const version = versionLabel(dependency)}
                <li class="grid gap-2 border-t border-pg-border-soft px-3 py-2 text-[13px] md:grid-cols-[minmax(9rem,1fr)_8rem_minmax(8rem,1fr)_minmax(10rem,2fr)] md:items-center">
                  <span class="text-pg-text-bright">{dependency.display_name}</span>
                  <span class="flex items-center gap-1.5">
                    <StatusGlyph
                      state={managedDependencyGlyph(dependency.readiness_state, dependency.install_state)}
                      small
                    />
                    <span>{sentenceCaseState(statusLabel(dependency))}</span>
                  </span>
                  <span class="pg-muted" class:pg-mono={version !== 'Unselected'}>{version}</span>
                  <span
                    class="truncate pg-muted"
                    title={dependency.unavailable_reason ?? dependency.missing_files.join(', ')}
                  >
                    {dependency.unavailable_reason ?? dependency.missing_files.join(', ') ?? ''}
                  </span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</section>
