<script lang="ts">
  import type { RuntimeLifecycleSnapshot } from '../../services/ConfigService';
  import { KeyValue, type KeyValueRow } from '../workbench/ui';

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

  function reusedLabel(snapshot: RuntimeLifecycleSnapshot | null): string {
    if (snapshot?.runtime_reused === null) {
      return 'unknown';
    }
    return snapshot?.runtime_reused ? 'yes' : 'no';
  }

  function snapshotRows(
    snapshot: RuntimeLifecycleSnapshot | null,
    runtimeId: string,
    modelTarget: string | null
  ): KeyValueRow[] {
    const rows: KeyValueRow[] = [{ key: 'Runtime', value: runtimeId, mono: true }];
    if (modelTarget) {
      rows.push({ key: 'Target', value: modelTarget, mono: true });
    }
    rows.push(
      { key: 'Instance', value: snapshot?.runtime_instance_id ?? 'unreported', mono: true },
      { key: 'Warmup', value: formatRuntimeDuration(snapshot?.warmup_duration_ms ?? null), mono: true },
      { key: 'Reused', value: reusedLabel(snapshot) }
    );
    return rows;
  }

  let cards = $derived([
    {
      title: 'Active runtime',
      snapshot: activeRuntime,
      rows: snapshotRows(
        activeRuntime,
        activeRuntime?.runtime_id ?? fallbackActiveRuntimeId ?? 'unknown',
        activeModelTarget
      ),
    },
    {
      title: 'Embedding runtime',
      snapshot: embeddingRuntime,
      rows: snapshotRows(
        embeddingRuntime,
        embeddingRuntime?.runtime_id ?? 'not active',
        embeddingModelTarget
      ),
    },
  ]);
</script>

<div class="grid grid-cols-[repeat(auto-fit,minmax(min(16rem,100%),1fr))] gap-2">
  {#each cards as card (card.title)}
    <div class="rounded border border-pg-border bg-pg-panel-sunken px-3 py-2 pg-small">
      <div class="flex items-baseline justify-between gap-2">
        <h4 class="m-0 text-[13px] font-bold text-pg-text-bright">{card.title}</h4>
        <span class="pg-muted">{runtimeStateLabel(card.snapshot)}</span>
      </div>
      <div class="mt-2 [&_.pg-kv]:text-[12.5px]">
        <KeyValue rows={card.rows} label={card.title} />
      </div>
      {#if card.snapshot?.lifecycle_decision_reason}
        <p class="m-0 mt-2 pg-muted">{card.snapshot.lifecycle_decision_reason}</p>
      {/if}
    </div>
  {/each}
</div>
