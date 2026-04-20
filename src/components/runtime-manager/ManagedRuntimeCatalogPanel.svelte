<script lang="ts">
  import type {
    ManagedRuntimeManagerRuntimeView,
    ManagedRuntimeVersionStatus,
  } from '../../services/managedRuntime';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
    selectableVersions: ManagedRuntimeVersionStatus[];
    selectionUpdating: boolean;
    installRequested: boolean;
    installingVersion: string | null;
    onUpdateSelected: (version: string | null) => Promise<void>;
    onUpdateDefault: (version: string | null) => Promise<void>;
    onInstallVersion: (version: string | null) => Promise<void>;
    versionBadgeLabel: (version: ManagedRuntimeVersionStatus) => string;
  };

  let {
    runtime,
    selectableVersions,
    selectionUpdating,
    installRequested,
    installingVersion,
    onUpdateSelected,
    onUpdateDefault,
    onInstallVersion,
    versionBadgeLabel,
  }: Props = $props();
</script>

<div>
  <h5 class="text-xs uppercase tracking-wider text-neutral-500">Version Policy</h5>
  {#if selectableVersions.length > 0}
    <div class="mt-2 space-y-2">
      <label class="block text-xs text-neutral-400" for={`${runtime.id}-selected-version`}>
        Selected version
      </label>
      <select
        id={`${runtime.id}-selected-version`}
        class="w-full rounded border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-sm text-neutral-200"
        value={runtime.selection.selected_version ?? ''}
        disabled={selectionUpdating}
        onchange={(event) =>
          onUpdateSelected((event.currentTarget as HTMLSelectElement).value || null)}
      >
        <option value="">Automatic</option>
        {#each selectableVersions as version (version.display_label)}
          <option value={version.version ?? ''}>{version.display_label}</option>
        {/each}
      </select>

      <label class="block text-xs text-neutral-400" for={`${runtime.id}-default-version`}>
        Default version
      </label>
      <select
        id={`${runtime.id}-default-version`}
        class="w-full rounded border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-sm text-neutral-200"
        value={runtime.selection.default_version ?? ''}
        disabled={selectionUpdating}
        onchange={(event) =>
          onUpdateDefault((event.currentTarget as HTMLSelectElement).value || null)}
      >
        <option value="">Unset</option>
        {#each selectableVersions as version (version.display_label)}
          <option value={version.version ?? ''}>{version.display_label}</option>
        {/each}
      </select>
    </div>
  {:else}
    <p class="mt-2 text-xs text-neutral-500">
      Install a runtime version before Pantograph can pin selection or default policy.
    </p>
  {/if}

  <h5 class="mt-4 text-xs uppercase tracking-wider text-neutral-500">Available Versions</h5>
  <ul class="mt-2 space-y-2">
    {#each runtime.versions as version (version.display_label)}
      <li class="rounded border border-neutral-800 bg-neutral-950/60 p-2 text-xs text-neutral-300">
        <div class="flex flex-wrap items-start justify-between gap-2">
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-medium">{version.display_label}</span>
              <span class="rounded bg-neutral-800 px-1.5 py-0.5 text-[10px] text-neutral-400">
                {versionBadgeLabel(version)}
              </span>
              {#if version.selected}
                <span class="rounded bg-blue-900/40 px-1.5 py-0.5 text-[10px] text-blue-300">Selected</span>
              {/if}
              {#if version.active}
                <span class="rounded bg-green-900/40 px-1.5 py-0.5 text-[10px] text-green-300">Active</span>
              {/if}
            </div>
            <div class="mt-1 text-[11px] text-neutral-500">
              {version.runtime_key} • {version.platform_key}
            </div>
          </div>

          {#if version.version && version.installable && version.install_state !== 'installed' && version.install_state !== 'system_provided' && !runtime.active_job}
            <button
              type="button"
              class="rounded border border-blue-700 px-2 py-1 text-[10px] text-blue-200 transition-colors hover:bg-blue-950/40 disabled:border-neutral-800 disabled:text-neutral-600"
              onclick={() => onInstallVersion(version.version)}
              disabled={installRequested}
            >
              {#if installRequested && installingVersion === version.version}
                Installing...
              {:else}
                Install
              {/if}
            </button>
          {/if}
        </div>
        {#if version.install_root}
          <div class="mt-1 break-all font-mono text-[11px] text-neutral-600">
            {version.install_root}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
</div>
