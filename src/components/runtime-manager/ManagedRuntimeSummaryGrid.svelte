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

<div class="mt-3 grid gap-2 text-xs text-neutral-400 md:grid-cols-4">
  <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
    <div class="text-neutral-500">Selected</div>
    <div class="mt-1 text-neutral-200">{selection.selected_version ?? 'Automatic'}</div>
  </div>
  <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
    <div class="text-neutral-500">Default</div>
    <div class="mt-1 text-neutral-200">{selection.default_version ?? 'Unset'}</div>
  </div>
  <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
    <div class="text-neutral-500">Install State</div>
    <div class="mt-1 text-neutral-200">{installState}</div>
  </div>
  <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
    <div class="text-neutral-500">Download Size</div>
    <div class="mt-1 text-neutral-200">
      {downloadSizeLabel} • {available ? 'ready' : `${installedVersionCount} installed`}
    </div>
  </div>
</div>
