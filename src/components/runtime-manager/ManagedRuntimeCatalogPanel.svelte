<script lang="ts">
  import type {
    ManagedRuntimeManagerRuntimeView,
    ManagedRuntimeVersionStatus,
  } from '../../services/managedRuntime';
  import { Chip } from '../workbench/ui';
  import { sentenceCaseState } from './managedRuntimePresenters';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
    selectableVersions: ManagedRuntimeVersionStatus[];
    selectionUpdating: boolean;
    installRequested: boolean;
    installingVersion: string | null;
    removingVersion: string | null;
    onUpdateSelected: (version: string | null, runtimeVariantId: string | null) => Promise<void>;
    onUpdateDefault: (version: string | null, runtimeVariantId: string | null) => Promise<void>;
    onInstallVersion: (version: string | null, runtimeVariantId: string | null) => Promise<void>;
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
  <h5 class="m-0 text-[12.5px] font-bold text-pg-text-soft">Version policy</h5>
  {#if selectableVersions.length > 0}
    <div class="mt-2 grid gap-3 sm:grid-cols-2">
      <div class="flex min-w-0 flex-col gap-1">
        <label class="pg-fl" for={`${runtime.id}-selected-version`}>Selected version</label>
        <select
          id={`${runtime.id}-selected-version`}
          class="pg-field w-full"
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
      </div>

      <div class="flex min-w-0 flex-col gap-1">
        <label class="pg-fl" for={`${runtime.id}-default-version`}>Default version</label>
        <select
          id={`${runtime.id}-default-version`}
          class="pg-field w-full"
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
    </div>
  {:else}
    <p class="m-0 mt-2 pg-muted pg-small">
      Install a runtime version before Pantograph can pin selection or default policy.
    </p>
  {/if}

  <div class="mt-4 flex items-baseline justify-between gap-2">
    <h5 class="m-0 text-[12.5px] font-bold text-pg-text-soft">Available versions</h5>
    <span class="pg-muted pg-small">{runtime.versions.length} known</span>
  </div>
  <p class="m-0 mt-1 pg-muted pg-small">Scroll the table to inspect versions, install roots, and actions without expanding the side panel.</p>
  <div class="mt-2 max-h-80 overflow-auto rounded border border-pg-border bg-pg-panel-sunken">
    <table class="pg-tbl min-w-[42rem]">
      <thead>
        <tr>
          <th scope="col">Version</th>
          <th scope="col">Status</th>
          <th scope="col">Target</th>
          <th scope="col">Install root</th>
          <th scope="col" class="text-right">Action</th>
        </tr>
      </thead>
      <tbody>
        {#each runtime.versions as version (version.display_label)}
          <tr>
            <td>
              <div class="pg-mono text-pg-text-bright">{version.display_label}</div>
              <div class="mt-1 flex flex-wrap gap-1.5">
                <Chip>{sentenceCaseState(versionBadgeLabel(version))}</Chip>
                {#if version.selected}
                  <Chip>Selected</Chip>
                {/if}
                {#if version.active}
                  <Chip glyph="succeeded">Active</Chip>
                {/if}
              </div>
            </td>
            <td>
              <div>{sentenceCaseState(version.install_state)}</div>
              <div class="mt-1 pg-muted pg-small">{sentenceCaseState(version.readiness_state)}</div>
            </td>
            <td class="whitespace-nowrap">
              <div class="pg-mono">{version.runtime_key}</div>
              <div class="mt-1 pg-mono pg-muted">{version.platform_key}</div>
            </td>
            <td>
              {#if version.install_root}
                <div class="block max-w-[16rem] truncate pg-mono pg-muted" title={version.install_root}>
                  {version.install_root}
                </div>
              {:else}
                <span class="pg-muted">Not installed</span>
              {/if}
            </td>
            <td class="whitespace-nowrap text-right">
              {#if version.version && version.install_state === 'installed' && !runtime.active_job}
                <button
                  type="button"
                  class="pg-btn pg-btn-sm pg-btn-danger"
                  onclick={() => onRemoveVersion(version.version ?? '')}
                  disabled={removingVersion !== null || installRequested || version.version === null}
                >
                  {#if removingVersion === version.version}
                    Uninstalling...
                  {:else}
                    Uninstall
                  {/if}
                  <span class="pg-sr-only">{version.display_label}</span>
                </button>
              {:else if version.version && version.installable && version.install_state !== 'installed' && version.install_state !== 'system_provided' && !runtime.active_job}
                <button
                  type="button"
                  class="pg-btn pg-btn-sm"
                  onclick={() => onInstallVersion(version.version, version.runtime_variant_id)}
                  disabled={installRequested || removingVersion !== null}
                >
                  {#if installRequested && installingVersion === version.version}
                    Installing...
                  {:else}
                    Install
                  {/if}
                  <span class="pg-sr-only">{version.display_label}</span>
                </button>
              {:else}
                <span class="pg-muted pg-small">No action</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
