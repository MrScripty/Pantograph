<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw, Save } from 'lucide-svelte';
  import type {
    DiagnosticsRetentionPolicy,
    IoArtifactProjectionRecord,
    ProjectionStateRecord,
  } from '../../services/diagnostics/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import {
    formatIoArtifactAvailabilityLabel,
    formatIoArtifactBytes,
    formatIoArtifactMediaLabel,
    formatProjectionFreshness,
  } from './ioInspectorPresenters';

  let artifacts = $state<IoArtifactProjectionRecord[]>([]);
  let projectionState = $state<ProjectionStateRecord | null>(null);
  let retentionPolicy = $state<DiagnosticsRetentionPolicy | null>(null);
  let retentionDays = $state('365');
  let retentionExplanation = $state('');
  let loadingArtifacts = $state(false);
  let loadingRetention = $state(false);
  let savingRetention = $state(false);
  let artifactError = $state<string | null>(null);
  let retentionError = $state<string | null>(null);
  let artifactRequestSerial = 0;

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  function formatTimestamp(value: number): string {
    return new Date(value).toLocaleString();
  }

  function applyRetentionPolicy(policy: DiagnosticsRetentionPolicy): void {
    retentionPolicy = policy;
    retentionDays = String(policy.retention_days);
    retentionExplanation = policy.explanation;
  }

  async function refreshArtifacts(runId = activeRunId()): Promise<void> {
    const requestSerial = ++artifactRequestSerial;
    artifactError = null;

    if (!runId) {
      artifacts = [];
      projectionState = null;
      loadingArtifacts = false;
      return;
    }

    loadingArtifacts = true;
    try {
      const response = await workflowService.queryIoArtifacts({
        workflow_run_id: runId,
        limit: 250,
      });
      if (requestSerial !== artifactRequestSerial) {
        return;
      }
      artifacts = response.artifacts;
      projectionState = response.projection_state;
    } catch (error) {
      if (requestSerial !== artifactRequestSerial) {
        return;
      }
      artifactError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestSerial === artifactRequestSerial) {
        loadingArtifacts = false;
      }
    }
  }

  async function refreshRetentionPolicy(): Promise<void> {
    loadingRetention = true;
    retentionError = null;
    try {
      const response = await workflowService.queryRetentionPolicy();
      applyRetentionPolicy(response.retention_policy);
    } catch (error) {
      retentionError = error instanceof Error ? error.message : String(error);
    } finally {
      loadingRetention = false;
    }
  }

  async function saveRetentionPolicy(): Promise<void> {
    const parsedDays = Number.parseInt(retentionDays, 10);
    if (!Number.isFinite(parsedDays) || parsedDays < 1) {
      retentionError = 'Retention days must be at least 1';
      return;
    }

    const explanation = retentionExplanation.trim();
    if (explanation.length === 0) {
      retentionError = 'Retention explanation is required';
      return;
    }

    savingRetention = true;
    retentionError = null;
    try {
      const response = await workflowService.updateRetentionPolicy({
        retention_days: parsedDays,
        explanation,
        reason: 'gui_io_inspector_policy_update',
      });
      applyRetentionPolicy(response.retention_policy);
      await refreshArtifacts();
    } catch (error) {
      retentionError = error instanceof Error ? error.message : String(error);
    } finally {
      savingRetention = false;
    }
  }

  $effect(() => {
    const runId = activeRunId();
    void refreshArtifacts(runId);
  });

  onMount(() => {
    void refreshRetentionPolicy();
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between border-b border-neutral-800 px-4 py-3">
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-neutral-100">I/O Inspector</h1>
      <div class="mt-1 truncate text-xs text-neutral-500">
        {#if $activeWorkflowRun}
          {$activeWorkflowRun.workflow_run_id}
        {:else}
          No active run selected
        {/if}
      </div>
    </div>
    <button
      type="button"
      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
      onclick={() => refreshArtifacts()}
      disabled={loadingArtifacts || !$activeWorkflowRun}
    >
      <RefreshCw size={14} aria-hidden="true" class={loadingArtifacts ? 'animate-spin' : ''} />
      Refresh
    </button>
  </div>

  <div class="grid min-h-0 flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[1fr_22rem]">
    <div class="min-h-0 overflow-auto">
      <div class="border-b border-neutral-900 px-4 py-3 text-xs text-neutral-500">
        {formatProjectionFreshness(projectionState)}
      </div>

      {#if artifactError}
        <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{artifactError}</div>
      {/if}

      {#if !$activeWorkflowRun}
        <div class="px-4 py-8 text-sm text-neutral-500">No active run selected</div>
      {:else if loadingArtifacts && artifacts.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">Loading artifacts</div>
      {:else if artifacts.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">No retained artifact metadata for this run</div>
      {:else}
        <div class="grid gap-3 p-4 xl:grid-cols-2 2xl:grid-cols-3">
          {#each artifacts as artifact (artifact.event_id)}
            <article class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="truncate font-mono text-xs text-neutral-100" title={artifact.artifact_id}>
                    {artifact.artifact_id}
                  </div>
                  <div class="mt-1 text-xs text-neutral-500">
                    {artifact.artifact_role} · {formatIoArtifactMediaLabel(artifact.media_type)}
                  </div>
                </div>
                <span class="shrink-0 rounded border border-neutral-700 px-2 py-0.5 text-xs text-neutral-300">
                  {formatIoArtifactAvailabilityLabel(artifact)}
                </span>
              </div>

              <dl class="mt-4 grid grid-cols-2 gap-x-3 gap-y-2 text-xs">
                <div>
                  <dt class="text-neutral-500">Size</dt>
                  <dd class="mt-0.5 text-neutral-200">{formatIoArtifactBytes(artifact.size_bytes)}</dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Node</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.node_id ?? ''}>
                    {artifact.node_id ?? 'Workflow'}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Policy</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.retention_policy_id ?? ''}>
                    {artifact.retention_policy_id ?? 'Unassigned'}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Observed</dt>
                  <dd class="mt-0.5 text-neutral-200">{formatTimestamp(artifact.occurred_at_ms)}</dd>
                </div>
              </dl>

              {#if artifact.content_hash}
                <div class="mt-3 truncate font-mono text-[11px] text-neutral-500" title={artifact.content_hash}>
                  {artifact.content_hash}
                </div>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    </div>

    <aside class="min-h-0 overflow-auto border-l border-neutral-800 bg-neutral-950/80">
      <form
        class="space-y-4 p-4"
        onsubmit={(event) => {
          event.preventDefault();
          void saveRetentionPolicy();
        }}
      >
        <div>
          <h2 class="text-sm font-semibold text-neutral-100">Retention Policy</h2>
          <div class="mt-1 text-xs text-neutral-500">
            {#if retentionPolicy}
              {retentionPolicy.policy_id} · applied {formatTimestamp(retentionPolicy.applied_at_ms)}
            {:else if loadingRetention}
              Loading policy
            {:else}
              Policy unavailable
            {/if}
          </div>
        </div>

        {#if retentionError}
          <div class="rounded border border-red-900 bg-red-950/50 px-3 py-2 text-sm text-red-200">{retentionError}</div>
        {/if}

        <div>
          <label for="io-retention-days" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
            Days
          </label>
          <input
            id="io-retention-days"
            type="number"
            min="1"
            bind:value={retentionDays}
            class="mt-2 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
          />
        </div>

        <div>
          <label for="io-retention-explanation" class="block text-xs uppercase tracking-[0.18em] text-neutral-500">
            Explanation
          </label>
          <textarea
            id="io-retention-explanation"
            rows="5"
            bind:value={retentionExplanation}
            class="mt-2 w-full resize-none rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 focus:border-cyan-500 focus:outline-none"
          ></textarea>
        </div>

        <button
          type="submit"
          class="inline-flex w-full items-center justify-center gap-2 rounded border border-cyan-800 bg-cyan-950 px-3 py-2 text-sm text-cyan-100 transition-colors hover:border-cyan-600 hover:bg-cyan-900 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
          disabled={savingRetention || loadingRetention}
        >
          <Save size={14} aria-hidden="true" />
          {savingRetention ? 'Saving' : 'Save Policy'}
        </button>
      </form>
    </aside>
  </div>
</section>
