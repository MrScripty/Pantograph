import type {
  IoArtifactAccessMode,
  DiagnosticsRetentionPolicy,
  IoArtifactLifecycleState,
  IoArtifactPayloadKind,
  IoArtifactProjectionRecord,
  IoArtifactRetentionState,
  ProjectionStateRecord,
  WorkflowRetentionCleanupResult,
} from '../../services/diagnostics/types';

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

export interface IoRetentionDetailRow {
  label: string;
  value: string;
  mono: boolean;
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
  return rows;
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

export function formatProjectionFreshness(state: ProjectionStateRecord | null): string {
  if (!state) {
    return 'Projection unavailable';
  }
  const cursor = `seq ${state.last_applied_event_seq}`;
  switch (state.status) {
    case 'current':
      return `Current at ${cursor}`;
    case 'rebuilding':
      return `Rebuilding at ${cursor}`;
    case 'needs_rebuild':
      return `Needs rebuild at ${cursor}`;
    case 'failed':
      return `Failed at ${cursor}`;
  }
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

export function formatIoRetentionTimestamp(value: number | null | undefined): string {
  if (!value) {
    return 'Unavailable';
  }
  return new Date(value).toLocaleString();
}
