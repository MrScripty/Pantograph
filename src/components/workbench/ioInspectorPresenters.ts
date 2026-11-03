import type {
  IoArtifactAccessMode,
  IoArtifactConversionStatus,
  DiagnosticsRetentionPolicy,
  IoArtifactLifecycleState,
  IoArtifactPayloadKind,
  IoArtifactProjectionRecord,
  IoArtifactRetentionState,
  NodeExecutionProjectionStatus,
  NodeStatusProjectionRecord,
  ProjectionStateRecord,
  ResolvedNodeIoDirection,
  ResolvedNodeIoRecord,
  WorkflowRetentionCleanupResult,
} from '../../services/diagnostics/types';
import type { GraphNode, WorkflowGraphDiagnostic } from '../../services/workflow/types';
import type { KeyValueRow } from './ui/types';
import { formatProjectionFreshnessState } from './projectionFreshness.ts';

export type IoArtifactMediaFamily =
  | 'text'
  | 'image'
  | 'audio'
  | 'video'
  | '3d'
  | 'table'
  | 'json'
  | 'file'
  | 'unknown';

export type IoArtifactPayloadAvailability =
  | 'referenced'
  | 'metadata_only';

export interface IoArtifactNodeGroup {
  node_id: string;
  node_type?: string | null;
  input_count: number;
  output_count: number;
  artifact_count: number;
  latest_event_seq: number;
}

export interface IoArtifactRendererSummary {
  family: IoArtifactMediaFamily;
  title: string;
  detail: string;
}

export interface IoArtifactDescriptorMetadataRow {
  label: string;
  value: string;
  mono: boolean;
}

type ResolvedNodeIoArtifactIdentity = Pick<
  IoArtifactProjectionRecord,
  'artifact_id' | 'artifact_fact_id' | 'payload_artifact_id'
>;

export interface ResolvedNodeIoDisplayRow<Artifact extends ResolvedNodeIoArtifactIdentity = IoArtifactProjectionRecord> {
  nodeIo: ResolvedNodeIoRecord;
  artifact: Artifact | null;
}

export interface IoRetentionDetailRow {
  label: string;
  value: string;
  mono: boolean;
}

export const IO_ARTIFACT_PREVIEW_MAX_BYTES = 64 * 1024;

export interface IoArtifactPreviewReadRequest {
  artifact_id: string;
  byte_range_start?: number;
  byte_range_end_exclusive?: number;
}

export type IoArtifactPreviewReadTarget =
  | string
  | (Pick<IoArtifactProjectionRecord, 'artifact_id'> &
      Partial<Pick<IoArtifactProjectionRecord, 'media_type' | 'payload_kind' | 'format'>>);

export interface IoArtifactPreviewExtent {
  mediaType: string;
  byteLength: number;
  complete: boolean;
}

export function assertCompleteIoArtifactImageRead(
  artifact: Pick<IoArtifactProjectionRecord, 'media_type' | 'payload_kind'> &
    Partial<Pick<IoArtifactProjectionRecord, 'format'>>,
  response: { media_type: string; complete: boolean },
): void {
  const artifactFamily = classifyIoArtifactMedia(resolveIoArtifactMediaType(artifact), artifact.payload_kind);
  const responseFamily = classifyIoArtifactMedia(response.media_type);
  const artifactIsExplicitImage = artifact.payload_kind === 'image';
  if ((artifactIsExplicitImage || artifactFamily === 'image' || responseFamily === 'image') && !response.complete) {
    throw new Error('Image preview is incomplete; the complete image body is required.');
  }
}

export interface IoArtifactTextPreview {
  text: string;
  truncated: boolean;
}

export function ioArtifactPayloadTargetId(
  artifact: Pick<IoArtifactProjectionRecord, 'artifact_id' | 'payload_artifact_id'>,
): string {
  const payloadArtifactId = artifact.payload_artifact_id?.trim();
  return payloadArtifactId && payloadArtifactId.length > 0 ? payloadArtifactId : artifact.artifact_id;
}

export function buildResolvedNodeIoDisplayRows<Artifact extends ResolvedNodeIoArtifactIdentity>(
  records: ResolvedNodeIoRecord[],
  artifacts: Artifact[],
): ResolvedNodeIoDisplayRow<Artifact>[] {
  const artifactsByFactId = buildArtifactIdentityIndex(artifacts, 'artifact_fact_id');
  const artifactsByPayloadId = buildArtifactIdentityIndex(artifacts, 'payload_artifact_id');
  const artifactsByArtifactId = buildArtifactIdentityIndex(artifacts, 'artifact_id');

  return dedupeResolvedNodeIo(records).map((nodeIo) => ({
    nodeIo,
    artifact:
      findArtifactByIdentity(nodeIo.artifact_fact_id, artifactsByFactId) ??
      findArtifactByIdentity(nodeIo.payload_artifact_id, artifactsByPayloadId) ??
      findArtifactByIdentity(nodeIo.artifact_id, artifactsByArtifactId) ??
      null,
  }));
}

export function isNormalResolvedNodeIoDisplayRow(
  row: Pick<ResolvedNodeIoDisplayRow<ResolvedNodeIoArtifactIdentity & Partial<IoArtifactProjectionRecord>>, 'nodeIo' | 'artifact'>,
): boolean {
  return ![
    row.nodeIo.port_id,
    row.artifact?.producer_port_id,
    row.artifact?.consumer_port_id,
  ].some(isDiagnosticDataPort);
}

export function formatResolvedNodeIoDirectionLabel(
  direction: ResolvedNodeIoRecord['direction'],
): string {
  switch (direction) {
    case 'input':
      return 'Input';
    case 'output':
      return 'Output';
  }
}

export function formatResolvedNodeIoResolutionLabel(
  resolution: ResolvedNodeIoRecord['resolution'],
): string {
  switch (resolution) {
    case 'produced_output':
      return 'Produced output';
    case 'derived_from_edge':
      return 'Derived from edge';
    case 'explicit_input':
      return 'Explicit input';
    case 'workflow_boundary':
      return 'Workflow boundary';
  }
}

export function formatResolvedNodeIoProvenance(
  record: Pick<
    ResolvedNodeIoRecord,
    'direction' | 'resolution' | 'provenance_kind' | 'port_id' | 'upstream_node_id' | 'upstream_port_id'
  >,
): string {
  switch (record.provenance_kind ?? provenanceKindFromResolution(record.direction, record.resolution)) {
    case 'graph_edge':
      return record.upstream_node_id && record.upstream_port_id
        ? `From ${formatIoArtifactEndpointValue(record.upstream_node_id, record.upstream_port_id)}`
        : 'From graph edge';
    case 'workflow_input_boundary':
      return 'From workflow input boundary';
    case 'workflow_output_boundary':
      return 'To workflow output boundary';
    case 'cache_replay':
      return `Cache replay on ${record.port_id}`;
    case 'coercion':
      return `Coerced input on ${record.port_id}`;
    case 'redaction':
      return `Redacted input on ${record.port_id}`;
    case 'dynamic_route':
      return `Dynamic route on ${record.port_id}`;
    case 'fan_in_aggregation':
      return `Fan-in aggregation on ${record.port_id}`;
    case 'runtime_injected':
      return `Runtime injected on ${record.port_id}`;
    case 'explicit_input':
      return `Resolved input on ${record.port_id}`;
    case 'produced_output':
      return `Produced on ${record.port_id}`;
  }
}

function isDiagnosticDataPort(portId: string | null | undefined): boolean {
  return portId?.trim() === '_data';
}

