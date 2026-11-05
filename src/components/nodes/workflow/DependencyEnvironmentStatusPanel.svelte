<script lang="ts">
  import { dependencyCodeLabel, type ModelDependencyStatus } from './dependencyEnvironmentState';

  interface DependencyBadge {
    label: string;
    className: string;
  }

  interface Props {
    hasModelBinding: boolean;
    dependencyBadge: DependencyBadge;
    dependencyStatus: ModelDependencyStatus | null;
    isBusy: boolean;
    onResolve: () => void;
    onCheck: () => void;
    onInstall: () => void;
  }

  let {
    hasModelBinding,
    dependencyBadge,
    dependencyStatus,
    isBusy,
    onResolve,
    onCheck,
    onInstall,
  }: Props = $props();
</script>

{#if !hasModelBinding}
  <div class="text-[10px] text-pg-text-soft">
    Connect this sidecar to an inference node.
  </div>
{:else}
  <!-- State is in the badge label; the box stays neutral (token palette, no state hues). -->
  <div class="rounded border border-pg-border px-2 py-1 text-[10px] text-pg-text-soft">
    <div class="flex items-center gap-2">
      <span>{dependencyBadge.label}</span>
      <button
        type="button"
        class="ml-auto text-pg-text-muted hover:text-pg-text disabled:opacity-50"
        onclick={onResolve}
        disabled={isBusy}
      >
        Resolve
      </button>
      <button
        type="button"
        class="text-pg-text-muted hover:text-pg-text disabled:opacity-50"
        onclick={onCheck}
        disabled={isBusy}
      >
        Check
      </button>
      <button
        type="button"
        class="text-pg-text-muted hover:text-pg-text disabled:opacity-50"
        onclick={onInstall}
        disabled={isBusy}
      >
        Install
      </button>
    </div>
    {#if dependencyStatus?.message}
      <div class="mt-1 text-[9px] text-pg-text-muted truncate" title={dependencyStatus.message}>
        {dependencyStatus.message}
      </div>
    {/if}
    {#if dependencyStatus?.code}
      <div class="mt-1 text-[9px] text-pg-text-soft truncate" title={dependencyStatus.code}>
        code: {dependencyCodeLabel(dependencyStatus.code) ?? dependencyStatus.code}
      </div>
    {/if}
  </div>
{/if}
