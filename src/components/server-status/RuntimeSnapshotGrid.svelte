<script lang="ts">
  import type { RuntimeLifecycleSnapshot } from '../../services/ConfigService';

  type Props = {
    activeRuntime: RuntimeLifecycleSnapshot | null;
    activeModelTarget: string | null;
    embeddingRuntime: RuntimeLifecycleSnapshot | null;
    embeddingModelTarget: string | null;
    fallbackActiveRuntimeId: string | null;
  };

  let {
    activeRuntime,
    activeModelTarget,
    embeddingRuntime,
    embeddingModelTarget,
    fallbackActiveRuntimeId,
  }: Props = $props();

  function formatRuntimeDuration(durationMs: number | null): string {
    if (durationMs === null) {
      return 'n/a';
    }

    if (durationMs < 1000) {
      return `${durationMs}ms`;
    }

    return `${(durationMs / 1000).toFixed(1)}s`;
  }

  function runtimeStateLabel(snapshot: RuntimeLifecycleSnapshot | null): string {
    if (!snapshot) {
      return 'Unavailable';
    }

    if (snapshot.last_error) {
      return 'Error';
    }

    return snapshot.active ? 'Active' : 'Idle';
  }
</script>

<div class="grid gap-2 text-[10px] text-neutral-400 md:grid-cols-2">
  <div class="rounded border border-neutral-700 bg-neutral-900/60 p-2">
    <div class="flex items-center justify-between gap-2 text-neutral-500">
      <span>Active Runtime</span>
      <span>{runtimeStateLabel(activeRuntime)}</span>
    </div>
    <div class="mt-1 font-mono text-neutral-300">
      {activeRuntime?.runtime_id ?? fallbackActiveRuntimeId ?? 'unknown'}
    </div>
    {#if activeModelTarget}
      <div class="mt-1 break-all text-neutral-500">Target {activeModelTarget}</div>
    {/if}
    <div class="mt-1 text-neutral-500">
      Instance {activeRuntime?.runtime_instance_id ?? 'unreported'}
    </div>
    <div class="mt-1 text-neutral-500">
      Warmup {formatRuntimeDuration(activeRuntime?.warmup_duration_ms ?? null)}
      • Reused
      {activeRuntime?.runtime_reused === null
        ? ' unknown'
        : activeRuntime?.runtime_reused
          ? ' yes'
          : ' no'}
    </div>
    {#if activeRuntime?.lifecycle_decision_reason}
      <div class="mt-1 text-neutral-600">{activeRuntime.lifecycle_decision_reason}</div>
    {/if}
  </div>

  <div class="rounded border border-neutral-700 bg-neutral-900/60 p-2">
    <div class="flex items-center justify-between gap-2 text-neutral-500">
      <span>Embedding Runtime</span>
      <span>{runtimeStateLabel(embeddingRuntime)}</span>
    </div>
    <div class="mt-1 font-mono text-neutral-300">
      {embeddingRuntime?.runtime_id ?? 'not active'}
    </div>
    {#if embeddingModelTarget}
      <div class="mt-1 break-all text-neutral-500">Target {embeddingModelTarget}</div>
    {/if}
    <div class="mt-1 text-neutral-500">
      Instance {embeddingRuntime?.runtime_instance_id ?? 'unreported'}
    </div>
    <div class="mt-1 text-neutral-500">
      Warmup {formatRuntimeDuration(embeddingRuntime?.warmup_duration_ms ?? null)}
      • Reused
      {embeddingRuntime?.runtime_reused === null
        ? ' unknown'
        : embeddingRuntime?.runtime_reused
          ? ' yes'
          : ' no'}
    </div>
    {#if embeddingRuntime?.lifecycle_decision_reason}
      <div class="mt-1 text-neutral-600">{embeddingRuntime.lifecycle_decision_reason}</div>
    {/if}
  </div>
</div>