function provenanceKindFromResolution(
  direction: ResolvedNodeIoRecord['direction'],
  resolution: ResolvedNodeIoRecord['resolution'],
): NonNullable<ResolvedNodeIoRecord['provenance_kind']> {
  switch (resolution) {
    case 'derived_from_edge':
      return 'graph_edge';
    case 'workflow_boundary':
      return direction === 'input' ? 'workflow_input_boundary' : 'workflow_output_boundary';
    case 'explicit_input':
      return 'explicit_input';
    case 'produced_output':
      return 'produced_output';
  }
}

function dedupeResolvedNodeIo(records: ResolvedNodeIoRecord[]): ResolvedNodeIoRecord[] {
  const recordsByLogicalPayload = new Map<string, ResolvedNodeIoRecord>();

  for (const record of records) {
    const key = resolvedNodeIoLogicalKey(record);
    const existing = recordsByLogicalPayload.get(key);
    if (!existing || resolvedNodeIoResolutionRank(record) < resolvedNodeIoResolutionRank(existing)) {
      recordsByLogicalPayload.set(key, record);
    }
  }

  return [...recordsByLogicalPayload.values()];
}

function resolvedNodeIoLogicalKey(record: ResolvedNodeIoRecord): string {
  const payloadIdentity =
    normalizeArtifactIdentity(record.payload_artifact_id) ??
    normalizeArtifactIdentity(record.artifact_id) ??
    normalizeArtifactIdentity(record.artifact_fact_id) ??
    record.resolution;
  return [record.direction, record.node_id, record.port_id, payloadIdentity].join('\u001f');
}

function resolvedNodeIoResolutionRank(record: ResolvedNodeIoRecord): number {
  if (record.direction === 'output') {
    switch (record.resolution) {
      case 'produced_output':
        return 0;
      case 'workflow_boundary':
        return 1;
      case 'explicit_input':
        return 2;
      case 'derived_from_edge':
        return 3;
    }
  }

  switch (record.resolution) {
    case 'derived_from_edge':
      return 0;
    case 'explicit_input':
      return 1;
    case 'workflow_boundary':
      return 2;
    case 'produced_output':
      return 3;
  }
}

function buildArtifactIdentityIndex<Artifact extends ResolvedNodeIoArtifactIdentity>(
  artifacts: Artifact[],
  identityField: 'artifact_fact_id' | 'payload_artifact_id' | 'artifact_id',
): Map<string, Artifact> {
  const index = new Map<string, Artifact>();
  for (const artifact of artifacts) {
    const identity = normalizeArtifactIdentity(artifact[identityField]);
    if (identity && !index.has(identity)) {
      index.set(identity, artifact);
    }
  }
  return index;
}

function findArtifactByIdentity<Artifact extends ResolvedNodeIoArtifactIdentity>(
  identity: string | null | undefined,
  index: Map<string, Artifact>,
): Artifact | null {
  const normalized = normalizeArtifactIdentity(identity);
  return normalized ? index.get(normalized) ?? null : null;
}

function normalizeArtifactIdentity(identity: string | null | undefined): string | null {
  const trimmed = identity?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : null;
}

type IoArtifactNodeGroupSource = Pick<
  IoArtifactProjectionRecord,
  | 'node_id'
  | 'node_type'
  | 'producer_node_id'
  | 'consumer_node_id'
  | 'artifact_role'
  | 'event_seq'
>;

export function isWorkflowInputArtifact(
  artifact: Pick<IoArtifactProjectionRecord, 'artifact_role'>,
): boolean {
  return artifact.artifact_role === 'workflow_input';
}

export function isWorkflowOutputArtifact(
  artifact: Pick<IoArtifactProjectionRecord, 'artifact_role'>,
): boolean {
  return artifact.artifact_role === 'workflow_output';
}

export function formatIoArtifactRoleLabel(role: string | null | undefined): string {
  switch (role) {
    case 'workflow_input':
      return 'Workflow input';
    case 'workflow_output':
      return 'Workflow output';
    case 'node_input':
      return 'Node input';
    case 'node_output':
      return 'Node output';
    default:
      return role && role.trim().length > 0 ? role : 'Unclassified';
  }
}

export function buildIoArtifactNodeGroups(artifacts: IoArtifactNodeGroupSource[]): IoArtifactNodeGroup[] {
  const groups = new Map<string, IoArtifactNodeGroup>();
  const ensureGroup = (nodeId: string, artifact: IoArtifactNodeGroupSource): IoArtifactNodeGroup => {
    const group = groups.get(nodeId) ?? {
      node_id: nodeId,
      node_type: nodeId === artifact.node_id ? artifact.node_type : null,
      input_count: 0,
      output_count: 0,
      artifact_count: 0,
      latest_event_seq: artifact.event_seq,
    };
    group.latest_event_seq = Math.max(group.latest_event_seq, artifact.event_seq);
    if (!group.node_type && nodeId === artifact.node_id && artifact.node_type) {
      group.node_type = artifact.node_type;
    }
    groups.set(nodeId, group);
    return group;
  };

  for (const artifact of artifacts) {
    const countedGroups = new Set<string>();
    if (artifact.consumer_node_id) {
      const group = ensureGroup(artifact.consumer_node_id, artifact);
      group.input_count += 1;
      if (!countedGroups.has(group.node_id)) {
        group.artifact_count += 1;
        countedGroups.add(group.node_id);
      }
    }

    if (artifact.producer_node_id) {
      const group = ensureGroup(artifact.producer_node_id, artifact);
      group.output_count += 1;
      if (!countedGroups.has(group.node_id)) {
        group.artifact_count += 1;
        countedGroups.add(group.node_id);
      }
    }

    if (!artifact.consumer_node_id && !artifact.producer_node_id && artifact.node_id) {
      const group = ensureGroup(artifact.node_id, artifact);
      group.artifact_count += 1;
      if (artifact.artifact_role === 'node_input') {
        group.input_count += 1;
      }
      if (artifact.artifact_role === 'node_output') {
        group.output_count += 1;
      }
    }
  }

  return [...groups.values()].sort(
    (left, right) => right.latest_event_seq - left.latest_event_seq || left.node_id.localeCompare(right.node_id),
  );
}

export function classifyIoArtifactMedia(
  mediaType: string | null | undefined,
  payloadKind?: IoArtifactPayloadKind | null,
): IoArtifactMediaFamily {
  if (!mediaType) {
    switch (payloadKind) {
      case 'text':
        return 'text';
      case 'image':
        return 'image';
      case 'audio':
        return 'audio';
      case 'video':
        return 'video';
      case '3d':
        return '3d';
      case 'large_table':
        return 'table';
      case 'structured':
        return 'json';
      case 'generic_binary':
        return 'file';
      default:
        return 'unknown';
    }
  }
  const normalized = mediaType.toLowerCase();
  if (normalized.includes('csv') || normalized.includes('parquet') || normalized.includes('table')) {
    return 'table';
  }
  if (normalized.includes('model/gltf') || normalized.includes('model/obj') || normalized.includes('model/')) {
    return '3d';
  }
  if (normalized.startsWith('text/')) {
    return 'text';
  }
  if (normalized.startsWith('image/')) {
    return 'image';
  }
  if (normalized.startsWith('audio/')) {
    return 'audio';
  }
  if (normalized.startsWith('video/')) {
    return 'video';
  }
  if (normalized.includes('json')) {
    return 'json';
  }
  return 'file';
}

