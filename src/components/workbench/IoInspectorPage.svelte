<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    Braces,
    Check,
    CircleHelp,
    Download,
    Eye,
    File,
    FileText,
    Image as ImageIcon,
    Music,
    RefreshCw,
    Settings,
    Table2,
    Video,
  } from 'lucide-svelte';
  import type {
    IoArtifactProjectionRecord,
    IoArtifactRetentionSummaryRecord,
    NodeStatusProjectionRecord,
    ProjectionStateRecord,
    ResolvedNodeIoRecord,
  } from '../../services/diagnostics/types';
  import type {
    WorkflowArtifactBodyRead,
    WorkflowArtifactStreamBodyRead,
    WorkflowRunGraphProjection,
  } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import {
    activeWorkflowRun,
    focusSettingsSection,
  } from '../../stores/workbenchStore';
  import {
    buildIoArtifactDescriptorMetadataRows,
    buildIoArtifactDownloadFilename,
    buildIoArtifactPreviewReadRequest,
    buildIoArtifactRendererSummary,
    buildResolvedNodeIoDisplayRows,
    canRenderIoArtifactTextPreview,
    canAcknowledgeIoArtifactConsumed,
    canReadIoArtifactBody,
    decodeIoArtifactTextPreview,
    formatIoArtifactAvailabilityLabel,
    formatIoArtifactBytes,
    formatIoArtifactDetailValue,
    formatIoArtifactEndpointValue,
    formatIoArtifactMediaLabel,
    formatIoArtifactPreviewExtent,
    formatIoArtifactRetentionStateLabel,
    formatIoArtifactRoleLabel,
    formatProjectionFreshness,
    ioArtifactPayloadTargetId,
  } from './ioInspectorPresenters';
  import {
    buildRunGraphNodeArtifactSummaries,
    buildRunGraphNodeStatusMap,
  } from './runGraphPresenters';
  import RunGraphSnapshot from './RunGraphSnapshot.svelte';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  interface ArtifactBodyPreview {
    objectUrl: string;
    mediaType: string;
    byteLength: number;
    complete: boolean;
    contentHash?: string | null;
    text?: string | null;
    textTruncated?: boolean;
  }

  const DOWNLOAD_OBJECT_URL_REVOKE_DELAY_MS = 30_000;

  let runGraph = $state<WorkflowRunGraphProjection | null>(null);
  let runNodeStatuses = $state<NodeStatusProjectionRecord[]>([]);
  let artifacts = $state<IoArtifactProjectionRecord[]>([]);
  let resolvedNodeIo = $state<ResolvedNodeIoRecord[]>([]);
  let retentionSummary = $state<IoArtifactRetentionSummaryRecord[]>([]);
  let projectionState = $state<ProjectionStateRecord | null>(null);
  let selectedNodeId = $state<string | null>(null);
  let selectedBackendFilter = $state('');
  let loadingInspector = $state(false);
  let inspectorError = $state<string | null>(null);
  let artifactBodyPreviews = $state<Record<string, ArtifactBodyPreview>>({});
  let artifactAccessLoading = $state<Record<string, boolean>>({});
  let artifactConsumeLoading = $state<Record<string, boolean>>({});
  let artifactAccessErrors = $state<Record<string, string>>({});
  let artifactConsumeMessages = $state<Record<string, string>>({});
  let inspectorRequestSerial = 0;

  let artifactSummaries = $derived(buildRunGraphNodeArtifactSummaries(artifacts));
  let nodeStatuses = $derived(buildRunGraphNodeStatusMap(runNodeStatuses));
  let selectedInputRows = $derived(
    selectedNodeId
      ? buildResolvedNodeIoDisplayRows(
          resolvedNodeIo.filter((record) => record.node_id === selectedNodeId && record.direction === 'input'),
          artifacts,
        )
      : [],
  );
  let selectedOutputRows = $derived(
    selectedNodeId
      ? buildResolvedNodeIoDisplayRows(
          resolvedNodeIo.filter((record) => record.node_id === selectedNodeId && record.direction === 'output'),
          artifacts,
        )
      : [],
  );
  let selectedInputArtifacts = $derived(
    selectedInputRows.map((row) => row.artifact).filter(isPresentArtifact),
  );
  let selectedOutputArtifacts = $derived(
    selectedOutputRows.map((row) => row.artifact).filter(isPresentArtifact),
  );
  let selectedArtifacts = $derived([...selectedOutputArtifacts, ...selectedInputArtifacts]);
  let summarizedArtifactCount = $derived(
    retentionSummary.reduce((total, item) => total + item.artifact_count, 0),
  );

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  function formatTimestamp(value: number): string {
    return new Date(value).toLocaleString();
  }

  async function refreshInspector(
    runId = activeRunId(),
    backendFilterValue = selectedBackendFilter.trim(),
  ): Promise<void> {
    const requestSerial = ++inspectorRequestSerial;
    inspectorError = null;
    loadingInspector = true;

    if (!runId) {
      runGraph = null;
      runNodeStatuses = [];
      artifacts = [];
      resolvedNodeIo = [];
      retentionSummary = [];
      projectionState = null;
      selectedNodeId = null;
      loadingInspector = false;
      return;
    }

    try {
      const selectedBackendKey = backendFilterValue.trim();
      const inspectionResponse = await workflowService.queryRunInspection({
        workflow_run_id: runId,
        artifact_limit: 250,
      });
      if (requestSerial !== inspectorRequestSerial) {
        return;
      }

      const nextArtifacts =
        selectedBackendKey.length > 0
          ? inspectionResponse.io_artifacts.filter((artifact) => artifact.selected_backend_key === selectedBackendKey)
          : inspectionResponse.io_artifacts;
      const nextResolvedNodeIo = filterResolvedNodeIoByArtifacts(
        inspectionResponse.resolved_node_io ?? [],
        nextArtifacts,
        selectedBackendKey,
      );
      revokeMissingArtifactObjectUrls(nextArtifacts);
      runGraph = inspectionResponse.run_graph ?? null;
      runNodeStatuses = inspectionResponse.node_statuses;
      artifacts = nextArtifacts;
      resolvedNodeIo = nextResolvedNodeIo;
      retentionSummary = inspectionResponse.retention_summary;
      projectionState = inspectionResponse.io_projection_state;
      selectedNodeId = resolveSelectedNodeId(
        selectedNodeId,
        inspectionResponse.run_graph,
        nextArtifacts,
      );
    } catch (error) {
      if (requestSerial !== inspectorRequestSerial) {
        return;
      }
      inspectorError = formatWorkflowCommandError(error);
      runGraph = null;
      runNodeStatuses = [];
      artifacts = [];
      resolvedNodeIo = [];
      retentionSummary = [];
      projectionState = null;
      selectedNodeId = null;
    } finally {
      if (requestSerial === inspectorRequestSerial) {
        loadingInspector = false;
      }
    }
  }

  function resolveSelectedNodeId(
    currentNodeId: string | null,
    nextRunGraph: WorkflowRunGraphProjection | null | undefined,
    nextArtifacts: IoArtifactProjectionRecord[],
  ): string | null {
    const nodeIds = nextRunGraph?.graph.nodes.map((node) => node.id) ?? [];
    if (currentNodeId && nodeIds.includes(currentNodeId)) {
      return currentNodeId;
    }

    const artifactNodeId = nextArtifacts
      .map((artifact) => artifact.producer_node_id ?? artifact.consumer_node_id ?? artifact.node_id)
      .find((nodeId): nodeId is string => Boolean(nodeId && nodeIds.includes(nodeId)));
    return artifactNodeId ?? nodeIds[0] ?? null;
  }

  function isPresentArtifact(
    artifact: IoArtifactProjectionRecord | null,
  ): artifact is IoArtifactProjectionRecord {
    return artifact !== null;
  }

  function filterResolvedNodeIoByArtifacts(
    records: ResolvedNodeIoRecord[],
    nextArtifacts: IoArtifactProjectionRecord[],
    selectedBackendKey: string,
  ): ResolvedNodeIoRecord[] {
    if (selectedBackendKey.length === 0) {
      return records;
    }

    const artifactFactIds = new Set(nextArtifacts.map((artifact) => artifact.artifact_fact_id).filter(isNonEmptyString));
    const payloadArtifactIds = new Set(nextArtifacts.map((artifact) => artifact.payload_artifact_id).filter(isNonEmptyString));
    const artifactIds = new Set(nextArtifacts.map((artifact) => artifact.artifact_id).filter(isNonEmptyString));
    return records.filter((record) =>
      (record.artifact_fact_id && artifactFactIds.has(record.artifact_fact_id)) ||
      (record.payload_artifact_id && payloadArtifactIds.has(record.payload_artifact_id)) ||
      (record.artifact_id && artifactIds.has(record.artifact_id)),
    );
  }

  function isNonEmptyString(value: string | null | undefined): value is string {
    return Boolean(value && value.trim().length > 0);
  }

  async function readArtifactPreview(artifact: IoArtifactProjectionRecord): Promise<void> {
    const payloadArtifactId = ioArtifactPayloadTargetId(artifact);
    setArtifactLoading(artifact.artifact_id, true);
    setArtifactAccessError(artifact.artifact_id, null);
    try {
      await verifyArtifactReadable(artifact);
      const read = await workflowService.readArtifactBody(
        buildIoArtifactPreviewReadRequest(payloadArtifactId),
      );
      replaceArtifactBodyPreview(artifact.artifact_id, createArtifactBodyPreview(read));
    } catch (error) {
      setArtifactAccessError(artifact.artifact_id, formatWorkflowCommandError(error));
    } finally {
      setArtifactLoading(artifact.artifact_id, false);
    }
  }

  async function readArtifactStreamPreview(artifact: IoArtifactProjectionRecord): Promise<void> {
    const payloadArtifactId = ioArtifactPayloadTargetId(artifact);
    setArtifactLoading(artifact.artifact_id, true);
    setArtifactAccessError(artifact.artifact_id, null);
    try {
      await verifyArtifactStreamReadable(artifact);
      const read = await workflowService.readArtifactStream(
        buildIoArtifactPreviewReadRequest(payloadArtifactId),
      );
      replaceArtifactBodyPreview(artifact.artifact_id, createArtifactBodyPreview(read));
    } catch (error) {
      setArtifactAccessError(artifact.artifact_id, formatWorkflowCommandError(error));
    } finally {
      setArtifactLoading(artifact.artifact_id, false);
    }
  }

  async function downloadArtifactBody(artifact: IoArtifactProjectionRecord): Promise<void> {
    const payloadArtifactId = ioArtifactPayloadTargetId(artifact);
    setArtifactLoading(artifact.artifact_id, true);
    setArtifactAccessError(artifact.artifact_id, null);
    try {
      await verifyArtifactReadable(artifact);
      const read = await workflowService.readArtifactBody({ artifact_id: payloadArtifactId });
      const preview = createArtifactBodyPreview(read);
      const anchor = document.createElement('a');
      anchor.href = preview.objectUrl;
      anchor.download = buildIoArtifactDownloadFilename({
        artifact_id: payloadArtifactId,
        media_type: read.response.media_type || artifact.media_type,
        format: artifact.format,
        payload_kind: artifact.payload_kind,
      });
      anchor.rel = 'noopener noreferrer';
      anchor.style.display = 'none';
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => {
        revokeObjectUrl(preview.objectUrl);
      }, DOWNLOAD_OBJECT_URL_REVOKE_DELAY_MS);
    } catch (error) {
      setArtifactAccessError(artifact.artifact_id, formatWorkflowCommandError(error));
    } finally {
      setArtifactLoading(artifact.artifact_id, false);
    }
  }

  async function acknowledgeArtifactConsumed(artifact: IoArtifactProjectionRecord): Promise<void> {
    const payloadArtifactId = ioArtifactPayloadTargetId(artifact);
    setArtifactConsumeLoading(artifact.artifact_id, true);
    setArtifactAccessError(artifact.artifact_id, null);
    artifactConsumeMessages = withoutArtifactKey(artifactConsumeMessages, artifact.artifact_id);
    try {
      const response = await workflowService.acknowledgeArtifactConsumed({
        artifact_id: payloadArtifactId,
        consumer_id: 'pantograph-gui-io-inspector',
      });
      if (!response.retained_after_consume) {
        releaseArtifactBodyPreview(artifact.artifact_id);
      }
      artifactConsumeMessages = {
        ...artifactConsumeMessages,
        [artifact.artifact_id]: response.retained_after_consume
          ? 'Consume acknowledged; payload retained'
          : 'Consume acknowledged; payload released',
      };
      await refreshInspector();
    } catch (error) {
      setArtifactAccessError(artifact.artifact_id, formatWorkflowCommandError(error));
    } finally {
      setArtifactConsumeLoading(artifact.artifact_id, false);
    }
  }

  async function verifyArtifactReadable(artifact: IoArtifactProjectionRecord): Promise<void> {
    const response = await workflowService.artifactDescriptor({
      artifact_id: ioArtifactPayloadTargetId(artifact),
    });
    const descriptor = response.artifact;
    if (!descriptor) {
      throw new Error('Artifact descriptor unavailable');
    }
    if (descriptor.retention_state !== 'retained' || descriptor.lifecycle_state !== 'retained') {
      throw new Error('Artifact body is not retained');
    }
    if (
      !descriptor.read_handle &&
      !descriptor.access_modes.includes('read') &&
      !descriptor.access_modes.includes('download')
    ) {
      throw new Error('Artifact body is not readable');
    }
  }

  async function verifyArtifactStreamReadable(artifact: IoArtifactProjectionRecord): Promise<void> {
    const descriptor = await workflowService
      .artifactDescriptor({ artifact_id: ioArtifactPayloadTargetId(artifact) })
      .then((response) => response.artifact ?? null);
    if (!descriptor) {
      throw new Error('Artifact descriptor is unavailable');
    }
    if (!descriptor.stream_handle && !descriptor.access_modes.includes('stream')) {
      throw new Error('Artifact stream is not readable');
    }
  }

  function createArtifactBodyPreview(
    read: WorkflowArtifactBodyRead | WorkflowArtifactStreamBodyRead,
  ): ArtifactBodyPreview {
    if (read.response.body_transport !== 'binary_body') {
      throw new Error('Artifact body transport is not available as a binary body');
    }
    const byteArray = Uint8Array.from(read.body);
    const mediaType = read.response.media_type || 'application/octet-stream';
    const blob = new Blob([byteArray], { type: mediaType });
    const textPreview = canRenderIoArtifactTextPreview(mediaType)
      ? decodeIoArtifactTextPreview(byteArray)
      : null;
    return {
      objectUrl: URL.createObjectURL(blob),
      mediaType,
      byteLength: read.response.byte_length,
      complete: read.response.complete,
      contentHash: 'content_hash' in read.response ? read.response.content_hash : null,
      text: textPreview?.text ?? null,
      textTruncated: textPreview?.truncated ?? false,
    };
  }

  function replaceArtifactBodyPreview(artifactId: string, preview: ArtifactBodyPreview): void {
    const existing = artifactBodyPreviews[artifactId];
    if (existing) {
      revokeObjectUrl(existing.objectUrl);
    }
    artifactBodyPreviews = {
      ...artifactBodyPreviews,
      [artifactId]: preview,
    };
  }

  function releaseArtifactBodyPreview(artifactId: string): void {
    const existing = artifactBodyPreviews[artifactId];
    if (!existing) {
      return;
    }
    revokeObjectUrl(existing.objectUrl);
    artifactBodyPreviews = withoutArtifactKey(artifactBodyPreviews, artifactId);
  }

  function revokeMissingArtifactObjectUrls(nextArtifacts: IoArtifactProjectionRecord[]): void {
    const nextIds = new Set(nextArtifacts.map((artifact) => artifact.artifact_id));
    let nextPreviews = artifactBodyPreviews;
    let changed = false;
    for (const [artifactId, preview] of Object.entries(artifactBodyPreviews)) {
      if (!nextIds.has(artifactId)) {
        revokeObjectUrl(preview.objectUrl);
        if (!changed) {
          nextPreviews = { ...nextPreviews };
          changed = true;
        }
        delete nextPreviews[artifactId];
      }
    }
    if (changed) {
      artifactBodyPreviews = nextPreviews;
    }
  }

  function revokeAllArtifactObjectUrls(): void {
    for (const preview of Object.values(artifactBodyPreviews)) {
      revokeObjectUrl(preview.objectUrl);
    }
    artifactBodyPreviews = {};
  }

  function revokeObjectUrl(objectUrl: string): void {
    URL.revokeObjectURL(objectUrl);
  }

  function setArtifactLoading(artifactId: string, loading: boolean): void {
    artifactAccessLoading = setArtifactFlag(artifactAccessLoading, artifactId, loading);
  }

  function setArtifactConsumeLoading(artifactId: string, loading: boolean): void {
    artifactConsumeLoading = setArtifactFlag(artifactConsumeLoading, artifactId, loading);
  }

  function setArtifactAccessError(artifactId: string, message: string | null): void {
    artifactAccessErrors = message
      ? { ...artifactAccessErrors, [artifactId]: message }
      : withoutArtifactKey(artifactAccessErrors, artifactId);
  }

  function setArtifactFlag(
    flags: Record<string, boolean>,
    artifactId: string,
    value: boolean,
  ): Record<string, boolean> {
    return value ? { ...flags, [artifactId]: true } : withoutArtifactKey(flags, artifactId);
  }

  function withoutArtifactKey<T>(record: Record<string, T>, artifactId: string): Record<string, T> {
    const next = { ...record };
    delete next[artifactId];
    return next;
  }

  $effect(() => {
    const runId = activeRunId();
    const backendFilterValue = selectedBackendFilter.trim();
    void refreshInspector(runId, backendFilterValue);
  });

  onDestroy(() => {
    revokeAllArtifactObjectUrls();
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between gap-4 border-b border-neutral-800 px-4 py-3">
    <div class="min-w-0">
      <h1 class="text-base font-semibold text-neutral-100">I/O Inspector</h1>
      <div class="mt-1 truncate text-xs text-neutral-500">
        {#if $activeWorkflowRun}
          {$activeWorkflowRun.workflow_run_id}
        {:else}
          Select a workflow run in Scheduler to inspect node I/O
        {/if}
      </div>
    </div>
    <div class="flex shrink-0 items-center gap-2">
      <button
        type="button"
        class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
        onclick={() => focusSettingsSection('diagnostics_retention')}
      >
        <Settings size={14} aria-hidden="true" />
        Retention Settings
      </button>
      <button
        type="button"
        class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
        onclick={() => refreshInspector()}
        disabled={loadingInspector}
      >
        <RefreshCw size={14} aria-hidden="true" class={loadingInspector ? 'animate-spin' : ''} />
        Refresh
      </button>
    </div>
  </div>

  {#if inspectorError}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{inspectorError}</div>
  {/if}

  {#if !$activeWorkflowRun}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      No active run selected
    </div>
  {:else}
    <div class="grid min-h-0 flex-1 grid-rows-[minmax(18rem,42%)_minmax(0,1fr)] overflow-hidden">
      <section class="min-h-0 border-b border-neutral-800">
        {#if loadingInspector && !runGraph}
          <div class="flex h-full items-center justify-center text-sm text-neutral-500">Loading run snapshot</div>
        {:else if !runGraph}
          <div class="flex h-full items-center justify-center text-sm text-neutral-500">
            No versioned graph captured for this run
          </div>
        {:else}
          <RunGraphSnapshot
            {runGraph}
            {artifactSummaries}
            {nodeStatuses}
            compact
            {selectedNodeId}
            onSelectNode={(nodeId) => {
              selectedNodeId = nodeId;
            }}
          />
        {/if}
      </section>

      <section class="flex min-h-0 flex-col overflow-hidden">
        <div class="shrink-0 border-b border-neutral-900 px-4 py-3">
          <div class="flex flex-wrap items-end justify-between gap-3">
            <div class="min-w-0">
              <h2 class="text-sm font-semibold text-neutral-100">
                {selectedNodeId ? `Node I/O: ${selectedNodeId}` : 'Node I/O'}
              </h2>
              <div class="mt-1 text-xs text-neutral-500">
                {formatProjectionFreshness(projectionState)}
                {#if summarizedArtifactCount > 0}
                  · {summarizedArtifactCount} retained artifacts in run
                {/if}
              </div>
            </div>
            <div class="flex min-w-[14rem] items-end gap-2">
              <label class="min-w-0 flex-1">
                <span class="mb-2 block text-xs uppercase tracking-[0.18em] text-neutral-500">Backend</span>
                <input
                  type="text"
                  bind:value={selectedBackendFilter}
                  placeholder="all"
                  class="w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-cyan-500 focus:outline-none"
                />
              </label>
            </div>
          </div>
          {#if retentionSummary.length > 0}
            <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
              {#each retentionSummary as item (item.retention_state)}
                <span class="inline-flex items-center gap-2 rounded border border-neutral-800 bg-neutral-950 px-2 py-1 text-neutral-300">
                  <span>{formatIoArtifactRetentionStateLabel(item.retention_state)}</span>
                  <span class="font-mono text-neutral-500">{item.artifact_count}</span>
                </span>
              {/each}
            </div>
          {/if}
          {#if selectedNodeId}
            <div class="mt-3 flex flex-wrap gap-2 text-xs">
              <span class="rounded border border-neutral-800 bg-neutral-900 px-2 py-1 text-neutral-300">
                Outputs {selectedOutputArtifacts.length}
              </span>
              <span class="rounded border border-neutral-800 bg-neutral-900 px-2 py-1 text-neutral-300">
                Inputs {selectedInputArtifacts.length}
              </span>
            </div>
          {/if}
        </div>

        <div class="min-h-0 overflow-auto p-4">
          {#if !selectedNodeId}
            <div class="text-sm text-neutral-500">Select a node in the run snapshot to inspect retained I/O.</div>
          {:else if selectedArtifacts.length === 0}
            <div class="text-sm text-neutral-500">No retained artifact metadata for the selected node.</div>
          {:else}
            <div class="grid gap-3 xl:grid-cols-2 2xl:grid-cols-3">
              {#each selectedArtifacts as artifact (artifact.event_id)}
                {@const renderer = buildIoArtifactRendererSummary(artifact)}
                {@const descriptorRows = buildIoArtifactDescriptorMetadataRows(artifact)}
                {@const bodyPreview = artifactBodyPreviews[artifact.artifact_id]}
                <article class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate font-mono text-xs text-neutral-100" title={artifact.artifact_id}>
                        {artifact.artifact_id}
                      </div>
                      <div class="mt-1 text-xs text-neutral-500">
                        {formatIoArtifactRoleLabel(artifact.artifact_role)} · {formatIoArtifactMediaLabel(
                          artifact.media_type ?? artifact.format?.media_type,
                          artifact.payload_kind,
                        )}
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
                      {:else if renderer.family === '3d'}
                        <File size={16} aria-hidden="true" class="text-indigo-300" />
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

                    {#if bodyPreview}
                      <div class="mt-3 overflow-hidden rounded border border-neutral-800 bg-neutral-900/80">
                        {#if renderer.family === 'image'}
                          <img
                            src={bodyPreview.objectUrl}
                            alt={`Preview of ${artifact.artifact_id}`}
                            class="max-h-64 w-full object-contain"
                          />
                        {:else if renderer.family === 'audio'}
                          <audio
                            src={bodyPreview.objectUrl}
                            controls
                            class="w-full"
                            aria-label={`Audio preview of ${artifact.artifact_id}`}
                          ></audio>
                        {:else if renderer.family === 'video'}
                          <!-- svelte-ignore a11y_media_has_caption -->
                          <video
                            src={bodyPreview.objectUrl}
                            controls
                            class="max-h-64 w-full bg-black"
                            aria-label={`Video preview of ${artifact.artifact_id}`}
                          ></video>
                        {:else if bodyPreview.text !== null && bodyPreview.text !== undefined}
                          <pre
                            class="max-h-72 overflow-auto whitespace-pre-wrap break-words px-3 py-2 font-mono text-xs leading-relaxed text-neutral-100"
                          >{bodyPreview.text}</pre>
                        {:else}
                          <div class="px-3 py-2 text-xs text-neutral-400">
                            Binary preview loaded. Use Download to inspect the retained body outside Pantograph.
                          </div>
                        {/if}
                      </div>
                      <div class="mt-2 text-xs text-neutral-500">
                        {formatIoArtifactPreviewExtent(bodyPreview)}
                        {#if bodyPreview.textTruncated}
                          · text truncated
                        {/if}
                      </div>
                    {/if}
                  </div>

                  <div class="mt-3 flex flex-wrap gap-2">
                    <button
                      type="button"
                      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-xs text-neutral-200 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                      onclick={() => {
                        void readArtifactPreview(artifact);
                      }}
                      aria-label={`Read retained artifact ${artifact.artifact_id}`}
                      disabled={!canReadIoArtifactBody(artifact) || artifactAccessLoading[artifact.artifact_id]}
                    >
                      <Eye size={14} aria-hidden="true" />
                      {bodyPreview ? 'Refresh Read' : 'Read'}
                    </button>
                    <button
                      type="button"
                      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-xs text-neutral-200 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                      onclick={() => {
                        void readArtifactStreamPreview(artifact);
                      }}
                      aria-label={`Read artifact stream ${artifact.artifact_id}`}
                      disabled={!artifact.stream_handle || artifactAccessLoading[artifact.artifact_id]}
                    >
                      <RefreshCw size={14} aria-hidden="true" />
                      {bodyPreview ? 'Refresh Stream' : 'Read Stream'}
                    </button>
                    <button
                      type="button"
                      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-xs text-neutral-200 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                      onclick={() => {
                        void downloadArtifactBody(artifact);
                      }}
                      aria-label={`Download retained artifact ${artifact.artifact_id}`}
                      disabled={!canReadIoArtifactBody(artifact) || artifactAccessLoading[artifact.artifact_id]}
                    >
                      <Download size={14} aria-hidden="true" />
                      Download
                    </button>
                    <button
                      type="button"
                      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-xs text-neutral-200 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
                      onclick={() => {
                        void acknowledgeArtifactConsumed(artifact);
                      }}
                      aria-label={`Acknowledge consume for artifact ${artifact.artifact_id}`}
                      disabled={!canAcknowledgeIoArtifactConsumed(artifact) || artifactConsumeLoading[artifact.artifact_id]}
                    >
                      <Check size={14} aria-hidden="true" />
                      {artifactConsumeLoading[artifact.artifact_id] ? 'Acknowledging' : 'Acknowledge'}
                    </button>
                  </div>

                  {#if artifactAccessErrors[artifact.artifact_id]}
                    <div class="mt-3 rounded border border-red-900 bg-red-950/50 px-3 py-2 text-xs text-red-200">
                      {artifactAccessErrors[artifact.artifact_id]}
                    </div>
                  {/if}

                  {#if artifactConsumeMessages[artifact.artifact_id]}
                    <div class="mt-3 rounded border border-emerald-900 bg-emerald-950/40 px-3 py-2 text-xs text-emerald-200">
                      {artifactConsumeMessages[artifact.artifact_id]}
                    </div>
                  {/if}

                  <dl class="mt-4 grid grid-cols-2 gap-x-3 gap-y-2 text-xs">
                    <div>
                      <dt class="text-neutral-500">Size</dt>
                      <dd class="mt-0.5 text-neutral-200">{formatIoArtifactBytes(artifact.size_bytes)}</dd>
                    </div>
                    <div>
                      <dt class="text-neutral-500">Producer</dt>
                      <dd class="mt-0.5 truncate text-neutral-200" title={formatIoArtifactEndpointValue(artifact.producer_node_id, artifact.producer_port_id)}>
                        {formatIoArtifactEndpointValue(artifact.producer_node_id, artifact.producer_port_id)}
                      </dd>
                    </div>
                    <div>
                      <dt class="text-neutral-500">Consumer</dt>
                      <dd class="mt-0.5 truncate text-neutral-200" title={formatIoArtifactEndpointValue(artifact.consumer_node_id, artifact.consumer_port_id)}>
                        {formatIoArtifactEndpointValue(artifact.consumer_node_id, artifact.consumer_port_id)}
                      </dd>
                    </div>
                    <div>
                      <dt class="text-neutral-500">Observed</dt>
                      <dd class="mt-0.5 text-neutral-200">{formatTimestamp(artifact.occurred_at_ms)}</dd>
                    </div>
                    <div>
                      <dt class="text-neutral-500">Backend</dt>
                      <dd class="mt-0.5 truncate text-neutral-200" title={artifact.selected_backend_key ?? ''}>
                        {formatIoArtifactDetailValue(artifact.selected_backend_key)}
                      </dd>
                    </div>
                    <div>
                      <dt class="text-neutral-500">Model</dt>
                      <dd class="mt-0.5 truncate text-neutral-200" title={artifact.model_id ?? ''}>
                        {formatIoArtifactDetailValue(artifact.model_id)}
                      </dd>
                    </div>
                    <div class="col-span-2">
                      <dt class="text-neutral-500">Payload Ref</dt>
                      <dd class="mt-0.5 truncate font-mono text-neutral-200" title={artifact.payload_ref ?? ''}>
                        {formatIoArtifactDetailValue(artifact.payload_ref)}
                      </dd>
                    </div>
                  </dl>

                  <section class="mt-4 rounded border border-neutral-800 bg-neutral-950/50 p-3">
                    <h3 class="text-xs font-semibold uppercase tracking-[0.18em] text-neutral-500">
                      Artifact Descriptor
                    </h3>
                    <dl class="mt-3 grid grid-cols-2 gap-x-3 gap-y-2 text-xs">
                      {#each descriptorRows as row (row.label)}
                        <div
                          class={row.label.includes('Handle') ||
                          row.label.includes('Version') ||
                          row.label.includes('Command') ||
                          row.label.includes('Dependency') ||
                          row.label.includes('Lease')
                            ? 'col-span-2'
                            : ''}
                        >
                          <dt class="text-neutral-500">{row.label}</dt>
                          <dd class={`mt-0.5 truncate text-neutral-200 ${row.mono ? 'font-mono' : ''}`} title={row.value}>
                            {row.value}
                          </dd>
                        </div>
                      {/each}
                    </dl>
                  </section>

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
      </section>
    </div>
  {/if}
</section>
