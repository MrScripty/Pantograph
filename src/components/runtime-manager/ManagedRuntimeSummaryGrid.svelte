<script lang="ts">
  import type { ManagedRuntimeSelectionState } from '../../services/managedRuntime';
  import { sentenceCaseState } from './managedRuntimePresenters';

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

<dl class="m-0 mt-3 grid gap-x-4 gap-y-2 border-t border-pg-border-soft pt-3 pg-small sm:grid-cols-2 xl:grid-cols-4">
  <div class="min-w-0">
    <dt class="pg-muted">Selected</dt>
    <dd class="m-0 mt-0.5 truncate pg-mono" title={selection.selected_version ?? 'Automatic'}>
      {selection.selected_version ?? 'Automatic'}
    </dd>
  </div>
  <div class="min-w-0">
    <dt class="pg-muted">Default</dt>
    <dd class="m-0 mt-0.5 truncate pg-mono" title={selection.default_version ?? 'Unset'}>
      {selection.default_version ?? 'Unset'}
    </dd>
  </div>
  <div class="min-w-0">
    <dt class="pg-muted">Install state</dt>
    <dd class="m-0 mt-0.5">{sentenceCaseState(installState)}</dd>
  </div>
  <div class="min-w-0">
    <dt class="pg-muted">Footprint</dt>
    <dd class="m-0 mt-0.5">
      <span class="pg-mono">{downloadSizeLabel}</span><span class="pg-muted">, {available ? 'ready on this host' : `${installedVersionCount} installed`}</span>
    </dd>
  </div>
</dl>