export function formatIoArtifactMediaLabel(
  mediaType: string | null | undefined,
  payloadKind?: IoArtifactPayloadKind | null,
): string {
  switch (classifyIoArtifactMedia(mediaType, payloadKind)) {
    case 'text':
      return 'Text';
    case 'image':
      return 'Image';
    case 'audio':
      return 'Audio';
    case 'video':
      return 'Video';
    case '3d':
      return '3D';
    case 'table':
      return 'Table';
    case 'json':
      return 'JSON';
    case 'file':
      return 'File';
    case 'unknown':
      return 'Unknown';
  }
}

export function buildIoArtifactRendererSummary(
  artifact: Pick<IoArtifactProjectionRecord, 'media_type' | 'payload_ref'> &
    Partial<Pick<IoArtifactProjectionRecord, 'retention_state' | 'payload_kind' | 'lifecycle_state' | 'format'>>,
): IoArtifactRendererSummary {
  const mediaType = resolveIoArtifactMediaType(artifact);
  const family = classifyIoArtifactMedia(mediaType, artifact.payload_kind);
  const detail = artifact.lifecycle_state
    ? `${formatIoArtifactLifecycleStateLabel(artifact.lifecycle_state)} · ${formatIoArtifactRetentionStateLabel(artifact.retention_state)}`
    : formatIoArtifactRetentionStateLabel(artifact.retention_state);

  switch (family) {
    case 'text':
      return { family, title: 'Text', detail };
    case 'image':
      return { family, title: 'Image preview', detail };
    case 'audio':
      return { family, title: 'Audio', detail };
    case 'video':
      return { family, title: 'Video', detail };
    case '3d':
      return { family, title: '3D asset', detail };
    case 'table':
      return { family, title: 'Table', detail };
    case 'json':
      return { family, title: 'JSON', detail };
    case 'file':
      return { family, title: 'File', detail };
    case 'unknown':
      return { family, title: 'Unknown media', detail };
  }
}

export function resolveIoArtifactPayloadAvailability(
  artifact: Pick<IoArtifactProjectionRecord, 'payload_ref'> &
    Partial<Pick<IoArtifactProjectionRecord, 'retention_state' | 'read_handle' | 'stream_handle' | 'access_modes'>>,
): IoArtifactPayloadAvailability {
  if (
    artifact.retention_state === 'metadata_only' ||
    artifact.retention_state === 'expired' ||
    artifact.retention_state === 'deleted' ||
    artifact.retention_state === 'too_large'
  ) {
    return 'metadata_only';
  }
  if (artifact.read_handle || artifact.stream_handle || (artifact.access_modes?.length ?? 0) > 0) {
    return 'referenced';
  }
  return artifact.payload_ref && artifact.payload_ref.trim().length > 0
    ? 'referenced'
    : 'metadata_only';
}

export function formatIoArtifactAvailabilityLabel(
  artifact: Pick<IoArtifactProjectionRecord, 'payload_ref'> &
    Partial<Pick<IoArtifactProjectionRecord, 'retention_state' | 'read_handle' | 'stream_handle' | 'access_modes'>>,
): string {
  switch (resolveIoArtifactPayloadAvailability(artifact)) {
    case 'referenced':
      return 'Payload referenced';
    case 'metadata_only':
      return 'Metadata only';
  }
}

export function canReadIoArtifactBody(
  artifact: Pick<IoArtifactProjectionRecord, 'retention_state'> &
    Partial<Pick<IoArtifactProjectionRecord, 'read_handle' | 'access_modes' | 'lifecycle_state'>>,
): boolean {
  if (artifact.retention_state !== 'retained') {
    return false;
  }
  if (artifact.lifecycle_state && artifact.lifecycle_state !== 'retained') {
    return false;
  }
  return Boolean(
    artifact.read_handle ||
      artifact.access_modes?.includes('read') ||
      artifact.access_modes?.includes('download'),
  );
}

export function canAcknowledgeIoArtifactConsumed(
  artifact: Pick<IoArtifactProjectionRecord, 'artifact_id' | 'retention_state'>,
): boolean {
  return artifact.artifact_id.trim().length > 0 && artifact.retention_state === 'retained';
}

export function buildIoArtifactDownloadFilename(
  artifact: Pick<IoArtifactProjectionRecord, 'artifact_id'> &
    Partial<Pick<IoArtifactProjectionRecord, 'media_type' | 'format' | 'payload_kind'>>,
): string {
  const mediaType = resolveIoArtifactMediaType(artifact);
  const base = sanitizeArtifactFilenamePart(artifact.artifact_id) || 'artifact';
  const extension = artifactFileExtension(mediaType, artifact.payload_kind);
  return extension ? `${base}.${extension}` : base;
}

export function resolveIoArtifactMediaType(
  artifact: Pick<IoArtifactProjectionRecord, 'media_type'> &
    Partial<Pick<IoArtifactProjectionRecord, 'format'>>,
): string | null | undefined {
  return artifact.media_type ?? artifact.format?.media_type;
}

export function formatIoArtifactPayloadKindLabel(
  payloadKind: IoArtifactPayloadKind | null | undefined,
): string {
  switch (payloadKind) {
    case 'text':
      return 'Text';
    case 'image':
      return 'Image';
    case 'audio':
      return 'Audio';
    case 'video':
      return 'Video';
    case '3d':
      return '3D';
    case 'large_table':
      return 'Large table';
    case 'generic_binary':
      return 'Generic binary';
    case 'structured':
      return 'Structured';
    default:
      return 'Payload kind unknown';
  }
}

export function formatIoArtifactLifecycleStateLabel(
  lifecycleState: IoArtifactLifecycleState | null | undefined,
): string {
  switch (lifecycleState) {
    case 'declared':
      return 'Declared';
    case 'writing':
      return 'Writing';
    case 'streaming':
      return 'Streaming';
    case 'finalizing':
      return 'Finalizing';
    case 'retained':
      return 'Retained';
    case 'failed':
      return 'Failed';
    case 'expired':
      return 'Expired';
    case 'deleted':
      return 'Deleted';
    default:
      return 'Lifecycle unknown';
  }
}

export function formatIoArtifactAccessModes(modes: IoArtifactAccessMode[] | null | undefined): string {
  if (!modes || modes.length === 0) {
    return 'Unavailable';
  }
  return modes.map(formatRetentionEnumLabel).join(', ');
}

