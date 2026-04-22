<script lang="ts">
  import { dependencyCodeLabel, type ModelDependencyStatus } from './dependencyEnvironmentState';

  interface DependencyBadge {
    label: string;
    className: string;
  }

  interface Props {
    hasModelPath: boolean;
    dependencyBadge: DependencyBadge;
    dependencyStatus: ModelDependencyStatus | null;
    isBusy: boolean;
    onRun: () => void;
    onResolve: () => void;
    onCheck: () => void;
    onInstall: () => void;
  }

  let {
    hasModelPath,
    dependencyBadge,
    dependencyStatus,
    isBusy,
    onRun,
    onResolve,
    onCheck,
    onInstall,
  }: Props = $props();
</script>

{#if !hasModelPath}
  <div class="text-[10px] text-amber-400">
    Connect Puma-Lib `model_path` and `dependency_requirements`.
  </div>
{:else}
  <div class="rounded border px-2 py-1 text-[10px] {dependencyBadge.className}">
    <div class="flex items-center gap-2">
      <span>{dependencyBadge.label}</span>
      <button
        type="button"
        class="ml-auto text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
        onclick={onRun}
        disabled={isBusy}
      >
        Run
      </button>
      <button
        type="button"
        class="text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
        onclick={onResolve}
        disabled={isBusy}
      >
        Resolve
      </button>
      <button
        type="button"
        class="text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
        onclick={onCheck}
        disabled={isBusy}
      >
        Check
      </button>
      <button
        type="button"
        class="text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
        onclick={onInstall}
        disabled={isBusy}
      >
        Install
      </button>
    </div>
    {#if dependencyStatus?.message}
      <div class="mt-1 text-[9px] text-neutral-500 truncate" title={dependencyStatus.message}>
        {dependencyStatus.message}
      </div>
    {/if}
    {#if dependencyStatus?.code}
      <div class="mt-1 text-[9px] text-amber-300 truncate" title={dependencyStatus.code}>
        code: {dependencyCodeLabel(dependencyStatus.code) ?? dependencyStatus.code}
      </div>
    {/if}
  </div>
{/if}
