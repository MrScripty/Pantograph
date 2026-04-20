<script lang="ts">
  import type { ManagedRuntimeSelectionState } from '../../services/managedRuntime';

  type Props = {
    available: boolean;
    installState: string;
    selection: ManagedRuntimeSelectionState;
    installedVersionCount: number;
    downloadSizeLabel: string;
  };

  let {
    available,
    installState,
    selection,
    installedVersionCount,
    downloadSizeLabel,
  }: Props = $props();
</script>

<div class="mt-3 border-t border-neutral-800 pt-3">
  <dl class="grid gap-x-4 gap-y-2 text-xs sm:grid-cols-2 xl:grid-cols-4">
    <div class="min-w-0">
      <dt class="text-neutral-500">Selected</dt>
      <dd class="mt-1 truncate text-sm text-neutral-200" title={selection.selected_version ?? 'Automatic'}>
        {selection.selected_version ?? 'Automatic'}
      </dd>
    </div>
    <div class="min-w-0">
      <dt class="text-neutral-500">Default</dt>
      <dd class="mt-1 truncate text-sm text-neutral-200" title={selection.default_version ?? 'Unset'}>
        {selection.default_version ?? 'Unset'}
      </dd>
    </div>
    <div class="min-w-0">
      <dt class="text-neutral-500">Install State</dt>
      <dd class="mt-1 text-sm capitalize text-neutral-200">{installState.replaceAll('_', ' ')}</dd>
    </div>
    <div class="min-w-0">
      <dt class="text-neutral-500">Footprint</dt>
      <dd class="mt-1 text-sm text-neutral-200">
        {downloadSizeLabel}
        <span class="text-neutral-500">
          {' '}
          • {available ? 'ready on this host' : `${installedVersionCount} installed`}
        </span>
      </dd>
    </div>
  </dl>
</div>