export function buildIoArtifactDescriptorMetadataRows(
  artifact: Pick<
    IoArtifactProjectionRecord,
    | 'payload_kind'
    | 'lifecycle_state'
    | 'access_modes'
    | 'read_handle'
    | 'stream_handle'
    | 'runtime_id'
    | 'runtime_version'
    | 'selected_backend_key'
    | 'model_id'
    | 'model_version'
    | 'format'
  >,
): IoArtifactDescriptorMetadataRow[] {
  const rows: IoArtifactDescriptorMetadataRow[] = [
    {
      label: 'Payload Kind',
      value: formatIoArtifactPayloadKindLabel(artifact.payload_kind),
      mono: false,
    },
    {
      label: 'Lifecycle',
      value: formatIoArtifactLifecycleStateLabel(artifact.lifecycle_state),
      mono: false,
    },
    {
      label: 'Access',
      value: formatIoArtifactAccessModes(artifact.access_modes),
      mono: false,
    },
  ];

  if (artifact.read_handle) {
    rows.push({ label: 'Read Handle', value: artifact.read_handle, mono: true });
  }
  if (artifact.stream_handle) {
    rows.push({ label: 'Stream Handle', value: artifact.stream_handle, mono: true });
  }
  if (artifact.runtime_id) {
    rows.push({ label: 'Runtime', value: artifact.runtime_id, mono: true });
  }
  if (artifact.runtime_version) {
    rows.push({ label: 'Runtime Version', value: artifact.runtime_version, mono: true });
  }
  if (artifact.selected_backend_key) {
    rows.push({ label: 'Backend', value: artifact.selected_backend_key, mono: true });
  }
  if (artifact.model_id) {
    rows.push({ label: 'Model', value: artifact.model_id, mono: true });
  }
  if (artifact.model_version) {
    rows.push({ label: 'Model Version', value: artifact.model_version, mono: true });
  }

  const format = artifact.format;
  if (!format) {
    return rows;
  }

  rows.push(
    { label: 'Format', value: format.format_id, mono: true },
    { label: 'Format Media', value: format.media_type, mono: true },
  );
  if (format.codec_id) {
    rows.push({ label: 'Codec', value: format.codec_id, mono: true });
  }
  if (format.quality_percent !== null && format.quality_percent !== undefined) {
    rows.push({ label: 'Quality', value: `${format.quality_percent}%`, mono: false });
  }
  if (format.bitrate_kbps !== null && format.bitrate_kbps !== undefined) {
    rows.push({ label: 'Bitrate', value: `${format.bitrate_kbps} kbps`, mono: false });
  }
  if (format.crf !== null && format.crf !== undefined) {
    rows.push({ label: 'CRF', value: String(format.crf), mono: false });
  }
  if (format.bit_depth) {
    rows.push({ label: 'Bit Depth', value: format.bit_depth, mono: true });
  }
  if (format.color_profile_id) {
    rows.push({ label: 'Color Profile', value: format.color_profile_id, mono: true });
  }
  if (format.converter_id) {
    rows.push({ label: 'Converter', value: format.converter_id, mono: true });
  }
  if (format.converter_version) {
    rows.push({ label: 'Converter Version', value: format.converter_version, mono: true });
  }
  if (format.library_version) {
    rows.push({ label: 'Library Version', value: format.library_version, mono: true });
  }
  if (format.conversion_status) {
    rows.push({
      label: 'Conversion',
      value: formatIoArtifactConversionStatusLabel(format.conversion_status),
      mono: false,
    });
  }
  if (format.conversion_id) {
    rows.push({ label: 'Conversion Id', value: format.conversion_id, mono: true });
  }
  if (format.conversion_command_id) {
    rows.push({ label: 'Conversion Command', value: format.conversion_command_id, mono: true });
  }
  format.conversion_dependencies?.forEach((dependency, index) => {
    const ordinal = index + 1;
    rows.push({
      label: `Dependency ${ordinal}`,
      value: `${dependency.dependency_id}@${dependency.active_version} · ${dependency.lease_id}`,
      mono: true,
    });
    rows.push({
      label: `Lease Holder ${ordinal}`,
      value: dependency.lease_holder,
      mono: true,
    });
  });
  return rows;
}

export function formatIoArtifactConversionStatusLabel(
  status: IoArtifactConversionStatus | null | undefined,
): string {
  switch (status) {
    case 'converted':
      return 'Converted';
    case 'passed_through':
      return 'Passed through';
    case 'failed':
      return 'Failed';
    default:
      return 'Conversion unknown';
  }
}

export function formatIoArtifactRetentionStateLabel(
  retentionState: IoArtifactRetentionState | null | undefined,
): string {
  switch (retentionState) {
    case 'retained':
      return 'Payload retained';
    case 'metadata_only':
      return 'Metadata retained only';
    case 'external':
      return 'External reference';
    case 'truncated':
      return 'Payload truncated';
    case 'too_large':
      return 'Too large to retain';
    case 'expired':
      return 'Payload expired';
    case 'deleted':
      return 'Payload deleted';
    default:
      return 'Retention unknown';
  }
}

export function formatIoArtifactDetailValue(value: string | null | undefined): string {
  return value && value.trim().length > 0 ? value : 'Unavailable';
}

export function formatIoArtifactEndpointValue(
  nodeId: string | null | undefined,
  portId: string | null | undefined,
): string {
  const nodeLabel = formatIoArtifactDetailValue(nodeId);
  if (nodeLabel === 'Unavailable') {
    return nodeLabel;
  }
  return portId && portId.trim().length > 0 ? `${nodeLabel}:${portId}` : nodeLabel;
}

export function formatIoArtifactBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) {
    return 'Size unknown';
  }
  if (bytes >= 1_073_741_824) {
    return `${(bytes / 1_073_741_824).toFixed(1)} GiB`;
  }
  if (bytes >= 1_048_576) {
    return `${(bytes / 1_048_576).toFixed(1)} MiB`;
  }
  if (bytes >= 1_024) {
    return `${(bytes / 1_024).toFixed(1)} KiB`;
  }
  return `${bytes} B`;
}

export function buildIoArtifactPreviewReadRequest(
  target: IoArtifactPreviewReadTarget,
  maxBytes = IO_ARTIFACT_PREVIEW_MAX_BYTES,
): IoArtifactPreviewReadRequest {
  const artifactId = typeof target === 'string' ? target : target.artifact_id;
  const mediaType = typeof target === 'string' ? undefined : target.media_type ?? target.format?.media_type;
  const payloadKind = typeof target === 'string' ? undefined : target.payload_kind;
  if (classifyIoArtifactMedia(mediaType, payloadKind) === 'image') {
    return { artifact_id: artifactId };
  }

  return {
    artifact_id: artifactId,
    byte_range_start: 0,
    byte_range_end_exclusive: Math.max(1, Math.trunc(maxBytes)),
  };
}

export function formatIoArtifactPreviewExtent(preview: IoArtifactPreviewExtent): string {
  const base = `${preview.mediaType} · ${formatIoArtifactBytes(preview.byteLength)}`;
  return preview.complete ? base : `${base} · partial preview`;
}

export function canRenderIoArtifactTextPreview(
  mediaType: string | null | undefined,
  payloadKind?: IoArtifactPayloadKind | null,
): boolean {
  const family = classifyIoArtifactMedia(mediaType, payloadKind);
  return family === 'text' || family === 'json' || family === 'table';
}

export function decodeIoArtifactTextPreview(
  body: number[] | Uint8Array,
  maxCharacters = 32_000,
): IoArtifactTextPreview {
  const byteArray = body instanceof Uint8Array ? body : Uint8Array.from(body);
  const decoded = new TextDecoder('utf-8', { fatal: false }).decode(byteArray);
  if (decoded.length <= maxCharacters) {
    return {
      text: decoded,
      truncated: false,
    };
  }
  return {
    text: decoded.slice(0, Math.max(0, maxCharacters)),
    truncated: true,
  };
}

export function formatProjectionFreshness(state: ProjectionStateRecord | null): string {
  return formatProjectionFreshnessState(state);
}

export function buildRetentionPolicyDetailRows(
  policy: DiagnosticsRetentionPolicy | null,
): IoRetentionDetailRow[] {
  if (!policy) {
    return [];
  }
  return [
    { label: 'Policy', value: policy.policy_id, mono: true },
    { label: 'Version', value: String(policy.policy_version), mono: false },
    { label: 'Class', value: policy.retention_class, mono: true },
    { label: 'Days', value: String(policy.retention_days), mono: false },
    { label: 'Applied', value: formatIoRetentionTimestamp(policy.applied_at_ms), mono: false },
  ];
}

