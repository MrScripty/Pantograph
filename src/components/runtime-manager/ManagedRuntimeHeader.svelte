<script lang="ts">
  import type { ManagedRuntimeManagerRuntimeView } from '../../services/managedRuntime';
  import { StatusGlyph } from '../workbench/ui';
  import { managedRuntimeReadinessGlyph } from './managedRuntimePresenters';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
    installedVersionCount: number;
    error: string | null;
  };

  let { runtime, installedVersionCount, error }: Props = $props();
</script>

<div class="flex flex-wrap items-start justify-between gap-2">
  <div class="min-w-0 flex-1">
    <h4 class="m-0 text-[13px] font-bold text-pg-text-bright">{runtime.display_name}</h4>
    <p class="m-0 mt-1 break-words pg-small text-pg-text-soft">
      <StatusGlyph state={managedRuntimeReadinessGlyph(runtime.readiness_state)} small class="mr-1.5 align-middle" />
      Readiness {runtime.readiness_state.replaceAll('_', ' ')}{#if runtime.selection.active_version}<span class="pg-muted">, active</span> <span class="pg-mono">{runtime.selection.active_version}</span>{/if}{#if runtime.selection.selected_version}<span class="pg-muted">, selected</span> <span class="pg-mono">{runtime.selection.selected_version}</span>{/if}
    </p>
  </div>
  <div class="min-w-0 text-left pg-muted pg-small sm:text-right">
    <div>{runtime.available ? 'Ready for use' : 'Not ready'}</div>
    <div>{installedVersionCount} installed version{installedVersionCount === 1 ? '' : 's'}</div>
  </div>
</div>

{#if runtime.unavailable_reason}
  <p class="m-0 mt-3 pg-muted pg-small">
    <StatusGlyph state="warn" small class="mr-1.5 align-middle" />
    {runtime.unavailable_reason}
  </p>
{/if}

{#if error}
  <p class="m-0 mt-3 pg-muted pg-small" role="alert">{error}</p>
{/if}
