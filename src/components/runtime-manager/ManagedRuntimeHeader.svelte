<script lang="ts">
  import type { ManagedRuntimeManagerRuntimeView } from '../../services/managedRuntime';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
    installedVersionCount: number;
    error: string | null;
  };

  let { runtime, installedVersionCount, error }: Props = $props();
</script>

<div class="flex flex-wrap items-start justify-between gap-2">
  <div class="min-w-0 flex-1">
    <h4 class="text-sm font-medium text-neutral-100">{runtime.display_name}</h4>
    <p class="mt-1 break-words text-xs text-neutral-500">
      Readiness {runtime.readiness_state.replaceAll('_', ' ')}
      {#if runtime.selection.active_version}
        • Active {runtime.selection.active_version}
      {/if}
      {#if runtime.selection.selected_version}
        • Selected {runtime.selection.selected_version}
      {/if}
    </p>
  </div>
  <div class="min-w-0 text-left text-[11px] text-neutral-500 sm:text-right">
    <div>{runtime.available ? 'Ready for use' : 'Not ready'}</div>
    <div>{installedVersionCount} installed version{installedVersionCount === 1 ? '' : 's'}</div>
  </div>
</div>

{#if runtime.unavailable_reason}
  <div class="mt-3 rounded border border-amber-800/50 bg-amber-950/20 p-2 text-xs text-amber-300">
    {runtime.unavailable_reason}
  </div>
{/if}

{#if error}
  <div class="mt-3 rounded border border-red-800/50 bg-red-950/20 p-2 text-xs text-red-300">
    {error}
  </div>
{/if}