export function buildRetentionPolicySettingRows(
  policy: DiagnosticsRetentionPolicy | null,
): IoRetentionDetailRow[] {
  if (!policy) {
    return [];
  }
  const settings = policy.settings;
  return [
    {
      label: 'Final Outputs',
      value: formatRetentionScopePolicy(settings.final_outputs),
      mono: false,
    },
    {
      label: 'Workflow Inputs',
      value: formatRetentionScopePolicy(settings.workflow_inputs),
      mono: false,
    },
    {
      label: 'Intermediate Node I/O',
      value: formatRetentionScopePolicy(settings.intermediate_node_io),
      mono: false,
    },
    {
      label: 'Failed Run Data',
      value: formatRetentionScopePolicy(settings.failed_run_data),
      mono: false,
    },
    {
      label: 'Maximum Artifact Size',
      value: formatIoArtifactBytes(settings.max_artifact_bytes),
      mono: false,
    },
    {
      label: 'Maximum Total Storage',
      value: formatIoArtifactBytes(settings.max_total_storage_bytes),
      mono: false,
    },
    {
      label: 'Media Behavior',
      value: formatRetentionEnumLabel(settings.media_behavior),
      mono: false,
    },
    {
      label: 'Compression',
      value: formatRetentionEnumLabel(settings.compression_behavior),
      mono: false,
    },
    {
      label: 'Cleanup Trigger',
      value: formatRetentionEnumLabel(settings.cleanup_trigger),
      mono: false,
    },
  ];
}

export function buildRetentionCleanupDetailRows(
  cleanup: WorkflowRetentionCleanupResult | null,
): IoRetentionDetailRow[] {
  if (!cleanup) {
    return [];
  }
  return [
    { label: 'Policy', value: cleanup.policy_id, mono: true },
    { label: 'Version', value: String(cleanup.policy_version), mono: false },
    { label: 'Class', value: cleanup.retention_class, mono: true },
    { label: 'Cutoff', value: formatIoRetentionTimestamp(cleanup.cutoff_occurred_before_ms), mono: false },
    { label: 'Expired', value: String(cleanup.expired_artifact_count), mono: false },
    {
      label: 'Last Event Seq',
      value:
        cleanup.last_event_seq === null || cleanup.last_event_seq === undefined
          ? 'Unavailable'
          : String(cleanup.last_event_seq),
      mono: false,
    },
  ];
}

function formatRetentionScopePolicy(
  policy: DiagnosticsRetentionPolicy['settings']['final_outputs'],
): string {
  return `${policy.retention_days} days, ${formatRetentionEnumLabel(policy.payload_mode)}`;
}

