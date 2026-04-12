<script lang="ts">
  import type { DiagnosticsSchedulerSnapshot } from '../../services/diagnostics/types';
  import {
    formatDiagnosticsTimestamp,
    getSchedulerStateClasses,
  } from './presenters';

  export let scheduler: DiagnosticsSchedulerSnapshot;
</script>

<div class="h-full overflow-auto px-4 py-4">
  {#if !scheduler.sessionId}
    <div class="flex h-full items-center justify-center rounded-2xl border border-dashed border-neutral-800 bg-neutral-950/70 px-6 text-center text-sm text-neutral-500">
      Open or run a workflow session to inspect queue state, keep-alive behavior, and scheduler load.
    </div>
  {:else}
    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Session</div>
        <div class="mt-3 truncate text-sm font-medium text-neutral-100">{scheduler.sessionId}</div>
        <div class="mt-2 text-xs text-neutral-500">{scheduler.workflowId ?? 'Unknown workflow'}</div>
        <div class="mt-3 text-xs text-neutral-500">
          Refreshed {formatDiagnosticsTimestamp(scheduler.capturedAtMs)}
        </div>
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">State</div>
        <div class="mt-3">
          {#if scheduler.session}
            <span class={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${getSchedulerStateClasses(scheduler.session.state)}`}>
              {scheduler.session.state}
            </span>
          {:else}
            <span class="text-sm text-neutral-500">Unavailable</span>
          {/if}
        </div>
        <div class="mt-3 text-xs text-neutral-500">
          Keep alive {scheduler.session?.keep_alive ? 'enabled' : 'disabled'}
        </div>
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Queue Depth</div>
        <div class="mt-3 text-2xl font-semibold text-neutral-100">
          {scheduler.session?.queued_runs ?? scheduler.items.length}
        </div>
        <div class="mt-2 text-xs text-neutral-500">
          {scheduler.items.filter((item) => item.status === 'running').length} running • {scheduler.items.filter((item) => item.status === 'pending').length} pending
        </div>
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Completed Runs</div>
        <div class="mt-3 text-2xl font-semibold text-neutral-100">
          {scheduler.session?.run_count ?? 0}
        </div>
        <div class="mt-2 text-xs text-neutral-500">
          Queue refreshed on session changes and execution lifecycle events.
        </div>
      </article>
    </div>

    {#if scheduler.lastError}
      <div class="mt-4 rounded-xl border border-red-900/80 bg-red-950/40 px-4 py-3 text-sm text-red-200">
        {scheduler.lastError}
      </div>
    {/if}

    <section class="mt-4 rounded-xl border border-neutral-800 bg-neutral-950/80">
      <header class="border-b border-neutral-800 px-4 py-3">
        <div class="text-sm font-medium text-neutral-100">Queue Items</div>
        <div class="text-xs text-neutral-500">
          Current scheduler ordering for the active session.
        </div>
      </header>

      {#if scheduler.items.length === 0}
        <div class="px-4 py-6 text-sm text-neutral-500">
          No queued or running items are currently tracked for this session.
        </div>
      {:else}
        <div class="overflow-auto">
          <table class="min-w-full divide-y divide-neutral-800 text-sm">
            <thead class="bg-neutral-950/90 text-left text-xs uppercase tracking-[0.24em] text-neutral-500">
              <tr>
                <th class="px-4 py-3 font-medium">Queue Id</th>
                <th class="px-4 py-3 font-medium">Run Id</th>
                <th class="px-4 py-3 font-medium">Priority</th>
                <th class="px-4 py-3 font-medium">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-neutral-900">
              {#each scheduler.items as item (item.queue_id)}
                <tr>
                  <td class="px-4 py-3 text-neutral-200">{item.queue_id}</td>
                  <td class="px-4 py-3 text-neutral-400">{item.run_id ?? 'auto-generated'}</td>
                  <td class="px-4 py-3 text-neutral-300">{item.priority}</td>
                  <td class="px-4 py-3">
                    <span class={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${getSchedulerStateClasses(item.status)}`}>
                      {item.status}
                    </span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  {/if}
</div>
