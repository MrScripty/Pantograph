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
  <div class="mt-2 overflow-hidden rounded border border-neutral-800 bg-neutral-950/50">
    <div class="max-h-72 overflow-auto">
      <table class="min-w-full table-fixed text-left text-xs text-neutral-300">
        <thead class="sticky top-0 bg-neutral-950 text-[11px] uppercase tracking-wider text-neutral-500">
          <tr>
            <th class="px-3 py-2 font-medium">Version</th>
            <th class="px-3 py-2 font-medium">Status</th>
            <th class="px-3 py-2 font-medium">Target</th>
            <th class="px-3 py-2 font-medium">Install Root</th>
            <th class="px-3 py-2 text-right font-medium">Action</th>
          </tr>
        </thead>
        <tbody>
          {#each runtime.versions as version (version.display_label)}
            <tr class="border-t border-neutral-800 align-top">
              <td class="px-3 py-2">
                <div class="font-medium text-neutral-100">{version.display_label}</div>
                <div class="mt-1 flex flex-wrap gap-1.5">
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
              </td>
              <td class="px-3 py-2 text-neutral-400">
                <div class="capitalize">{version.install_state.replaceAll('_', ' ')}</div>
                <div class="mt-1 capitalize text-[11px] text-neutral-500">
                  {version.readiness_state.replaceAll('_', ' ')}
                </div>
              </td>
              <td class="px-3 py-2 text-neutral-400">
                <div>{version.runtime_key}</div>
                <div class="mt-1 text-[11px] text-neutral-500">{version.platform_key}</div>
              </td>
              <td class="px-3 py-2">
                {#if version.install_root}
                  <div
                    class="truncate font-mono text-[11px] text-neutral-500"
                    title={version.install_root}
                  >
                    {version.install_root}
                  </div>
                {:else}
                  <span class="text-neutral-600">Not installed</span>
                {/if}
              </td>
              <td class="px-3 py-2 text-right">
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
                {:else}
                  <span class="text-[11px] text-neutral-600">No action</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>