function formatRetentionEnumLabel(value: string): string {
  return value
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function sanitizeArtifactFilenamePart(value: string): string {
  return value
    .trim()
    .replace(/[^a-zA-Z0-9._-]+/g, '-')
    .replace(/^[.-]+|[.-]+$/g, '')
    .slice(0, 96);
}

function artifactFileExtension(
  mediaType: string | null | undefined,
  payloadKind: IoArtifactPayloadKind | null | undefined,
): string | null {
  const normalized = mediaType?.toLowerCase() ?? '';
  if (normalized.includes('jpeg')) {
    return 'jpg';
  }
  if (normalized.includes('png')) {
    return 'png';
  }
  if (normalized.includes('webp')) {
    return 'webp';
  }
  if (normalized.includes('gif')) {
    return 'gif';
  }
  if (normalized.includes('wav')) {
    return 'wav';
  }
  if (normalized.includes('mpeg') || normalized.includes('mp3')) {
    return 'mp3';
  }
  if (normalized.includes('ogg')) {
    return 'ogg';
  }
  if (normalized.includes('mp4')) {
    return 'mp4';
  }
  if (normalized.includes('json')) {
    return 'json';
  }
  if (normalized.includes('csv')) {
    return 'csv';
  }
  if (normalized.startsWith('text/')) {
    return 'txt';
  }
  if (payloadKind === 'structured') {
    return 'json';
  }
  return null;
}

export function formatIoRetentionTimestamp(value: number | null | undefined): string {
  if (!value) {
    return 'Unavailable';
  }
  return new Date(value).toLocaleString();
}

// ---------------------------------------------------------------------------
// Node and port tree (I/O Inspector board, workbench redesign M3).
//
// The left panel lists the run snapshot's nodes, each expandable to the ports
// that have retained-artifact metadata. Entries come from the backend-resolved
// node I/O rows joined with the artifact projection; artifacts that no resolved
// row references fall back to their own producer/consumer metadata so nothing
// recorded for the run is hidden. Everything here is a pure projection of the
// `workflow_run_inspection_query` response.
// ---------------------------------------------------------------------------

export type IoInspectorDirectionFilter = 'all' | ResolvedNodeIoDirection;

export interface IoInspectorTreeEntry {
  /** Stable row identity: direction, node, port and artifact. */
  key: string;
  nodeId: string;
  portId: string;
  direction: ResolvedNodeIoDirection;
  artifact: IoArtifactProjectionRecord;
  /** The backend-resolved row, or null when the entry came from artifact metadata alone. */
  nodeIo: ResolvedNodeIoRecord | null;
}

export interface IoInspectorTreeNode {
  /** Node id; for the workflow boundary group this is `IO_INSPECTOR_BOUNDARY_NODE_ID`. */
  nodeId: string;
  nodeType: string | null;
  status: NodeExecutionProjectionStatus | null;
  /** True for the synthetic group holding artifacts not attached to any node. */
  boundary: boolean;
  /** Inputs first, then outputs, in backend order within each direction. */
  entries: IoInspectorTreeEntry[];
}

export interface IoInspectorTreeSource {
  graphNodes: Pick<GraphNode, 'id' | 'node_type'>[];
  nodeStatuses: Pick<NodeStatusProjectionRecord, 'node_id' | 'node_type' | 'status'>[];
  resolvedNodeIo: ResolvedNodeIoRecord[];
  artifacts: IoArtifactProjectionRecord[];
}

export interface IoInspectorDirectionCounts {
  all: number;
  input: number;
  output: number;
}

export const IO_INSPECTOR_BOUNDARY_NODE_ID = '\u0000workflow-boundary';
const UNRECORDED_PORT_LABEL = 'port not recorded';

export function ioInspectorEntryKey(
  direction: ResolvedNodeIoDirection,
  nodeId: string,
  portId: string,
  artifactId: string,
): string {
  return [direction, nodeId, portId, artifactId].join('\u001f');
}

export function buildIoInspectorNodeTree(source: IoInspectorTreeSource): IoInspectorTreeNode[] {
  const nodes = new Map<string, IoInspectorTreeNode>();
  const ensureNode = (nodeId: string): IoInspectorTreeNode => {
    let node = nodes.get(nodeId);
    if (!node) {
      const boundary = nodeId === IO_INSPECTOR_BOUNDARY_NODE_ID;
      node = { nodeId, nodeType: null, status: null, boundary, entries: [] };
      nodes.set(nodeId, node);
    }
    return node;
  };

  for (const graphNode of source.graphNodes) {
    ensureNode(graphNode.id).nodeType = graphNode.node_type || null;
  }
  for (const status of source.nodeStatuses) {
    const node = ensureNode(status.node_id);
    node.status = status.status;
    node.nodeType ??= status.node_type ?? null;
  }

  const seenKeys = new Set<string>();
  const referencedArtifactIds = new Set<string>();
  const addEntry = (entry: Omit<IoInspectorTreeEntry, 'key'>): void => {
    const key = ioInspectorEntryKey(entry.direction, entry.nodeId, entry.portId, entry.artifact.artifact_id);
    if (seenKeys.has(key)) {
      return;
    }
    seenKeys.add(key);
    const node = ensureNode(entry.nodeId);
    if (!node.nodeType && entry.artifact.node_id === entry.nodeId && entry.artifact.node_type) {
      node.nodeType = entry.artifact.node_type;
    }
    node.entries.push({ ...entry, key });
  };

  for (const row of buildResolvedNodeIoDisplayRows(source.resolvedNodeIo, source.artifacts)) {
    if (!row.artifact || !isNormalResolvedNodeIoDisplayRow(row)) {
      continue;
    }
    referencedArtifactIds.add(row.artifact.artifact_id);
    addEntry({
      nodeId: row.nodeIo.node_id,
      portId: row.nodeIo.port_id,
      direction: row.nodeIo.direction,
      artifact: row.artifact,
      nodeIo: row.nodeIo,
    });
  }

  for (const artifact of source.artifacts) {
    if (referencedArtifactIds.has(artifact.artifact_id) || isDiagnosticArtifact(artifact)) {
      continue;
    }
    for (const entry of entriesFromArtifactMetadata(artifact)) {
      addEntry(entry);
    }
  }

  const ordered = [...nodes.values()].map((node) => ({
    ...node,
    entries: [
      ...node.entries.filter((entry) => entry.direction === 'input'),
      ...node.entries.filter((entry) => entry.direction === 'output'),
    ],
  }));
  // The boundary group only exists when an artifact needed it; keep it last.
  return [...ordered.filter((node) => !node.boundary), ...ordered.filter((node) => node.boundary)];
}

function isDiagnosticArtifact(artifact: IoArtifactProjectionRecord): boolean {
  return [artifact.producer_port_id, artifact.consumer_port_id].some(isDiagnosticDataPort);
}

function entriesFromArtifactMetadata(
  artifact: IoArtifactProjectionRecord,
): Omit<IoInspectorTreeEntry, 'key'>[] {
  const entries: Omit<IoInspectorTreeEntry, 'key'>[] = [];
  const producer = normalizeArtifactIdentity(artifact.producer_node_id);
  const consumer = normalizeArtifactIdentity(artifact.consumer_node_id);
  if (producer) {
    entries.push({
      nodeId: producer,
      portId: normalizeArtifactIdentity(artifact.producer_port_id) ?? UNRECORDED_PORT_LABEL,
      direction: 'output',
      artifact,
      nodeIo: null,
    });
  }
  if (consumer) {
    entries.push({
      nodeId: consumer,
      portId: normalizeArtifactIdentity(artifact.consumer_port_id) ?? UNRECORDED_PORT_LABEL,
      direction: 'input',
      artifact,
      nodeIo: null,
    });
  }
  if (entries.length > 0) {
    return entries;
  }
  const direction: ResolvedNodeIoDirection = artifact.artifact_role.includes('input') ? 'input' : 'output';
  return [
    {
      nodeId: normalizeArtifactIdentity(artifact.node_id) ?? IO_INSPECTOR_BOUNDARY_NODE_ID,
      portId: UNRECORDED_PORT_LABEL,
      direction,
      artifact,
      nodeIo: null,
    },
  ];
}

/**
 * Narrow the tree to one direction and a search text. With no search text
 * every node stays listed (a node with no matching ports still shows its
 * status); with search text a node stays when its id or type matches (all its
 * ports in the direction) or when one of its ports or artifact ids matches.
 */
export function filterIoInspectorNodeTree(
  nodes: IoInspectorTreeNode[],
  filter: { direction: IoInspectorDirectionFilter; search: string },
): IoInspectorTreeNode[] {
  const query = filter.search.trim().toLowerCase();
  const inDirection = (entry: IoInspectorTreeEntry) =>
    filter.direction === 'all' || entry.direction === filter.direction;
  return nodes.flatMap((node) => {
    const directional = node.entries.filter(inDirection);
    if (!query) {
      return [{ ...node, entries: directional }];
    }
    const nodeMatches = [formatIoInspectorNodeLabel(node), node.nodeType ?? '']
      .some((value) => value.toLowerCase().includes(query));
    const entries = nodeMatches
      ? directional
      : directional.filter((entry) =>
          [entry.portId, entry.artifact.artifact_id].some((value) => value.toLowerCase().includes(query)),
        );
    return nodeMatches || entries.length > 0 ? [{ ...node, entries }] : [];
  });
}

export function countIoInspectorEntriesByDirection(nodes: IoInspectorTreeNode[]): IoInspectorDirectionCounts {
  const counts: IoInspectorDirectionCounts = { all: 0, input: 0, output: 0 };
  for (const entry of nodes.flatMap((node) => node.entries)) {
    counts.all += 1;
    counts[entry.direction] += 1;
  }
  return counts;
}

export function findIoInspectorEntry(
  nodes: IoInspectorTreeNode[],
  key: string | null,
): IoInspectorTreeEntry | null {
  if (!key) {
    return null;
  }
  for (const node of nodes) {
    const entry = node.entries.find((candidate) => candidate.key === key);
    if (entry) {
      return entry;
    }
  }
  return null;
}

/**
 * Keep the chosen entry while it exists; otherwise open on the first image
 * output (what most runs are inspected for), then the first output, then the
 * first entry. Null when the run has no retained I/O.
 */
export function resolveIoInspectorSelectedEntryKey(
  currentKey: string | null,
  nodes: IoInspectorTreeNode[],
): string | null {
  if (findIoInspectorEntry(nodes, currentKey)) {
    return currentKey;
  }
  const entries = nodes.flatMap((node) => node.entries);
  const outputs = entries.filter((entry) => entry.direction === 'output');
  const image = outputs.find(
    (entry) =>
      classifyIoArtifactMedia(resolveIoArtifactMediaType(entry.artifact), entry.artifact.payload_kind) === 'image',
  );
  return (image ?? outputs[0] ?? entries[0])?.key ?? null;
}

export function formatIoInspectorNodeLabel(node: Pick<IoInspectorTreeNode, 'nodeId' | 'boundary'>): string {
  return node.boundary ? 'Workflow boundary' : node.nodeId;
}

export function formatIoInspectorNodeStatusLabel(status: NodeExecutionProjectionStatus | null): string {
  switch (status) {
    case 'queued':
      return 'Queued';
    case 'running':
      return 'Running';
    case 'waiting':
      return 'Waiting';
    case 'completed':
      return 'Succeeded';
    case 'failed':
      return 'Failed';
    case 'cancelled':
      return 'Cancelled';
    case null:
      return 'No status recorded';
  }
}

/** Second line of a node row, for example "Succeeded, 2 retained ports". */
export function formatIoInspectorNodeSubLabel(
  node: Pick<IoInspectorTreeNode, 'status' | 'boundary' | 'entries'>,
  staleFactCount = 0,
): string {
  const parts: string[] = [];
  if (!node.boundary) {
    parts.push(formatIoInspectorNodeStatusLabel(node.status));
  }
  const ports = node.entries.length;
  if (ports > 0) {
    parts.push(`${ports} retained ${ports === 1 ? 'port' : 'ports'}`);
  } else if (node.status === 'queued' || node.status === 'waiting') {
    parts.push('not reached yet');
  } else {
    parts.push('no retained I/O');
  }
  if (staleFactCount > 0) {
    parts.push(`${staleFactCount} stale graph ${staleFactCount === 1 ? 'fact' : 'facts'}`);
  }
  return parts.join(', ');
}

export function formatIoInspectorDirectionMark(direction: ResolvedNodeIoDirection): string {
  return direction === 'input' ? '←' : '→';
}

/** Kind and retention of an entry, for example "Image, payload retained". */
export function formatIoInspectorEntryDetail(
  artifact: Pick<IoArtifactProjectionRecord, 'payload_kind' | 'media_type' | 'retention_state'> &
    Partial<Pick<IoArtifactProjectionRecord, 'format' | 'lifecycle_state'>>,
): string {
  return `${formatIoInspectorKindLabel(artifact)}, ${formatIoInspectorRetentionLabel(artifact).toLowerCase()}`;
}

export function formatIoInspectorKindLabel(
  artifact: Pick<IoArtifactProjectionRecord, 'payload_kind' | 'media_type'> &
    Partial<Pick<IoArtifactProjectionRecord, 'format'>>,
): string {
  return artifact.payload_kind
    ? formatIoArtifactPayloadKindLabel(artifact.payload_kind)
    : formatIoArtifactMediaLabel(resolveIoArtifactMediaType(artifact));
}

/** Retention in words; a body still being written reads "Streaming". */
export function formatIoInspectorRetentionLabel(
  artifact: Pick<IoArtifactProjectionRecord, 'retention_state'> &
    Partial<Pick<IoArtifactProjectionRecord, 'lifecycle_state'>>,
): string {
  if (artifact.lifecycle_state === 'streaming' || artifact.lifecycle_state === 'writing') {
    return formatIoArtifactLifecycleStateLabel(artifact.lifecycle_state);
  }
  return formatIoArtifactRetentionStateLabel(artifact.retention_state);
}

/** Static chips under the artifact title: role, kind, retention, media and size. */
export function buildIoArtifactChipLabels(artifact: IoArtifactProjectionRecord): string[] {
  const media = resolveIoArtifactMediaType(artifact)?.trim() || 'media type unknown';
  return [
    formatIoArtifactRoleLabel(artifact.artifact_role),
    formatIoInspectorKindLabel(artifact),
    formatIoInspectorRetentionLabel(artifact),
    `${media}, ${formatIoArtifactBytes(artifact.size_bytes)}`,
  ];
}

/**
 * Dimension line: "run_x, 5 nodes, 6 retained artifacts, 4.1 MiB of payloads
 * kept", followed by the I/O projection freshness when given (ADR-014 asks
 * pages backed by cursor read models to show it).
 */
export function buildIoInspectorDimensionLabel(input: {
  runId: string;
  nodeCount: number;
  artifacts: Pick<IoArtifactProjectionRecord, 'retention_state' | 'size_bytes'>[];
  freshness?: string | null;
}): string {
  const keptBytes = input.artifacts
    .filter((artifact) => artifact.retention_state === 'retained')
    .reduce((total, artifact) => total + (artifact.size_bytes ?? 0), 0);
  const nodes = `${input.nodeCount} ${input.nodeCount === 1 ? 'node' : 'nodes'}`;
  const artifacts = `${input.artifacts.length} retained ${input.artifacts.length === 1 ? 'artifact' : 'artifacts'}`;
  const base = `${input.runId}, ${nodes}, ${artifacts}, ${formatIoArtifactBytes(keptBytes)} of payloads kept`;
  const freshness = input.freshness?.trim();
  return freshness ? `${base}, projection ${freshness.charAt(0).toLowerCase()}${freshness.slice(1)}` : base;
}

export interface IoInspectorBackendFilterResult {
  artifacts: IoArtifactProjectionRecord[];
  resolvedNodeIo: ResolvedNodeIoRecord[];
}

/**
 * Apply the Backend filter: keep artifacts whose selected backend key equals
 * the filter and the resolved rows that point at a kept artifact. An empty
 * filter keeps everything.
 */
export function applyIoInspectorBackendFilter(
  artifacts: IoArtifactProjectionRecord[],
  resolvedNodeIo: ResolvedNodeIoRecord[],
  backendKey: string,
): IoInspectorBackendFilterResult {
  const key = backendKey.trim();
  if (key.length === 0) {
    return { artifacts, resolvedNodeIo };
  }
  const kept = artifacts.filter((artifact) => artifact.selected_backend_key === key);
  const ids = (field: 'artifact_fact_id' | 'payload_artifact_id' | 'artifact_id') =>
    new Set(kept.map((artifact) => normalizeArtifactIdentity(artifact[field])).filter((id): id is string => id !== null));
  const factIds = ids('artifact_fact_id');
  const payloadIds = ids('payload_artifact_id');
  const artifactIds = ids('artifact_id');
  const has = (set: Set<string>, value: string | null | undefined) => {
    const normalized = normalizeArtifactIdentity(value);
    return normalized !== null && set.has(normalized);
  };
  return {
    artifacts: kept,
    resolvedNodeIo: resolvedNodeIo.filter(
      (record) =>
        has(factIds, record.artifact_fact_id) ||
        has(payloadIds, record.payload_artifact_id) ||
        has(artifactIds, record.artifact_id),
    ),
  };
}

export interface IoArtifactDetailSections {
  origin: KeyValueRow[];
  making: KeyValueRow[];
  storage: KeyValueRow[];
}

const STORAGE_DESCRIPTOR_LABELS = new Set(['Payload Kind', 'Lifecycle', 'Access', 'Read Handle', 'Stream Handle']);

/**
 * The "Where it came from", "How it was made" and "Storage" sections of the
 * artifact detail. Descriptor rows keep their existing values; format and
 * conversion rows go under "How it was made". `formatTimestamp` is injected so
 * the caller owns locale formatting.
 */
export function buildIoArtifactDetailSections(
  entry: Pick<IoInspectorTreeEntry, 'artifact' | 'nodeIo' | 'direction' | 'portId'>,
  formatTimestamp: (ms: number) => string,
): IoArtifactDetailSections {
  const { artifact, nodeIo } = entry;
  const origin: KeyValueRow[] = [
    { key: 'Producer', value: formatIoArtifactEndpointValue(artifact.producer_node_id, artifact.producer_port_id), mono: true },
    { key: 'Consumer', value: formatIoArtifactEndpointValue(artifact.consumer_node_id, artifact.consumer_port_id), mono: true },
    { key: 'Direction', value: formatResolvedNodeIoDirectionLabel(entry.direction) },
    { key: 'Observed', value: formatTimestamp(artifact.occurred_at_ms) },
  ];
  if (nodeIo) {
    origin.push(
      { key: 'Resolution', value: formatResolvedNodeIoResolutionLabel(nodeIo.resolution) },
      { key: 'Provenance', value: formatResolvedNodeIoProvenance(nodeIo) },
    );
  } else {
    origin.push({ key: 'Provenance', value: 'From artifact metadata; no resolved node I/O row' });
  }

  const making: KeyValueRow[] = [];
  const storage: KeyValueRow[] = [];
  for (const row of buildIoArtifactDescriptorMetadataRows(artifact)) {
    const target = STORAGE_DESCRIPTOR_LABELS.has(row.label) ? storage : making;
    target.push({ key: sentenceCaseLabel(row.label), value: row.value, mono: row.mono });
  }
  if (!making.some((row) => row.key === 'Backend')) {
    making.unshift({ key: 'Backend', value: formatIoArtifactDetailValue(artifact.selected_backend_key), mono: true });
  }
  storage.push(
    { key: 'Retention', value: formatIoArtifactRetentionStateLabel(artifact.retention_state) },
    { key: 'Availability', value: formatIoArtifactAvailabilityLabel(artifact) },
  );
  if (artifact.retention_reason?.trim()) {
    storage.push({ key: 'Retention reason', value: artifact.retention_reason.trim() });
  }
  storage.push(
    { key: 'Size', value: formatIoArtifactBytes(artifact.size_bytes), mono: true },
    { key: 'Payload ref', value: formatIoArtifactDetailValue(artifact.payload_ref), mono: true },
    { key: 'Content hash', value: formatIoArtifactDetailValue(artifact.content_hash), mono: true },
  );
  return { origin, making, storage };
}

/** "Read Handle" → "Read handle"; all-caps words such as "CRF" stay as they are. */
export function sentenceCaseLabel(label: string): string {
  return label
    .split(' ')
    .map((word, index) => (index === 0 || /^[A-Z0-9]+$/.test(word) ? word : word.toLowerCase()))
    .join(' ');
}

export type IoArtifactPreviewMode = 'none' | 'image' | 'audio' | 'text' | 'binary';

/**
 * How a loaded body renders. Images and audio play from the object URL; text,
 * JSON and tables show the decoded bounded preview; everything else (video
 * included, whose preview read is bounded to the first 64 KiB) gets the
 * binary-body message pointing at Download.
 */
export function resolveIoArtifactPreviewMode(
  family: IoArtifactMediaFamily,
  preview: { text?: string | null } | null | undefined,
): IoArtifactPreviewMode {
  if (!preview) {
    return 'none';
  }
  if (family === 'image') {
    return 'image';
  }
  if (family === 'audio') {
    return 'audio';
  }
  if (preview.text !== null && preview.text !== undefined) {
    return 'text';
  }
  return 'binary';
}

/** Why Read body and Download are disabled, or null when they are available. */
export function describeIoArtifactReadUnavailable(
  artifact: Pick<IoArtifactProjectionRecord, 'retention_state'> &
    Partial<Pick<IoArtifactProjectionRecord, 'read_handle' | 'access_modes' | 'lifecycle_state'>>,
): string | null {
  if (canReadIoArtifactBody(artifact)) {
    return null;
  }
  if (artifact.retention_state !== 'retained') {
    return `${formatIoArtifactRetentionStateLabel(artifact.retention_state)}, so there is no body to read.`;
  }
  if (artifact.lifecycle_state && artifact.lifecycle_state !== 'retained') {
    return `The body is ${formatIoArtifactLifecycleStateLabel(artifact.lifecycle_state).toLowerCase()}; it can be read once it is retained.`;
  }
  return 'The engine did not grant read or download access to this body.';
}

export function describeIoArtifactStreamUnavailable(
  artifact: Partial<Pick<IoArtifactProjectionRecord, 'stream_handle'>>,
): string | null {
  return artifact.stream_handle ? null : 'This body was not written as a stream.';
}

export function describeIoArtifactAcknowledgeUnavailable(
  artifact: Pick<IoArtifactProjectionRecord, 'artifact_id' | 'retention_state'>,
): string | null {
  return canAcknowledgeIoArtifactConsumed(artifact)
    ? null
    : 'Only retained payloads can be acknowledged as consumed.';
}

export interface StaleGraphFactCard {
  key: string;
  title: string;
  code: string;
  explanation: string;
  suggestion: string | null;
}

/**
 * Muted, ErrorCard-shaped explanations of backend graph diagnostics ("stale
 * graph facts"). They describe the captured graph, not a failure of this page,
 * so they never use the danger colour.
 */
export function buildStaleGraphFactCards(diagnostics: WorkflowGraphDiagnostic[]): StaleGraphFactCard[] {
  const seen = new Set<string>();
  return diagnostics.flatMap((diagnostic) => {
    const key = [diagnostic.scope, diagnostic.node_id ?? '', diagnostic.code, diagnostic.message].join('\u001f');
    if (seen.has(key)) {
      return [];
    }
    seen.add(key);
    return [
      {
        key,
        title: humanizeDiagnosticCode(diagnostic.code),
        code: `${formatGraphDiagnosticSeverityLabel(diagnostic.severity)}, ${diagnostic.code}`,
        explanation: diagnostic.message,
        suggestion: diagnostic.blocking_submission
          ? 'Try: open the workflow in Graph and fix this node before running it again.'
          : null,
      },
    ];
  });
}

export function formatGraphDiagnosticSeverityLabel(severity: WorkflowGraphDiagnostic['severity']): string {
  switch (severity) {
    case 'error':
      return 'Error';
    case 'warning':
      return 'Warning';
    case 'info':
      return 'Note';
  }
}

function humanizeDiagnosticCode(code: string): string {
  const words = code.replaceAll('_', ' ').trim();
  return words.length > 0 ? words.charAt(0).toUpperCase() + words.slice(1) : 'Graph fact';
}

/** Split graph diagnostics into graph-scope facts and facts per node id. */
export function partitionGraphDiagnosticsByNode(diagnostics: WorkflowGraphDiagnostic[]): {
  graph: WorkflowGraphDiagnostic[];
  byNodeId: Map<string, WorkflowGraphDiagnostic[]>;
} {
  const graph: WorkflowGraphDiagnostic[] = [];
  const byNodeId = new Map<string, WorkflowGraphDiagnostic[]>();
  for (const diagnostic of diagnostics) {
    const nodeId = normalizeArtifactIdentity(diagnostic.node_id);
    if (!nodeId) {
      graph.push(diagnostic);
      continue;
    }
    byNodeId.set(nodeId, [...(byNodeId.get(nodeId) ?? []), diagnostic]);
  }
  return { graph, byNodeId };
}

export interface GraphCanvasViewBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

const FALLBACK_CANVAS_VIEWBOX: GraphCanvasViewBox = { x: 0, y: 0, width: 640, height: 360 };

/**
 * Read a canvas view box given either as the SVG string "x y w h" or as an
 * object. Malformed input falls back to the empty-canvas box so the saved
 * graph surface still lays out.
 */
export function parseGraphCanvasViewBox(viewBox: string | GraphCanvasViewBox | null | undefined): GraphCanvasViewBox {
  if (viewBox && typeof viewBox === 'object') {
    return viewBox;
  }
  const parts = (viewBox ?? '').trim().split(/[\s,]+/).map(Number);
  if (parts.length !== 4 || parts.some((part) => !Number.isFinite(part)) || parts[2] <= 0 || parts[3] <= 0) {
    return { ...FALLBACK_CANVAS_VIEWBOX };
  }
  const [x, y, width, height] = parts;
  return { x, y, width, height };
}

export interface IoArtifactDownloadUrlOwner {
  /** Revoke `objectUrl` after the delay; the owner keeps the timer. */
  release: (objectUrl: string) => void;
  /** Cancel pending timers and revoke every URL still held. */
  dispose: () => void;
  pendingCount: () => number;
}

/**
 * One owner for the object URLs handed to the browser for downloads. Each URL
 * is revoked after `delayMs` (the browser needs it alive while the download
 * starts) or when the page is destroyed, whichever comes first.
 */
export function createIoArtifactDownloadUrlOwner(options: {
  delayMs: number;
  revoke: (objectUrl: string) => void;
  schedule: (callback: () => void, delayMs: number) => number;
  cancel: (handle: number) => void;
}): IoArtifactDownloadUrlOwner {
  const pending = new Map<number, string>();
  let disposed = false;
  return {
    release(objectUrl) {
      if (disposed) {
        options.revoke(objectUrl);
        return;
      }
      const handle = options.schedule(() => {
        pending.delete(handle);
        options.revoke(objectUrl);
      }, options.delayMs);
      pending.set(handle, objectUrl);
    },
    dispose() {
      disposed = true;
      for (const [handle, objectUrl] of pending) {
        options.cancel(handle);
        options.revoke(objectUrl);
      }
      pending.clear();
    },
    pendingCount: () => pending.size,
  };
}
