<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Braces,
    CircleHelp,
    File,
    FileText,
    Image as ImageIcon,
    Music,
    RefreshCw,
    Save,
    Table2,
    Trash2,
    Video,
  } from 'lucide-svelte';
  import type {
    DiagnosticsRetentionPolicy,
    IoArtifactProjectionRecord,
    IoArtifactRetentionSummaryRecord,
    ProjectionStateRecord,
    WorkflowRetentionCleanupResult,
    WorkflowIoArtifactQueryRequest,
  } from '../../services/diagnostics/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import {
    buildIoArtifactNodeGroups,
    buildIoArtifactRendererSummary,
    buildRetentionCleanupDetailRows,
    buildRetentionPolicyDetailRows,
    buildRetentionPolicySettingRows,
    formatIoArtifactAvailabilityLabel,
    formatIoArtifactBytes,
    formatIoArtifactDetailValue,
    formatIoArtifactEndpointValue,
    formatIoArtifactMediaLabel,
    formatIoArtifactRetentionStateLabel,
    formatIoArtifactRoleLabel,
    formatProjectionFreshness,
    isWorkflowInputArtifact,
    isWorkflowOutputArtifact,
  } from './ioInspectorPresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  let artifacts = $state<IoArtifactProjectionRecord[]>([]);
  let retentionSummary = $state<IoArtifactRetentionSummaryRecord[]>([]);
  let projectionState = $state<ProjectionStateRecord | null>(null);
  let retentionPolicy = $state<DiagnosticsRetentionPolicy | null>(null);
  let retentionCleanup = $state<WorkflowRetentionCleanupResult | null>(null);
  let retentionDays = $state('365');
  let retentionExplanation = $state('');
  let loadingArtifacts = $state(false);
  let loadingRetention = $state(false);
  let savingRetention = $state(false);
  let applyingRetentionCleanup = $state(false);
  let artifactError = $state<string | null>(null);
  let retentionError = $state<string | null>(null);
  let retentionCleanupMessage = $state<string | null>(null);
  let endpointFilterMode = $state<'all' | 'producer' | 'consumer'>('all');
  let endpointNodeFilter = $state('');
  let artifactRequestSerial = 0;
  let workflowInputArtifacts = $derived(artifacts.filter(isWorkflowInputArtifact));
  let workflowOutputArtifacts = $derived(artifacts.filter(isWorkflowOutputArtifact));
  let nodeGroups = $derived(buildIoArtifactNodeGroups(artifacts));
  let retentionPolicyRows = $derived(buildRetentionPolicyDetailRows(retentionPolicy));
  let retentionPolicySettingRows = $derived(buildRetentionPolicySettingRows(retentionPolicy));
  let retentionCleanupRows = $derived(buildRetentionCleanupDetailRows(retentionCleanup));
  let summarizedArtifactCount = $derived(
    retentionSummary.reduce((total, item) => total + item.artifact_count, 0),
  );

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

  async function refreshArtifacts(
    runId = activeRunId(),
    filterMode = endpointFilterMode,
    filterNodeValue = endpointNodeFilter.trim(),
  ): Promise<void> {
    const requestSerial = ++artifactRequestSerial;
    artifactError = null;

    loadingArtifacts = true;
    try {
      const request: WorkflowIoArtifactQueryRequest = {
        workflow_run_id: runId ?? null,
        limit: 250,
      };
      const filterNodeId = filterNodeValue.trim();
      if (filterNodeId.length > 0 && filterMode === 'producer') {
        request.producer_node_id = filterNodeId;
      }
      if (filterNodeId.length > 0 && filterMode === 'consumer') {
        request.consumer_node_id = filterNodeId;
      }

      const response = await workflowService.queryIoArtifacts(request);
      if (requestSerial !== artifactRequestSerial) {
        return;
      }
      artifacts = response.artifacts;
      retentionSummary = response.retention_summary;
      projectionState = response.projection_state;
    } catch (error) {
      if (requestSerial !== artifactRequestSerial) {
        return;
      }
      artifactError = formatWorkflowCommandError(error);
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
      retentionError = formatWorkflowCommandError(error);
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
      retentionCleanup = null;
      retentionCleanupMessage = null;
      applyRetentionPolicy(response.retention_policy);
      await refreshArtifacts();
    } catch (error) {
      retentionError = formatWorkflowCommandError(error);
    } finally {
      savingRetention = false;
    }
  }

  async function applyRetentionCleanup(): Promise<void> {
    applyingRetentionCleanup = true;
    retentionError = null;
    retentionCleanupMessage = null;
    try {
      const response = await workflowService.applyRetentionCleanup({
        limit: 250,
        reason: 'gui_io_inspector_cleanup_apply',
      });
      retentionCleanup = response.cleanup;
      retentionCleanupMessage = `${response.cleanup.expired_artifact_count} artifacts expired`;
      await refreshArtifacts();
    } catch (error) {
      retentionError = formatWorkflowCommandError(error);
    } finally {
      applyingRetentionCleanup = false;
    }
  }

  $effect(() => {
    const runId = activeRunId();
    const filterMode = endpointFilterMode;
    const filterNodeValue = endpointNodeFilter.trim();
    void refreshArtifacts(runId, filterMode, filterNodeValue);
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
          Browsing retained artifacts across runs
        {/if}
      </div>
    </div>
    <button
      type="button"
      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
      onclick={() => refreshArtifacts()}
      disabled={loadingArtifacts}
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
      <div class="flex flex-wrap items-end gap-3 border-b border-neutral-900 px-4 py-3">
        <div>
          <div class="mb-2 text-xs uppercase tracking-[0.18em] text-neutral-500">Endpoint</div>
          <div class="inline-flex overflow-hidden rounded border border-neutral-800">
            <button
              type="button"
              aria-pressed={endpointFilterMode === 'all'}
              class="px-3 py-1.5 text-xs text-neutral-300 transition-colors hover:bg-neutral-900 hover:text-neutral-100"
              class:bg-cyan-950={endpointFilterMode === 'all'}
              class:text-cyan-100={endpointFilterMode === 'all'}
              onclick={() => {
                endpointFilterMode = 'all';
              }}
            >
              All
            </button>
            <button
              type="button"
              aria-pressed={endpointFilterMode === 'producer'}
              class="border-l border-neutral-800 px-3 py-1.5 text-xs text-neutral-300 transition-colors hover:bg-neutral-900 hover:text-neutral-100"
              class:bg-cyan-950={endpointFilterMode === 'producer'}
              class:text-cyan-100={endpointFilterMode === 'producer'}
              onclick={() => {
                endpointFilterMode = 'producer';
              }}
            >
              Produced
            </button>
            <button
              type="button"
              aria-pressed={endpointFilterMode === 'consumer'}
              class="border-l border-neutral-800 px-3 py-1.5 text-xs text-neutral-300 transition-colors hover:bg-neutral-900 hover:text-neutral-100"
              class:bg-cyan-950={endpointFilterMode === 'consumer'}
              class:text-cyan-100={endpointFilterMode === 'consumer'}
              onclick={() => {
                endpointFilterMode = 'consumer';
              }}
            >
              Consumed
            </button>
          </div>
        </div>
        <div class="min-w-[14rem] flex-1">
          <label for="io-endpoint-node-filter" class="mb-2 block text-xs uppercase tracking-[0.18em] text-neutral-500">
            Node id
          </label>
          <input
            id="io-endpoint-node-filter"
            type="text"
            bind:value={endpointNodeFilter}
            disabled={endpointFilterMode === 'all'}
            placeholder="node-id"
            class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none disabled:opacity-50"
          />
        </div>
      </div>
      {#if retentionSummary.length > 0}
        <div class="flex flex-wrap items-center gap-2 border-b border-neutral-900 px-4 py-3 text-xs">
          <span class="text-neutral-500">{summarizedArtifactCount} artifacts</span>
          {#each retentionSummary as item (item.retention_state)}
            <span class="inline-flex items-center gap-2 rounded border border-neutral-800 bg-neutral-950 px-2 py-1 text-neutral-300">
              <span>{formatIoArtifactRetentionStateLabel(item.retention_state)}</span>
              <span class="font-mono text-neutral-500">{item.artifact_count}</span>
            </span>
          {/each}
        </div>
      {/if}

      {#if artifactError}
        <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{artifactError}</div>
      {/if}

      {#if loadingArtifacts && artifacts.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">Loading artifacts</div>
      {:else if artifacts.length === 0}
        <div class="px-4 py-8 text-sm text-neutral-500">
          {#if $activeWorkflowRun}
            No retained artifact metadata for this run
          {:else}
            No retained artifact metadata available
          {/if}
        </div>
      {:else}
        <div class="grid gap-3 border-b border-neutral-900 p-4 lg:grid-cols-2">
          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <div class="flex items-center justify-between gap-3">
              <h2 class="text-sm font-semibold text-neutral-100">Workflow Inputs</h2>
              <span class="rounded border border-neutral-700 px-2 py-0.5 text-xs text-neutral-300">
                {workflowInputArtifacts.length}
              </span>
            </div>

            {#if workflowInputArtifacts.length === 0}
              <div class="mt-3 text-sm text-neutral-500">No retained workflow input metadata</div>
            {:else}
              <div class="mt-3 space-y-2">
                {#each workflowInputArtifacts.slice(0, 5) as artifact (artifact.event_id)}
                  <div class="min-w-0 rounded border border-neutral-800 bg-neutral-950/60 px-3 py-2">
                    <div class="truncate font-mono text-xs text-neutral-100" title={artifact.artifact_id}>
                      {artifact.artifact_id}
                    </div>
                    <div class="mt-1 text-xs text-neutral-500">
                      {formatIoArtifactMediaLabel(artifact.media_type)} · {formatIoArtifactBytes(artifact.size_bytes)}
                    </div>
                  </div>
                {/each}
                {#if workflowInputArtifacts.length > 5}
                  <div class="text-xs text-neutral-500">+{workflowInputArtifacts.length - 5} more</div>
                {/if}
              </div>
            {/if}
          </section>

          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <div class="flex items-center justify-between gap-3">
              <h2 class="text-sm font-semibold text-neutral-100">Workflow Outputs</h2>
              <span class="rounded border border-neutral-700 px-2 py-0.5 text-xs text-neutral-300">
                {workflowOutputArtifacts.length}
              </span>
            </div>

            {#if workflowOutputArtifacts.length === 0}
              <div class="mt-3 text-sm text-neutral-500">No retained workflow output metadata</div>
            {:else}
              <div class="mt-3 space-y-2">
                {#each workflowOutputArtifacts.slice(0, 5) as artifact (artifact.event_id)}
                  <div class="min-w-0 rounded border border-neutral-800 bg-neutral-950/60 px-3 py-2">
                    <div class="truncate font-mono text-xs text-neutral-100" title={artifact.artifact_id}>
                      {artifact.artifact_id}
                    </div>
                    <div class="mt-1 text-xs text-neutral-500">
                      {formatIoArtifactMediaLabel(artifact.media_type)} · {formatIoArtifactBytes(artifact.size_bytes)}
                    </div>
                  </div>
                {/each}
                {#if workflowOutputArtifacts.length > 5}
                  <div class="text-xs text-neutral-500">+{workflowOutputArtifacts.length - 5} more</div>
                {/if}
              </div>
            {/if}
          </section>
        </div>

        <section class="border-b border-neutral-900 px-4 py-4">
          <div class="flex items-center justify-between gap-3">
            <h2 class="text-sm font-semibold text-neutral-100">Node I/O</h2>
            <span class="rounded border border-neutral-700 px-2 py-0.5 text-xs text-neutral-300">
              {nodeGroups.length}
            </span>
          </div>

          {#if nodeGroups.length === 0}
            <div class="mt-3 text-sm text-neutral-500">No retained node-level artifact metadata</div>
          {:else}
            <div class="mt-3 grid gap-2 xl:grid-cols-2 2xl:grid-cols-3">
              {#each nodeGroups.slice(0, 9) as group (group.node_id)}
                <article class="rounded border border-neutral-800 bg-neutral-900/50 px-3 py-2">
                  <div class="truncate font-mono text-xs text-neutral-100" title={group.node_id}>
                    {group.node_id}
                  </div>
                  <div class="mt-1 truncate text-xs text-neutral-500" title={group.node_type ?? ''}>
                    {group.node_type ?? 'Unknown node type'}
                  </div>
                  <dl class="mt-3 grid grid-cols-3 gap-2 text-xs">
                    <div>
                      <dt class="text-neutral-500">Inputs</dt>
                      <dd class="mt-0.5 text-neutral-200">{group.input_count}</dd>
                    </div>
                    <div>
                      <dt class="text-neutral-500">Outputs</dt>
                      <dd class="mt-0.5 text-neutral-200">{group.output_count}</dd>
                    </div>
                    <div>
                      <dt class="text-neutral-500">Total</dt>
                      <dd class="mt-0.5 text-neutral-200">{group.artifact_count}</dd>
                    </div>
                  </dl>
                </article>
              {/each}
            </div>
            {#if nodeGroups.length > 9}
              <div class="mt-3 text-xs text-neutral-500">+{nodeGroups.length - 9} more nodes</div>
            {/if}
          {/if}
        </section>

        <div class="grid gap-3 p-4 xl:grid-cols-2 2xl:grid-cols-3">
          {#each artifacts as artifact (artifact.event_id)}
            {@const renderer = buildIoArtifactRendererSummary(artifact)}
            <article class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="truncate font-mono text-xs text-neutral-100" title={artifact.artifact_id}>
                    {artifact.artifact_id}
                  </div>
                  <div class="mt-1 text-xs text-neutral-500">
                    {formatIoArtifactRoleLabel(artifact.artifact_role)} · {formatIoArtifactMediaLabel(artifact.media_type)}
                  </div>
                </div>
                <span class="shrink-0 rounded border border-neutral-700 px-2 py-0.5 text-xs text-neutral-300">
                  {formatIoArtifactAvailabilityLabel(artifact)}
                </span>
              </div>

              <div class="mt-4 rounded border border-neutral-800 bg-neutral-950/70 px-3 py-3">
                <div class="flex items-center gap-2 text-sm text-neutral-100">
                  {#if renderer.family === 'text'}
                    <FileText size={16} aria-hidden="true" class="text-cyan-300" />
                  {:else if renderer.family === 'image'}
                    <ImageIcon size={16} aria-hidden="true" class="text-emerald-300" />
                  {:else if renderer.family === 'audio'}
                    <Music size={16} aria-hidden="true" class="text-amber-300" />
                  {:else if renderer.family === 'video'}
                    <Video size={16} aria-hidden="true" class="text-rose-300" />
                  {:else if renderer.family === 'table'}
                    <Table2 size={16} aria-hidden="true" class="text-sky-300" />
                  {:else if renderer.family === 'json'}
                    <Braces size={16} aria-hidden="true" class="text-violet-300" />
                  {:else if renderer.family === 'file'}
                    <File size={16} aria-hidden="true" class="text-neutral-300" />
                  {:else}
                    <CircleHelp size={16} aria-hidden="true" class="text-neutral-400" />
                  {/if}
                  <span>{renderer.title}</span>
                </div>
                <div class="mt-2 text-xs text-neutral-500">{renderer.detail}</div>
              </div>

              <dl class="mt-4 grid grid-cols-2 gap-x-3 gap-y-2 text-xs">
                <div>
                  <dt class="text-neutral-500">Size</dt>
                  <dd class="mt-0.5 text-neutral-200">{formatIoArtifactBytes(artifact.size_bytes)}</dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Producer</dt>
                  <dd
                    class="mt-0.5 truncate text-neutral-200"
                    title={formatIoArtifactEndpointValue(artifact.producer_node_id, artifact.producer_port_id)}
                  >
                    {formatIoArtifactEndpointValue(artifact.producer_node_id, artifact.producer_port_id)}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Consumer</dt>
                  <dd
                    class="mt-0.5 truncate text-neutral-200"
                    title={formatIoArtifactEndpointValue(artifact.consumer_node_id, artifact.consumer_port_id)}
                  >
                    {formatIoArtifactEndpointValue(artifact.consumer_node_id, artifact.consumer_port_id)}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Event Node</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.node_id ?? ''}>
                    {formatIoArtifactDetailValue(artifact.node_id)}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Run</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.workflow_run_id}>
                    {artifact.workflow_run_id}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Retention</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.retention_reason ?? ''}>
                    {formatIoArtifactRetentionStateLabel(artifact.retention_state)}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Policy</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.retention_policy_id ?? ''}>
                    {formatIoArtifactDetailValue(artifact.retention_policy_id)}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Observed</dt>
                  <dd class="mt-0.5 text-neutral-200">{formatTimestamp(artifact.occurred_at_ms)}</dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Runtime</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.runtime_id ?? ''}>
                    {formatIoArtifactDetailValue(artifact.runtime_id)}
                  </dd>
                </div>
                <div>
                  <dt class="text-neutral-500">Model</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.model_id ?? ''}>
                    {formatIoArtifactDetailValue(artifact.model_id)}
                  </dd>
                </div>
                <div class="col-span-2">
                  <dt class="text-neutral-500">Retention Reason</dt>
                  <dd class="mt-0.5 truncate text-neutral-200" title={artifact.retention_reason ?? ''}>
                    {formatIoArtifactDetailValue(artifact.retention_reason)}
                  </dd>
                </div>
                <div class="col-span-2">
                  <dt class="text-neutral-500">Payload Ref</dt>
                  <dd class="mt-0.5 truncate font-mono text-neutral-200" title={artifact.payload_ref ?? ''}>
                    {formatIoArtifactDetailValue(artifact.payload_ref)}
                  </dd>
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
        {#if retentionCleanupMessage}
          <div class="rounded border border-emerald-900 bg-emerald-950/40 px-3 py-2 text-sm text-emerald-200">
            {retentionCleanupMessage}
          </div>
        {/if}

        {#if retentionPolicyRows.length > 0}
          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-3">
            <h3 class="text-xs font-semibold uppercase tracking-[0.18em] text-neutral-500">Current Policy</h3>
            <dl class="mt-3 space-y-2 text-xs">
              {#each retentionPolicyRows as row (row.label)}
                <div>
                  <dt class="text-neutral-500">{row.label}</dt>
                  <dd class={`mt-0.5 truncate text-neutral-200 ${row.mono ? 'font-mono' : ''}`} title={row.value}>
                    {row.value}
                  </dd>
                </div>
              {/each}
            </dl>
          </section>
        {/if}

        {#if retentionPolicySettingRows.length > 0}
          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-3">
            <h3 class="text-xs font-semibold uppercase tracking-[0.18em] text-neutral-500">Retention Settings</h3>
            <dl class="mt-3 space-y-2 text-xs">
              {#each retentionPolicySettingRows as row (row.label)}
                <div>
                  <dt class="text-neutral-500">{row.label}</dt>
                  <dd class={`mt-0.5 truncate text-neutral-200 ${row.mono ? 'font-mono' : ''}`} title={row.value}>
                    {row.value}
                  </dd>
                </div>
              {/each}
            </dl>
          </section>
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

        <button
          type="button"
          class="inline-flex w-full items-center justify-center gap-2 rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-neutral-200 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
          onclick={() => {
            void applyRetentionCleanup();
          }}
          disabled={applyingRetentionCleanup || loadingRetention}
        >
          <Trash2 size={14} aria-hidden="true" />
          {applyingRetentionCleanup ? 'Applying Cleanup' : 'Apply Cleanup'}
        </button>

        {#if retentionCleanupRows.length > 0}
          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-3">
            <h3 class="text-xs font-semibold uppercase tracking-[0.18em] text-neutral-500">Last Cleanup</h3>
            <dl class="mt-3 space-y-2 text-xs">
              {#each retentionCleanupRows as row (row.label)}
                <div>
                  <dt class="text-neutral-500">{row.label}</dt>
                  <dd class={`mt-0.5 truncate text-neutral-200 ${row.mono ? 'font-mono' : ''}`} title={row.value}>
                    {row.value}
                  </dd>
                </div>
              {/each}
            </dl>
          </section>
        {/if}
      </form>
    </aside>
  </div>
</section>
