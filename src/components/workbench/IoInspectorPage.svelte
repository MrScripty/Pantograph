<script lang="ts">
  // I/O Inspector (workbench redesign M3, docs/design/workbench-redesign-spec.md).
  // Left: the selected run's nodes, each expandable to its retained ports.
  // Right: one artifact with Read body / Read stream / Download / Acknowledge.
  // Data authority is unchanged: `queryRunInspection` for the run, the artifact
  // descriptor/read/stream/consume commands for bodies, and `listWorkflows` +
  // `inspectWorkflowGraph` for the saved-workflow disclosure in the no-run state.
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import type {
    IoArtifactProjectionRecord,
    IoArtifactRetentionSummaryRecord,
    NodeStatusProjectionRecord,
    ProjectionStateRecord,
    ResolvedNodeIoRecord,
  } from '../../services/diagnostics/types';
  import { subscribeDiagnosticsProjectionInvalidations } from '../../services/workflow/WorkflowProjectionSubscriptionService';
  import type {
    WorkflowArtifactBodyRead,
    WorkflowArtifactStreamBodyRead,
    WorkflowGraphInspectionProjection,
    WorkflowMetadata,
    WorkflowRunGraphProjection,
  } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun, focusSettingsSection } from '../../stores/workbenchStore';
  import { openRunContextPicker } from './shell/runContextStore';
  import {
    Chip,
    DimensionLine,
    EmptyState,
    ErrorCard,
    FilterChip,
    KeyValue,
    SegmentedControl,
    StatusGlyph,
    glyphStateForStatus,
    type SegmentOption,
  } from './ui';
  import {
    IO_ARTIFACT_PREVIEW_MAX_BYTES,
    IO_INSPECTOR_BOUNDARY_NODE_ID,
    applyIoInspectorBackendFilter,
    assertCompleteIoArtifactImageRead,
    buildIoArtifactChipLabels,
    buildIoArtifactDetailSections,
    buildIoArtifactDownloadFilename,
    buildIoArtifactPreviewReadRequest,
    buildIoArtifactRendererSummary,
    buildIoInspectorDimensionLabel,
    buildIoInspectorNodeTree,
    buildStaleGraphFactCards,
    canAcknowledgeIoArtifactConsumed,
    canReadIoArtifactBody,
    canRenderIoArtifactTextPreview,
    countIoInspectorEntriesByDirection,
    createIoArtifactDownloadUrlOwner,
    decodeIoArtifactTextPreview,
    describeIoArtifactAcknowledgeUnavailable,
    describeIoArtifactReadUnavailable,
    describeIoArtifactStreamUnavailable,
    filterIoInspectorNodeTree,
    findIoInspectorEntry,
    formatIoArtifactBytes,
    formatIoArtifactPreviewExtent,
    formatIoArtifactRetentionStateLabel,
    formatIoInspectorDirectionMark,
    formatIoInspectorEntryDetail,
    formatIoInspectorNodeLabel,
    formatIoInspectorNodeSubLabel,
    formatProjectionFreshness,
    formatResolvedNodeIoDirectionLabel,
    ioArtifactPayloadTargetId,
    partitionGraphDiagnosticsByNode,
    resolveIoArtifactPreviewMode,
    resolveIoInspectorSelectedEntryKey,
    type IoInspectorDirectionFilter,
    type IoInspectorTreeEntry,
  } from './ioInspectorPresenters';
  import {
    buildSavedGraphInspectionDisplayModel,
    buildSavedGraphInspectionOptions,
    resolveSavedGraphSelectedNodeId,
    resolveSavedWorkflowInspectionPath,
    type SavedGraphInspectionOption,
  } from './graphInspectionPresenters';
  import SavedGraphInspectionSnapshot from './SavedGraphInspectionSnapshot.svelte';
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

  interface ArtifactAccessRequest {
    artifactId: string;
    payloadArtifactId: string;
    requestSerial: number;
    lifecycleGeneration: number;
  }

  const DOWNLOAD_OBJECT_URL_REVOKE_DELAY_MS = 30_000;
  const IMAGE_PREVIEW_DECODE_ERROR = 'Image preview could not be decoded.';
  const PREVIEW_LIMIT_LABEL = formatIoArtifactBytes(IO_ARTIFACT_PREVIEW_MAX_BYTES);

  // Run inspection projection (one `queryRunInspection` response).
  let runGraph = $state<WorkflowRunGraphProjection | null>(null);
  let runNodeStatuses = $state<NodeStatusProjectionRecord[]>([]);
  let artifacts = $state<IoArtifactProjectionRecord[]>([]);
  let resolvedNodeIo = $state<ResolvedNodeIoRecord[]>([]);
  let retentionSummary = $state<IoArtifactRetentionSummaryRecord[]>([]);
  let projectionState = $state<ProjectionStateRecord | null>(null);
  let loadingInspector = $state(false);
  let runInspectorError = $state<string | null>(null);
  let liveUpdatesError = $state<string | null>(null);

  // Presentation state for the tree and filters.
  let selectedEntryKey = $state<string | null>(null);
  let collapsedNodes = $state<Record<string, boolean>>({});
  let direction = $state<IoInspectorDirectionFilter>('all');
  let search = $state('');
  let filtersOpen = $state(false);
  let selectedBackendFilter = $state('');

  // Saved-workflow inspection (no-run disclosure).
  let savedInspectionOpen = $state(false);
  let savedWorkflows = $state<WorkflowMetadata[]>([]);
  let savedWorkflowOptions = $state<SavedGraphInspectionOption[]>([]);
  let selectedSavedWorkflowPath = $state<string | null>(null);
  let savedGraphInspection = $state<WorkflowGraphInspectionProjection | null>(null);
  let savedSelectedNodeId = $state<string | null>(null);
  let loadingSavedGraph = $state(false);
  let savedGraphError = $state<string | null>(null);

  // Artifact body access, keyed by artifact id.
  let artifactBodyPreviews = $state<Record<string, ArtifactBodyPreview>>({});
  let artifactAccessLoading = $state<Record<string, boolean>>({});
  let artifactConsumeLoading = $state<Record<string, boolean>>({});
  let artifactAccessErrors = $state<Record<string, string>>({});
  let artifactConsumeMessages = $state<Record<string, string>>({});
  let downloadLink = $state<{ url: string; filename: string } | null>(null);
  let downloadAnchor = $state<HTMLAnchorElement | null>(null);

  let inspectorRequestSerial = 0;
  let savedGraphRequestSerial = 0;
  let artifactAccessRequestSerial = 0;
  let pageLifecycleGeneration = 0;
  let pageLive = false;
  let inspectedRunId: string | null | undefined = undefined;
  let projectionUnsubscribe: (() => void) | null = null;
  let artifactAccessRequests: Record<string, ArtifactAccessRequest> = {};
  const downloadUrls = createIoArtifactDownloadUrlOwner({
    delayMs: DOWNLOAD_OBJECT_URL_REVOKE_DELAY_MS,
    revoke: revokeObjectUrl,
    schedule: (callback, delayMs) => window.setTimeout(callback, delayMs),
    cancel: (handle) => window.clearTimeout(handle),
  });

  let tree = $derived(
    buildIoInspectorNodeTree({
      graphNodes: runGraph?.graph.nodes ?? [],
      nodeStatuses: runNodeStatuses,
      resolvedNodeIo,
      artifacts,
    }),
  );
  let visibleTree = $derived(filterIoInspectorNodeTree(tree, { direction, search }));
  let directionCounts = $derived(
    countIoInspectorEntriesByDirection(filterIoInspectorNodeTree(tree, { direction: 'all', search })),
  );
  let directionOptions = $derived<SegmentOption<IoInspectorDirectionFilter>[]>([
    { value: 'all', label: 'All', count: directionCounts.all },
    { value: 'input', label: 'Inputs', count: directionCounts.input },
    { value: 'output', label: 'Outputs', count: directionCounts.output },
  ]);
  let effectiveEntryKey = $derived(resolveIoInspectorSelectedEntryKey(selectedEntryKey, tree));
  let selectedEntry = $derived(findIoInspectorEntry(tree, effectiveEntryKey));
  let graphDiagnostics = $derived(partitionGraphDiagnosticsByNode(runGraph?.graph_diagnostics ?? []));
  let graphFactCards = $derived(buildStaleGraphFactCards(graphDiagnostics.graph));
  let appliedBackendFilter = $derived(selectedBackendFilter.trim());
  let dimensionLabel = $derived(
    $activeWorkflowRun
      ? buildIoInspectorDimensionLabel({
          runId: $activeWorkflowRun.workflow_run_id,
          nodeCount: tree.filter((node) => !node.boundary).length,
          artifacts,
          freshness: liveUpdatesError
            ? `${formatProjectionFreshness(projectionState)}, live updates off`
            : formatProjectionFreshness(projectionState),
        })
      : 'no run selected',
  );
  let savedGraphModel = $derived(
    savedGraphInspection
      ? buildSavedGraphInspectionDisplayModel(savedGraphInspection, savedSelectedNodeId)
      : null,
  );

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  function recordSubscriptionError(subscriptionError: unknown): void {
    if (!pageLive) {
      return;
    }
    liveUpdatesError = formatWorkflowCommandError(subscriptionError);
  }

  function formatTimestamp(value: number): string {
    return new Date(value).toLocaleString();
  }

  async function refreshInspector(
    runId = activeRunId(),
    backendFilterValue = selectedBackendFilter.trim(),
  ): Promise<void> {
    const requestSerial = ++inspectorRequestSerial;
    invalidatePendingArtifactAccess();
    runInspectorError = null;
    loadingInspector = true;

    if (!runId) {
      clearRunInspectionState();
      loadingInspector = false;
      return;
    }

    try {
      const inspectionResponse = await workflowService.queryRunInspection({
        workflow_run_id: runId,
        artifact_limit: 250,
      });
      if (requestSerial !== inspectorRequestSerial) {
        return;
      }

      const filtered = applyIoInspectorBackendFilter(
        inspectionResponse.io_artifacts,
        inspectionResponse.resolved_node_io ?? [],
        backendFilterValue,
      );
      revokeMissingArtifactObjectUrls(filtered.artifacts);
      runGraph = inspectionResponse.run_graph ?? null;
      runNodeStatuses = inspectionResponse.node_statuses;
      artifacts = filtered.artifacts;
      resolvedNodeIo = filtered.resolvedNodeIo;
      retentionSummary = inspectionResponse.retention_summary;
      projectionState = inspectionResponse.io_projection_state;
    } catch (error) {
      if (requestSerial !== inspectorRequestSerial) {
        return;
      }
      runInspectorError = formatWorkflowCommandError(error);
      clearRunInspectionState();
    } finally {
      if (requestSerial === inspectorRequestSerial) {
        loadingInspector = false;
      }
    }
  }

  async function refreshSavedGraphInspectionMode(
    requestedPath = selectedSavedWorkflowPath,
  ): Promise<void> {
    const requestSerial = ++savedGraphRequestSerial;
    savedGraphError = null;
    loadingSavedGraph = true;

    try {
      const workflows = await workflowService.listWorkflows();
      if (requestSerial !== savedGraphRequestSerial) {
        return;
      }

      const options = buildSavedGraphInspectionOptions(workflows);
      const nextPath = resolveSavedWorkflowInspectionPath(requestedPath, options);
      savedWorkflows = workflows;
      savedWorkflowOptions = options;
      selectedSavedWorkflowPath = nextPath;

      if (!nextPath) {
        savedGraphInspection = null;
        savedSelectedNodeId = null;
        return;
      }

      const projection = await workflowService.inspectWorkflowGraph({
        path: nextPath,
        selected_node_id: savedSelectedNodeId,
      });
      if (requestSerial !== savedGraphRequestSerial) {
        return;
      }
      savedGraphInspection = projection;
      savedSelectedNodeId = resolveSavedGraphSelectedNodeId(savedSelectedNodeId, projection);
    } catch (error) {
      if (requestSerial !== savedGraphRequestSerial) {
        return;
      }
      savedGraphError = formatWorkflowCommandError(error);
      savedGraphInspection = null;
      savedSelectedNodeId = null;
    } finally {
      if (requestSerial === savedGraphRequestSerial) {
        loadingSavedGraph = false;
      }
    }
  }

  /** Supersede any saved-graph load in flight (a run was picked or the page closed). */
  function supersedeSavedGraphInspection(): void {
    savedGraphRequestSerial += 1;
    loadingSavedGraph = false;
  }

  function clearRunInspectionState(): void {
    runGraph = null;
    runNodeStatuses = [];
    artifacts = [];
    resolvedNodeIo = [];
    retentionSummary = [];
    projectionState = null;
    revokeMissingArtifactObjectUrls([]);
  }

  function beginArtifactAccessRequest(
    artifact: IoArtifactProjectionRecord,
    payloadArtifactId: string,
  ): ArtifactAccessRequest | null {
    if (!pageLive || !isCurrentArtifactSelection(artifact)) {
      return null;
    }

    const accessRequest: ArtifactAccessRequest = {
      artifactId: artifact.artifact_id,
      payloadArtifactId,
      requestSerial: ++artifactAccessRequestSerial,
      lifecycleGeneration: pageLifecycleGeneration,
    };
    artifactAccessRequests = {
      ...artifactAccessRequests,
      [artifact.artifact_id]: accessRequest,
    };
    setArtifactLoading(artifact.artifact_id, true);
    setArtifactAccessError(artifact.artifact_id, null);
    return accessRequest;
  }

  function isCurrentArtifactAccessRequest(
    accessRequest: ArtifactAccessRequest,
    artifact: IoArtifactProjectionRecord,
  ): boolean {
    return (
      pageLive &&
      accessRequest.lifecycleGeneration === pageLifecycleGeneration &&
      artifactAccessRequests[accessRequest.artifactId]?.requestSerial === accessRequest.requestSerial &&
      accessRequest.payloadArtifactId === ioArtifactPayloadTargetId(artifact) &&
      isCurrentArtifactSelection(artifact)
    );
  }

  function finishArtifactAccessRequest(
    accessRequest: ArtifactAccessRequest,
    artifact: IoArtifactProjectionRecord,
  ): void {
    if (!isCurrentArtifactAccessRequest(accessRequest, artifact)) {
      return;
    }
    artifactAccessRequests = withoutArtifactKey(artifactAccessRequests, accessRequest.artifactId);
    setArtifactLoading(accessRequest.artifactId, false);
  }

  function invalidatePendingArtifactAccess(clearPresentationState = true): void {
    artifactAccessRequestSerial += 1;
    artifactAccessRequests = {};
    if (clearPresentationState) {
      artifactAccessLoading = {};
      artifactAccessErrors = {};
    }
  }

  function isCurrentArtifactSelection(artifact: IoArtifactProjectionRecord): boolean {
    const currentArtifact = artifacts.find((candidate) => candidate.artifact_id === artifact.artifact_id);
    return Boolean(
      currentArtifact &&
        ioArtifactPayloadTargetId(currentArtifact) === ioArtifactPayloadTargetId(artifact) &&
        selectedEntry?.artifact.artifact_id === artifact.artifact_id,
    );
  }

  async function readArtifactPreview(artifact: IoArtifactProjectionRecord): Promise<void> {
    const payloadArtifactId = ioArtifactPayloadTargetId(artifact);
    const accessRequest = beginArtifactAccessRequest(artifact, payloadArtifactId);
    if (!accessRequest) {
      return;
    }

    try {
      await verifyArtifactReadable(artifact);
      if (!isCurrentArtifactAccessRequest(accessRequest, artifact)) {
        return;
      }
      const read = await workflowService.readArtifactBody(
        buildIoArtifactPreviewReadRequest({
          artifact_id: payloadArtifactId,
          media_type: artifact.media_type,
          payload_kind: artifact.payload_kind,
          format: artifact.format,
        }),
      );
      if (!isCurrentArtifactAccessRequest(accessRequest, artifact)) {
        return;
      }
      assertCompleteIoArtifactImageRead(artifact, read.response);
      const preview = createArtifactBodyPreview(read);
      replaceArtifactBodyPreview(artifact.artifact_id, preview);
      setArtifactAccessError(artifact.artifact_id, null);
    } catch (error) {
      if (isCurrentArtifactAccessRequest(accessRequest, artifact)) {
        setArtifactAccessError(artifact.artifact_id, formatWorkflowCommandError(error));
      }
    } finally {
      finishArtifactAccessRequest(accessRequest, artifact);
    }
  }

  async function readArtifactStreamPreview(artifact: IoArtifactProjectionRecord): Promise<void> {
    const payloadArtifactId = ioArtifactPayloadTargetId(artifact);
    const accessRequest = beginArtifactAccessRequest(artifact, payloadArtifactId);
    if (!accessRequest) {
      return;
    }

    try {
      await verifyArtifactStreamReadable(artifact);
      if (!isCurrentArtifactAccessRequest(accessRequest, artifact)) {
        return;
      }
      const read = await workflowService.readArtifactStream(
        buildIoArtifactPreviewReadRequest(payloadArtifactId),
      );
      if (!isCurrentArtifactAccessRequest(accessRequest, artifact)) {
        return;
      }
      assertCompleteIoArtifactImageRead(artifact, read.response);
      const preview = createArtifactBodyPreview(read);
      replaceArtifactBodyPreview(artifact.artifact_id, preview);
      setArtifactAccessError(artifact.artifact_id, null);
    } catch (error) {
      if (isCurrentArtifactAccessRequest(accessRequest, artifact)) {
        setArtifactAccessError(artifact.artifact_id, formatWorkflowCommandError(error));
      }
    } finally {
      finishArtifactAccessRequest(accessRequest, artifact);
    }
  }

  async function downloadArtifactBody(artifact: IoArtifactProjectionRecord): Promise<void> {
    if (!pageLive) return;
    // Fence every await: a body that arrives after the page is gone never creates a URL or state.
    const lifecycleGeneration = pageLifecycleGeneration;
    const isCurrent = () => pageLive && pageLifecycleGeneration === lifecycleGeneration;
    const payloadArtifactId = ioArtifactPayloadTargetId(artifact);
    setArtifactLoading(artifact.artifact_id, true);
    setArtifactAccessError(artifact.artifact_id, null);
    try {
      await verifyArtifactReadable(artifact);
      if (!isCurrent()) return;
      const read = await workflowService.readArtifactBody({ artifact_id: payloadArtifactId });
      if (!isCurrent()) return;
      assertCompleteIoArtifactImageRead(artifact, read.response);
      const preview = createArtifactBodyPreview(read);
      await startBrowserDownload(
        preview.objectUrl,
        buildIoArtifactDownloadFilename({
          artifact_id: payloadArtifactId,
          media_type: read.response.media_type || artifact.media_type,
          format: artifact.format,
          payload_kind: artifact.payload_kind,
        }),
      );
    } catch (error) {
      if (isCurrent()) setArtifactAccessError(artifact.artifact_id, formatWorkflowCommandError(error));
    } finally {
      if (isCurrent()) setArtifactLoading(artifact.artifact_id, false);
    }
  }

  /**
   * Hand an object URL to the browser as a download through the page's own
   * declarative anchor, then let the download URL owner revoke it after the
   * delay (or at once when the page is gone).
   */
  async function startBrowserDownload(objectUrl: string, filename: string): Promise<void> {
    if (!pageLive) {
      downloadUrls.release(objectUrl);
      return;
    }
    downloadLink = { url: objectUrl, filename };
    await tick();
    downloadAnchor?.click();
    downloadLink = null;
    downloadUrls.release(objectUrl);
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

  function handleArtifactImageError(artifactId: string, failedObjectUrl: string): void {
    if (!pageLive || !isCurrentArtifactSelectionById(artifactId)) {
      return;
    }
    const currentPreview = artifactBodyPreviews[artifactId];
    if (!currentPreview || currentPreview.objectUrl !== failedObjectUrl) {
      return;
    }
    revokeObjectUrl(failedObjectUrl);
    artifactBodyPreviews = withoutArtifactKey(artifactBodyPreviews, artifactId);
    setArtifactAccessError(artifactId, IMAGE_PREVIEW_DECODE_ERROR);
  }

  function isCurrentArtifactSelectionById(artifactId: string): boolean {
    return selectedEntry?.artifact.artifact_id === artifactId;
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

  // Reload whenever the shared run or the Backend filter changes. The work
  // itself is untracked so state read while refreshing never re-runs this.
  $effect(() => {
    const runId = activeRunId();
    const backendFilterValue = selectedBackendFilter.trim();
    untrack(() => {
      if (runId !== inspectedRunId) {
        inspectedRunId = runId;
        selectedEntryKey = null;
        collapsedNodes = {};
      }
      if (runId) {
        supersedeSavedGraphInspection();
        void refreshInspector(runId, backendFilterValue);
      } else {
        void refreshInspector(null);
        if (savedInspectionOpen) {
          void refreshSavedGraphInspectionMode();
        }
      }
    });
  });

  onMount(() => {
    pageLive = true;
    pageLifecycleGeneration += 1;
    let disposed = false;
    void subscribeDiagnosticsProjectionInvalidations({
      projections: ['run_detail', 'node_status', 'io_artifact'],
      getActiveRunId: activeRunId,
      refresh: () => refreshInspector(),
      onRefreshError: recordSubscriptionError,
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        liveUpdatesError = null;
        projectionUnsubscribe = nextUnlisten;
      })
      .catch(recordSubscriptionError);

    return () => {
      disposed = true;
      pageLive = false;
      pageLifecycleGeneration += 1;
      invalidatePendingArtifactAccess(false);
      inspectorRequestSerial += 1;
      savedGraphRequestSerial += 1;
      projectionUnsubscribe?.();
      projectionUnsubscribe = null;
    };
  });

  onDestroy(() => {
    downloadUrls.dispose();
    revokeAllArtifactObjectUrls();
  });

  function selectEntry(entry: IoInspectorTreeEntry): void {
    if (entry.key === effectiveEntryKey) {
      return;
    }
    selectedEntryKey = entry.key;
    invalidatePendingArtifactAccess();
  }

  function toggleNode(nodeId: string): void {
    collapsedNodes = { ...collapsedNodes, [nodeId]: !collapsedNodes[nodeId] };
  }

  function toggleSavedInspection(): void {
    savedInspectionOpen = !savedInspectionOpen;
    if (savedInspectionOpen && !activeRunId()) {
      void refreshSavedGraphInspectionMode();
    }
  }

  function readHintFor(entry: IoInspectorTreeEntry): string {
    const family = buildIoArtifactRendererSummary(entry.artifact).family;
    if (canReadIoArtifactBody(entry.artifact)) {
      return family === 'image'
        ? 'Read body loads the full image.'
        : `Read body shows the first ${PREVIEW_LIMIT_LABEL}.`;
    }
    if (entry.artifact.stream_handle) {
      return 'Read stream shows what has been written so far.';
    }
    return '';
  }
</script>

<div class="pg-main h-full io" data-testid="io-inspector-page">
  {#if !$activeWorkflowRun}
    <DimensionLine label={dimensionLabel} />
    <div class="norun" class:norun-open={savedInspectionOpen}>
      <div class="empty-slot">
        <EmptyState
          label="No run selected"
          question="Whose inputs and outputs?"
          sentence="The inspector shows what passed through each port of one run. Pick a run here, select it in the Scheduler, or paste a run id."
          actionLabel="Pick a run"
          onaction={() => openRunContextPicker()}
        />
      </div>
      <section class="pg-panel saved" class:saved-open={savedInspectionOpen} aria-label="Saved workflow inspection">
        <button
          type="button"
          class="saved-toggle"
          aria-expanded={savedInspectionOpen}
          aria-controls="io-saved-body"
          onclick={toggleSavedInspection}
        >
          <span class="chev" aria-hidden="true">{savedInspectionOpen ? '▾' : '▸'}</span>
          <span class="saved-toggle-label">Inspect a saved workflow instead</span>
          <span class="pg-small pg-muted">A saved graph and the engine's stale facts for it, without a run.</span>
        </button>
        {#if savedInspectionOpen}
          <div id="io-saved-body" class="saved-body">
            <div class="saved-bar">
              <label class="pg-fl saved-select">
                Saved workflow
                <select
                  class="pg-field"
                  value={selectedSavedWorkflowPath ?? ''}
                  disabled={loadingSavedGraph || savedWorkflowOptions.length === 0}
                  onchange={(event) => {
                    const nextPath = (event.currentTarget as HTMLSelectElement).value || null;
                    selectedSavedWorkflowPath = nextPath;
                    savedSelectedNodeId = null;
                    void refreshSavedGraphInspectionMode(nextPath);
                  }}
                >
                  {#each savedWorkflowOptions as option (option.inspectionPath)}
                    <option value={option.inspectionPath}>{option.label}</option>
                  {/each}
                </select>
              </label>
              <span class="pg-small pg-muted">
                {savedWorkflowOptions.length}
                {savedWorkflowOptions.length === 1 ? 'saved workflow' : 'saved workflows'} with stable ids
              </span>
              <button
                type="button"
                class="pg-btn pg-btn-sm saved-refresh"
                disabled={loadingSavedGraph}
                onclick={() => void refreshSavedGraphInspectionMode()}
              >
                {loadingSavedGraph ? 'Loading' : 'Refresh'}
              </button>
            </div>
            <div class="saved-graph">
              {#if savedGraphError}
                <div class="pad">
                  <ErrorCard
                    title="Could not inspect the saved workflow"
                    explanation={savedGraphError}
                    suggestion="check that the engine is running, then refresh."
                    actionLabel="Refresh"
                    onaction={() => void refreshSavedGraphInspectionMode()}
                  />
                </div>
              {:else if loadingSavedGraph && !savedGraphModel}
                <div class="pg-empty">Loading the saved graph.</div>
              {:else if savedGraphModel}
                <SavedGraphInspectionSnapshot
                  model={savedGraphModel}
                  selectedNodeId={savedSelectedNodeId}
                  onSelectNode={(nodeId) => {
                    savedSelectedNodeId = nodeId;
                  }}
                />
              {:else if savedWorkflows.length === 0}
                <div class="pg-empty">No saved workflows are available for inspection.</div>
              {:else}
                <div class="pg-empty">Saved workflows need backend-listed stable ids before inspection.</div>
              {/if}
            </div>
          </div>
        {/if}
      </section>
    </div>
  {:else}
    <div class="dimwrap" data-run-id={$activeWorkflowRun.workflow_run_id} title="Ctrl+C copies the run id">
      <DimensionLine label={dimensionLabel} />
    </div>

    <div class="split">
      <section class="pg-panel pg-col tree-panel" aria-label="Nodes and ports">
        <div class="pg-ptop">
          <div class="toprow">
            <label class="pg-sr-only" for="io-search">Search nodes and ports</label>
            <input
              id="io-search"
              class="pg-field grow"
              type="search"
              placeholder="Search nodes and ports"
              bind:value={search}
            />
            <button
              type="button"
              class="pg-tgl"
              aria-expanded={filtersOpen}
              aria-controls="io-filters"
              onclick={() => (filtersOpen = !filtersOpen)}
            >
              Filters{#if appliedBackendFilter}<span class="pg-n">1</span>{/if}
            </button>
          </div>
          <SegmentedControl label="Show" options={directionOptions} value={direction} onchange={(next) => (direction = next)} />
          {#if filtersOpen}
            <div id="io-filters" class="filters">
              <label class="pg-fl">
                Backend
                <input
                  class="pg-field pg-mono"
                  type="text"
                  placeholder="All backends"
                  bind:value={selectedBackendFilter}
                />
              </label>
            </div>
          {/if}
          {#if appliedBackendFilter || retentionSummary.length > 0}
            <div class="chips" aria-label="Applied filters and retention">
              {#if appliedBackendFilter}
                <FilterChip label={`Backend ${appliedBackendFilter}`} onclear={() => (selectedBackendFilter = '')} />
              {/if}
              {#each retentionSummary as item (item.retention_state)}
                <Chip>{formatIoArtifactRetentionStateLabel(item.retention_state)}<span class="pg-n">{item.artifact_count}</span></Chip>
              {/each}
            </div>
          {/if}
        </div>

        <div class="pg-scroll">
          {#if runInspectorError}
            <div class="pad">
              <ErrorCard
                title="Could not load this run's inputs and outputs"
                explanation={runInspectorError}
                suggestion="check that the engine is running, then refresh."
                actionLabel="Refresh"
                onaction={() => void refreshInspector()}
              />
            </div>
          {/if}
          {#each graphFactCards as card (card.key)}
            <div class="pad fact-pad">
              <div class="fact">
                <div><b class="fact-title">{card.title}</b> <span class="pg-small pg-muted">{card.code}</span></div>
                <div class="pg-small fact-text">{card.explanation}</div>
                {#if card.suggestion}<div class="pg-small pg-muted fact-hint">{card.suggestion}</div>{/if}
              </div>
            </div>
          {/each}

          {#if loadingInspector && tree.length === 0}
            <div class="pg-empty">Loading this run's nodes and ports.</div>
          {:else if tree.length === 0 && !runInspectorError}
            <div class="pg-empty">No graph was captured for this run and nothing has been retained yet.</div>
          {:else if visibleTree.length === 0 && tree.length > 0}
            <div class="pg-empty">No node or port matches “{search.trim()}”.</div>
          {:else}
            <ul class="tree">
              {#each visibleTree as node, index (node.nodeId)}
                {@const open = !collapsedNodes[node.nodeId]}
                {@const nodeFacts = buildStaleGraphFactCards(graphDiagnostics.byNodeId.get(node.nodeId) ?? [])}
                <li>
                  <button
                    type="button"
                    class="pg-item node"
                    aria-expanded={open}
                    aria-controls={`io-node-${index}`}
                    onclick={() => toggleNode(node.nodeId)}
                  >
                    <StatusGlyph state={node.boundary ? 'idle' : glyphStateForStatus(node.status)} />
                    <span class="min-w-0">
                      <span class="pg-name">{formatIoInspectorNodeLabel(node)}</span>{#if node.nodeType}<span class="pg-ver">{node.nodeType}</span>{/if}
                      <span class="pg-sub">{formatIoInspectorNodeSubLabel(node, nodeFacts.length)}</span>
                    </span>
                    <span class="pg-time" aria-hidden="true">{open ? '▾' : '▸'}</span>
                  </button>
                  {#if open}
                    <div id={`io-node-${index}`}>
                      {#each node.entries as entry (entry.key)}
                        <button
                          type="button"
                          class="art"
                          aria-current={entry.key === effectiveEntryKey ? 'true' : undefined}
                          data-testid="io-artifact-row"
                          data-artifact-id={entry.artifact.artifact_id}
                          onclick={() => selectEntry(entry)}
                        >
                          <span class="p">
                            <span aria-hidden="true">{formatIoInspectorDirectionMark(entry.direction)}</span>
                            <span class="pg-sr-only">{formatResolvedNodeIoDirectionLabel(entry.direction)}</span>
                            {entry.portId}
                          </span>
                          <span class="s">{formatIoArtifactBytes(entry.artifact.size_bytes)}</span>
                          <span class="d">{formatIoInspectorEntryDetail(entry.artifact)}</span>
                        </button>
                      {/each}
                      {#each nodeFacts as card (card.key)}
                        <div class="node-fact">
                          <div class="fact">
                            <div><b class="fact-title">{card.title}</b></div>
                            <div class="pg-small pg-muted">{card.code}</div>
                            <div class="pg-small fact-text">{card.explanation}</div>
                            {#if card.suggestion}<div class="pg-small pg-muted fact-hint">{card.suggestion}</div>{/if}
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <div class="pg-pfoot">
          <button
            type="button"
            data-testid="io-inspector-refresh"
            class="pg-btn pg-btn-sm"
            disabled={loadingInspector}
            title={liveUpdatesError ? `Live updates are off: ${liveUpdatesError}` : 'Query the run inspection projection again'}
            onclick={() => void refreshInspector()}
          >
            {loadingInspector ? 'Loading' : 'Refresh'}
          </button>
          <span class="grow"></span>
          <button type="button" class="pg-btn pg-btn-sm" onclick={() => focusSettingsSection('diagnostics_retention')}>
            Retention settings
          </button>
        </div>
      </section>

      <section class="pg-panel pg-col detail-panel" aria-label="Artifact">
        <div class="pg-scroll">
          {#if selectedEntry}
            {@const entry = selectedEntry}
            {@const artifact = entry.artifact}
            {@const renderer = buildIoArtifactRendererSummary(artifact)}
            {@const bodyPreview = artifactBodyPreviews[artifact.artifact_id]}
            {@const previewMode = resolveIoArtifactPreviewMode(renderer.family, bodyPreview)}
            {@const sections = buildIoArtifactDetailSections(entry, formatTimestamp)}
            {@const readBlocked = describeIoArtifactReadUnavailable(artifact)}
            {@const streamBlocked = describeIoArtifactStreamUnavailable(artifact)}
            {@const ackBlocked = describeIoArtifactAcknowledgeUnavailable(artifact)}
            {@const accessBusy = Boolean(artifactAccessLoading[artifact.artifact_id])}
            {@const nodeLabel = formatIoInspectorNodeLabel({ nodeId: entry.nodeId, boundary: entry.nodeId === IO_INSPECTOR_BOUNDARY_NODE_ID })}
            <div
              data-testid="io-artifact-card"
              data-artifact-id={artifact.artifact_id}
              data-artifact-media-family={renderer.family}
            >
              <div class="head">
                <div class="head-text">
                  <h2 class="pg-title art-title">{nodeLabel}<span class="pg-muted colon">:</span>{entry.portId}</h2>
                  <div class="pg-mono pg-muted art-id" title={artifact.artifact_id}>{artifact.artifact_id}</div>
                  <div class="chips">
                    {#each buildIoArtifactChipLabels(artifact) as label (label)}
                      <Chip>{label}</Chip>
                    {/each}
                  </div>
                </div>
                <div class="actions">
                  <button
                    type="button"
                    class="pg-btn"
                    data-testid="io-artifact-read-button"
                    disabled={!canReadIoArtifactBody(artifact) || accessBusy}
                    title={readBlocked ?? `Read the body (images in full, other media up to ${PREVIEW_LIMIT_LABEL})`}
                    onclick={() => void readArtifactPreview(artifact)}
                  >
                    {bodyPreview ? 'Refresh read' : 'Read body'}
                  </button>
                  <button
                    type="button"
                    class="pg-btn"
                    data-testid="io-artifact-stream-button"
                    disabled={!artifact.stream_handle || accessBusy}
                    title={streamBlocked ?? `Read up to ${PREVIEW_LIMIT_LABEL} of what the stream has written`}
                    onclick={() => void readArtifactStreamPreview(artifact)}
                  >
                    {bodyPreview && artifact.stream_handle ? 'Refresh stream' : 'Read stream'}
                  </button>
                  <button
                    type="button"
                    class="pg-btn"
                    data-testid="io-artifact-download-button"
                    disabled={!canReadIoArtifactBody(artifact) || accessBusy}
                    title={readBlocked ?? 'Download the full body'}
                    onclick={() => void downloadArtifactBody(artifact)}
                  >
                    Download
                  </button>
                  <button
                    type="button"
                    class="pg-btn"
                    data-testid="io-artifact-acknowledge-button"
                    disabled={!canAcknowledgeIoArtifactConsumed(artifact) || artifactConsumeLoading[artifact.artifact_id]}
                    title={ackBlocked ?? 'Tell the engine this GUI has consumed the payload'}
                    onclick={() => void acknowledgeArtifactConsumed(artifact)}
                  >
                    {artifactConsumeLoading[artifact.artifact_id] ? 'Acknowledging' : 'Acknowledge'}
                  </button>
                </div>
              </div>

              {#if artifactAccessErrors[artifact.artifact_id]}
                <div class="pad-x" data-testid="io-artifact-access-error">
                  <ErrorCard
                    title="Could not use this artifact's body"
                    explanation={artifactAccessErrors[artifact.artifact_id]}
                    suggestion="refresh the inspector; if the payload has expired, only its metadata remains."
                    actionLabel="Refresh"
                    onaction={() => void refreshInspector()}
                  />
                </div>
              {/if}
              {#if artifactConsumeMessages[artifact.artifact_id]}
                <div class="pad-x pg-small pg-muted" role="status">{artifactConsumeMessages[artifact.artifact_id]}</div>
              {/if}

              <div class="preview-wrap">
                {#if bodyPreview && previewMode === 'image'}
                  <div class="prev">
                    {#key bodyPreview.objectUrl}
                      <img
                        src={bodyPreview.objectUrl}
                        alt={`Preview of ${artifact.artifact_id}`}
                        class="prev-img"
                        data-testid="io-artifact-image-preview"
                        data-preview-complete={bodyPreview.complete}
                        data-preview-byte-length={bodyPreview.byteLength}
                        onerror={(event) =>
                          handleArtifactImageError(artifact.artifact_id, (event.currentTarget as HTMLImageElement).src)}
                      />
                    {/key}
                    <span class="cap">{formatIoArtifactPreviewExtent(bodyPreview)}, full body loaded</span>
                  </div>
                {:else if bodyPreview && previewMode === 'audio'}
                  <div class="prev prev-pad">
                    <audio src={bodyPreview.objectUrl} controls class="w-full" aria-label={`Audio preview of ${artifact.artifact_id}`}></audio>
                  </div>
                  <div class="pg-small pg-muted note">{formatIoArtifactPreviewExtent(bodyPreview)}</div>
                {:else if bodyPreview && previewMode === 'text'}
                  <pre class="pg-code text-prev">{bodyPreview.text}</pre>
                  <div class="pg-small pg-muted note">
                    {formatIoArtifactPreviewExtent(bodyPreview)}{#if bodyPreview.textTruncated} · text truncated{/if}
                  </div>
                {:else if bodyPreview}
                  <div class="prev prev-meta">
                    <div>Binary preview loaded. Use Download to inspect the retained body outside Pantograph.</div>
                    <div class="pg-small pg-muted">{formatIoArtifactPreviewExtent(bodyPreview)}</div>
                  </div>
                {:else if readBlocked && streamBlocked}
                  <div class="prev prev-meta">
                    <div>Only metadata was kept for this port.</div>
                    <div class="pg-small pg-muted">{artifact.retention_reason?.trim() || readBlocked}</div>
                  </div>
                {:else}
                  <div class="prev prev-meta">
                    <div>{renderer.title}, not read yet.</div>
                    <div class="pg-small pg-muted">{readHintFor(entry)}</div>
                  </div>
                {/if}
              </div>

              <div class="secs">
                <div class="pg-sec sec-a">
                  <h3>Where it came from</h3>
                  <KeyValue rows={sections.origin} />
                </div>
                <div class="pg-sec sec-b">
                  <h3>How it was made</h3>
                  <KeyValue rows={sections.making} />
                </div>
              </div>
              <div class="pg-sec">
                <h3>Storage</h3>
                <KeyValue rows={sections.storage} />
              </div>
            </div>
          {:else}
            <div class="pg-empty">
              {tree.some((node) => node.entries.length > 0)
                ? 'Pick a port on the left to see what passed through it.'
                : 'Nothing has been retained for this run yet. Ports appear here as nodes finish.'}
            </div>
          {/if}
        </div>
      </section>
    </div>
  {/if}

  {#if downloadLink}
    <a class="pg-sr-only" href={downloadLink.url} download={downloadLink.filename} rel="noopener noreferrer" tabindex="-1" aria-hidden="true" bind:this={downloadAnchor}>Download artifact body</a>
  {/if}
</div>

<style>
  .io {
    overflow: hidden;
  }

  /* The question and the saved-workflow disclosure read as one group: centred
     while the disclosure is closed, the saved graph takes the space when open. */
  .norun {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 10px;
  }

  .empty-slot {
    display: flex;
    flex: 0 0 260px;
    min-height: 0;
  }

  .norun-open .empty-slot {
    flex-basis: 200px;
  }

  .saved {
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .saved-open {
    flex: 2 1 0;
  }

  .saved-toggle {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    padding: 10px 14px;
    border: 0;
    background: var(--pg-panel-head);
    color: var(--pg-text);
    text-align: left;
    cursor: pointer;
    flex: none;
  }

  .saved-toggle:hover {
    background: var(--pg-panel-raised);
  }

  .saved-toggle-label {
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .chev {
    font-size: 10px;
    color: var(--pg-border-strong);
  }

  .saved-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--pg-border);
  }

  .saved-bar {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
    padding: 10px 14px;
    border-bottom: 1px solid var(--pg-border);
    flex: none;
  }

  .saved-select {
    flex: 0 1 360px;
    min-width: 220px;
  }

  .saved-refresh {
    margin-left: auto;
  }

  .saved-graph {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .dimwrap {
    flex: none;
  }

  .split {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .tree-panel {
    flex: 0 0 340px;
    min-height: 0;
  }

  .detail-panel {
    flex: 1 1 0%;
    min-height: 0;
  }

  .toprow {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .grow {
    flex: 1;
    min-width: 0;
  }

  .filters {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .pad {
    padding: 10px 12px;
  }

  .fact-pad + .fact-pad {
    padding-top: 0;
  }

  .pad-x {
    padding: 0 18px 12px;
  }

  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .node {
    align-items: center;
  }

  .node .pg-name {
    font-family: var(--pg-font-mono);
    font-size: 12.5px;
  }

  /* Port rows under a node (board `.art`). */
  .art {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px 10px;
    width: 100%;
    padding: 8px 12px 8px 46px;
    border: 0;
    border-top: 1px solid var(--pg-border-soft);
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: var(--pg-text);
    font-size: 13px;
  }

  .art:hover {
    background: var(--pg-panel-raised);
  }

  .art[aria-current='true'] {
    background: var(--pg-panel-selected);
    box-shadow: inset 3px 0 0 var(--pg-text);
  }

  .art .p {
    font-family: var(--pg-font-mono);
    font-size: 12px;
    color: var(--pg-text-bright);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .art .s {
    font-family: var(--pg-font-mono);
    font-size: 11px;
    color: var(--pg-text-muted);
    text-align: right;
    white-space: nowrap;
  }

  .art .d {
    grid-column: 1 / -1;
    font-size: 12px;
    color: var(--pg-text-muted);
  }

  .node-fact {
    padding: 6px 12px 10px 46px;
    border-top: 1px solid var(--pg-border-soft);
  }

  /* ErrorCard shape, muted: stale graph facts describe the captured graph. */
  .fact {
    padding: 8px 10px;
    border: 1px solid var(--pg-border);
    border-left: 3px solid var(--pg-border-strong);
    border-radius: 4px;
    background: var(--pg-panel-head);
  }

  .fact-title {
    color: var(--pg-text-bright);
  }

  .fact-text {
    margin-top: 2px;
  }

  .fact-hint {
    margin-top: 4px;
  }

  .head {
    padding: 16px 18px 12px;
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .head-text {
    flex: 1 1 260px;
    min-width: 0;
  }

  .art-title {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .colon {
    font-size: 18px;
  }

  .art-id {
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .head .chips {
    margin-top: 10px;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .preview-wrap {
    padding: 0 18px 16px;
  }

  .prev {
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background: var(--pg-panel-sunken);
    min-height: 220px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    position: relative;
  }

  .prev-img {
    display: block;
    max-width: 100%;
    max-height: 360px;
    object-fit: contain;
  }

  .prev-pad {
    min-height: 0;
    padding: 16px;
  }

  .prev-meta {
    min-height: 120px;
    flex-direction: column;
    gap: 6px;
    padding: 20px;
    color: var(--pg-text-muted);
    text-align: center;
  }

  .cap {
    position: absolute;
    left: 10px;
    bottom: 8px;
    font-family: var(--pg-font-mono);
    font-size: 11px;
    color: var(--pg-text-soft);
    background: var(--pg-panel-sunken);
    padding: 2px 6px;
    border-radius: 3px;
  }

  .text-prev {
    margin: 0;
    max-height: 260px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .note {
    margin-top: 6px;
  }

  .secs {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(260px, 100%), 1fr));
    border-top: 1px solid var(--pg-border);
  }

  .secs .pg-sec {
    border-top: 0;
  }

  .sec-b {
    border-left: 1px solid var(--pg-border);
  }
</style>
