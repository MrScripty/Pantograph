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
    removingVersion: string | null;
    onUpdateSelected: (version: string | null, runtimeVariantId: string | null) => Promise<void>;
    onUpdateDefault: (version: string | null, runtimeVariantId: string | null) => Promise<void>;
    onInstallVersion: (version: string | null) => Promise<void>;
    onRemoveVersion: (version: string) => Promise<void>;
    versionBadgeLabel: (version: ManagedRuntimeVersionStatus) => string;
  };

  let {
    runtime,
    selectableVersions,
    selectionUpdating,
    installRequested,
    installingVersion,
    removingVersion,
    onUpdateSelected,
    onUpdateDefault,
    onInstallVersion,
    onRemoveVersion,
    versionBadgeLabel,
  }: Props = $props();

  function versionOptionValue(version: string | null, runtimeVariantId: string | null): string {
    if (!version || !runtimeVariantId) {
      return '';
    }

    return `${encodeURIComponent(version)}:${encodeURIComponent(runtimeVariantId)}`;
  }

  function parseVersionOptionValue(value: string): [string | null, string | null] {
    if (!value) {
      return [null, null];
    }

    const separatorIndex = value.indexOf(':');
    if (separatorIndex < 0) {
      return [null, null];
    }

    return [
      decodeURIComponent(value.slice(0, separatorIndex)),
      decodeURIComponent(value.slice(separatorIndex + 1)),
    ];
  }
</script>

<div class="min-w-0">
  <h5 class="text-xs uppercase tracking-wider text-neutral-500">Version Policy</h5>
  {#if selectableVersions.length > 0}
    <div class="mt-2 space-y-2">
      <label class="block text-xs text-neutral-400" for={`${runtime.id}-selected-version`}>
        Selected version
      </label>
      <select
        id={`${runtime.id}-selected-version`}
        class="w-full rounded border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-sm text-neutral-200"
        value={versionOptionValue(
          runtime.selection.selected_version,
          runtime.selection.selected_runtime_variant_id
        )}
        disabled={selectionUpdating}
        onchange={(event) =>
          onUpdateSelected(
            ...parseVersionOptionValue((event.currentTarget as HTMLSelectElement).value)
          )}
      >
        <option value="">Automatic</option>
        {#each selectableVersions as version (version.display_label)}
          <option value={versionOptionValue(version.version, version.runtime_variant_id)}>
            {version.display_label}
          </option>
        {/each}
      </select>

      <label class="block text-xs text-neutral-400" for={`${runtime.id}-default-version`}>
        Default version
      </label>
      <select
        id={`${runtime.id}-default-version`}
        class="w-full rounded border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-sm text-neutral-200"
        value={versionOptionValue(
          runtime.selection.default_version,
          runtime.selection.default_runtime_variant_id
        )}
        disabled={selectionUpdating}
        onchange={(event) =>
          onUpdateDefault(
            ...parseVersionOptionValue((event.currentTarget as HTMLSelectElement).value)
          )}
      >
        <option value="">Unset</option>
        {#each selectableVersions as version (version.display_label)}
          <option value={versionOptionValue(version.version, version.runtime_variant_id)}>
            {version.display_label}
          </option>
        {/each}
      </select>
    </div>
  {:else}
    <p class="mt-2 text-xs text-neutral-500">
      Install a runtime version before Pantograph can pin selection or default policy.
    </p>
  {/if}

  <div class="mt-4 flex items-center justify-between gap-2">
    <h5 class="text-xs uppercase tracking-wider text-neutral-500">Available Versions</h5>
    <span class="text-[11px] text-neutral-600">{runtime.versions.length} known</span>
  </div>
  <p class="mt-1 text-[11px] text-neutral-600">Scroll the table to inspect versions, install roots, and actions without expanding the side panel.</p>
  <div class="mt-2 overflow-hidden rounded border border-neutral-800 bg-neutral-950/50">
    <div class="max-h-80 overflow-auto">
      <table class="min-w-[42rem] text-left text-xs text-neutral-300">
        <thead class="sticky top-0 bg-neutral-950 text-[11px] uppercase tracking-wider text-neutral-500">
          <tr>
            <th class="whitespace-nowrap px-3 py-2 font-medium">Version</th>
            <th class="whitespace-nowrap px-3 py-2 font-medium">Status</th>
            <th class="whitespace-nowrap px-3 py-2 font-medium">Target</th>
            <th class="whitespace-nowrap px-3 py-2 font-medium">Install Root</th>
            <th class="whitespace-nowrap px-3 py-2 text-right font-medium">Action</th>
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
              <td class="whitespace-nowrap px-3 py-2 text-neutral-400">
                <div>{version.runtime_key}</div>
                <div class="mt-1 text-[11px] text-neutral-500">{version.platform_key}</div>
              </td>
              <td class="px-3 py-2">
                {#if version.install_root}
                  <div
                    class="block max-w-[16rem] truncate font-mono text-[11px] text-neutral-500"
                    title={version.install_root}
                  >
                    {version.install_root}
                  </div>
                {:else}
                  <span class="text-neutral-600">Not installed</span>
                {/if}
              </td>
              <td class="whitespace-nowrap px-3 py-2 text-right">
                {#if version.version && version.install_state === 'installed' && !runtime.active_job}
                  <button
                    type="button"
                    class="rounded border border-neutral-700 px-2 py-1 text-[10px] text-neutral-300 transition-colors hover:bg-neutral-800 disabled:border-neutral-800 disabled:text-neutral-600"
                    onclick={() => onRemoveVersion(version.version ?? '')}
                    disabled={removingVersion !== null || installRequested || version.version === null}
                  >
                    {#if removingVersion === version.version}
                      Uninstalling...
                    {:else}
                      Uninstall
                    {/if}
                  </button>
                {:else if version.version && version.installable && version.install_state !== 'installed' && version.install_state !== 'system_provided' && !runtime.active_job}
                  <button
                    type="button"
                    class="rounded border border-blue-700 px-2 py-1 text-[10px] text-blue-200 transition-colors hover:bg-blue-950/40 disabled:border-neutral-800 disabled:text-neutral-600"
                    onclick={() => onInstallVersion(version.version)}
                    disabled={installRequested || removingVersion !== null}
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
