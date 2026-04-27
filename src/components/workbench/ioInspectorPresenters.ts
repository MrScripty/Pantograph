import type {
  IoArtifactProjectionRecord,
  IoArtifactRetentionState,
  ProjectionStateRecord,
} from '../../services/diagnostics/types';

export type IoArtifactMediaFamily =
  | 'text'
  | 'image'
  | 'audio'
  | 'video'
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

export function buildIoArtifactNodeGroups(
  artifacts: Pick<IoArtifactProjectionRecord, 'node_id' | 'node_type' | 'artifact_role' | 'event_seq'>[],
): IoArtifactNodeGroup[] {
  const groups = new Map<string, IoArtifactNodeGroup>();
  for (const artifact of artifacts) {
    if (!artifact.node_id) {
      continue;
    }

    const group = groups.get(artifact.node_id) ?? {
      node_id: artifact.node_id,
      node_type: artifact.node_type,
      input_count: 0,
      output_count: 0,
      artifact_count: 0,
      latest_event_seq: artifact.event_seq,
    };
    group.artifact_count += 1;
    group.latest_event_seq = Math.max(group.latest_event_seq, artifact.event_seq);
    if (artifact.artifact_role === 'node_input') {
      group.input_count += 1;
    }
    if (artifact.artifact_role === 'node_output') {
      group.output_count += 1;
    }
    if (!group.node_type && artifact.node_type) {
      group.node_type = artifact.node_type;
    }
    groups.set(artifact.node_id, group);
  }

  return [...groups.values()].sort((left, right) => right.latest_event_seq - left.latest_event_seq);
}

export function classifyIoArtifactMedia(mediaType: string | null | undefined): IoArtifactMediaFamily {
  if (!mediaType) {
    return 'unknown';
  }
  const normalized = mediaType.toLowerCase();
  if (normalized.includes('csv') || normalized.includes('parquet') || normalized.includes('table')) {
    return 'table';
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

export function formatIoArtifactMediaLabel(mediaType: string | null | undefined): string {
  switch (classifyIoArtifactMedia(mediaType)) {
    case 'text':
      return 'Text';
    case 'image':
      return 'Image';
    case 'audio':
      return 'Audio';
    case 'video':
      return 'Video';
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
    Partial<Pick<IoArtifactProjectionRecord, 'retention_state'>>,
): IoArtifactRendererSummary {
  const family = classifyIoArtifactMedia(artifact.media_type);
  const detail = formatIoArtifactRetentionStateLabel(artifact.retention_state);

  switch (family) {
    case 'text':
      return { family, title: 'Text', detail };
    case 'image':
      return { family, title: 'Image preview', detail };
    case 'audio':
      return { family, title: 'Audio', detail };
    case 'video':
      return { family, title: 'Video', detail };
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
    Partial<Pick<IoArtifactProjectionRecord, 'retention_state'>>,
): IoArtifactPayloadAvailability {
  if (
    artifact.retention_state === 'metadata_only' ||
    artifact.retention_state === 'expired' ||
    artifact.retention_state === 'deleted' ||
    artifact.retention_state === 'too_large'
  ) {
    return 'metadata_only';
  }
  return artifact.payload_ref && artifact.payload_ref.trim().length > 0
    ? 'referenced'
    : 'metadata_only';
}

export function formatIoArtifactAvailabilityLabel(
  artifact: Pick<IoArtifactProjectionRecord, 'payload_ref'> &
    Partial<Pick<IoArtifactProjectionRecord, 'retention_state'>>,
): string {
  switch (resolveIoArtifactPayloadAvailability(artifact)) {
    case 'referenced':
      return 'Payload referenced';
    case 'metadata_only':
      return 'Metadata only';
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
