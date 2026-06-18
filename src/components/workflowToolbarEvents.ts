import type { Edge } from '@xyflow/svelte';
import { applyWorkflowExecutionEvent } from '../../packages/svelte-graph/src/stores/workflowExecutionEvents.ts';

import {
  buildAudioRuntimeDataFromCompletedOutputs,
} from './nodes/workflow/audioOutputState.ts';
import type {
  NodeExecutionState,
  WorkflowEvent,
  WorkflowGraphValidationLifecycleEvent,
  WorkflowGraphValidationSubmitGate,
  WorkflowPortBinding,
  WorkflowRunResponse,
} from '../services/workflow/types.ts';

interface WorkflowToolbarStoreActions {
  setNodeExecutionState: (
    nodeId: string,
    state: NodeExecutionState,
    message?: string,
  ) => void;
  updateNodeRuntimeData: (nodeId: string, data: Record<string, unknown>) => void;
  appendStreamContent: (nodeId: string, chunk: string, sequence?: number | null) => void;
  setStreamContent: (nodeId: string, content: string, sequence?: number | null) => void;
}

export interface WorkflowToolbarEventInput {
  event: WorkflowEvent;
  activeWorkflowRunId: string | null;
  waitingForInput: boolean;
  edges: Edge[];
  workflow: WorkflowToolbarStoreActions;
}

export interface WorkflowToolbarEventResult {
  activeWorkflowRunId: string | null;
  waitingForInput: boolean;
  handled: boolean;
  shouldCleanup: boolean;
}

export interface WorkflowSubmitFailureContext {
  submittedWorkflowId: string | null | undefined;
  currentGraphId: string | null | undefined;
  currentGraphType: string | null | undefined;
}

export interface WorkflowSubmitDisabledReasonInput {
  isExecuting: boolean;
  isReadOnly: boolean;
  isDirty: boolean;
  hasSavedWorkflow: boolean;
  hasWorkflowId: boolean;
  semanticVersionInvalid: boolean;
  submitGate?: WorkflowGraphValidationSubmitGate | null;
}

export interface WorkflowValidationRefreshKeyInput {
  currentGraphType: string | null | undefined;
  graphSessionId: string | null | undefined;
  graphRevision: string | null | undefined;
}

export interface WorkflowValidationLifecycleRefreshInput extends WorkflowValidationRefreshKeyInput {
  event: WorkflowGraphValidationLifecycleEvent;
  currentValidationSummaryKey: string | null | undefined;
}

type WorkflowSubmitSuccessWorkbenchPage = 'scheduler' | 'io_inspector';

export function isNumericWorkflowSemanticVersion(version: string): boolean {
  const parts = version.split('.');
  return (
    parts.length === 3 &&
    parts.every((part) => part.length > 0 && [...part].every((character) => character >= '0' && character <= '9'))
  );
}

export function nextWorkflowPatchSemanticVersion(version: string): string {
  if (!isNumericWorkflowSemanticVersion(version)) {
    return '0.1.0';
  }

  const [major, minor, patch] = version.split('.').map((part) => Number.parseInt(part, 10));
  return `${major}.${minor}.${patch + 1}`;
}

export function workflowSubmitDisabledReason({
  isExecuting,
  isReadOnly,
  isDirty,
  hasSavedWorkflow,
  hasWorkflowId,
  semanticVersionInvalid,
  submitGate = null,
}: WorkflowSubmitDisabledReasonInput): string | null {
  if (isReadOnly) return 'Cannot submit a read-only graph';
  if (isDirty) return 'Save workflow changes before submitting';
  if (!hasSavedWorkflow || !hasWorkflowId) return 'Save the workflow before submitting';
  if (semanticVersionInvalid) return 'Workflow version must use numeric major.minor.patch format';
  if (!submitGate) return 'Workflow validation summary unavailable';
  if (!submitGate.allowed) return submitGate.message ?? 'Workflow validation does not allow submit';
  if (isExecuting) return 'Workflow submission is in progress';
  return null;
}

export function workflowValidationRefreshKey({
  currentGraphType,
  graphSessionId,
  graphRevision,
}: WorkflowValidationRefreshKeyInput): string | null {
  if (currentGraphType !== 'workflow' || !graphSessionId || !graphRevision) {
    return null;
  }

  return `${graphSessionId}:${graphRevision}`;
}

export function shouldRefreshValidationFromLifecycleEvent({
  event,
  currentGraphType,
  graphSessionId,
  graphRevision,
  currentValidationSummaryKey,
}: WorkflowValidationLifecycleRefreshInput): boolean {
  const refreshKey = workflowValidationRefreshKey({
    currentGraphType,
    graphSessionId,
    graphRevision,
  });
  return Boolean(
    refreshKey &&
      currentValidationSummaryKey === refreshKey &&
      event.graph_session_id === graphSessionId &&
      event.graph_revision === graphRevision,
  );
}

export function isWorkflowSemanticVersionConflictError(error: unknown): boolean {
  const candidate = error as {
    code?: unknown;
    message?: unknown;
    details?: unknown;
    backendEnvelope?: { message?: unknown } | null;
  };
  const message = typeof candidate.message === 'string'
    ? candidate.message
    : typeof candidate.backendEnvelope?.message === 'string'
      ? candidate.backendEnvelope.message
      : '';

  return (
    (candidate.code === 'invalid_request' || candidate.code === 'internal_error') &&
    message.toLowerCase().includes('workflow semantic version already points')
  );
}

export function isCurrentWorkflowSubmitFailure({
  submittedWorkflowId,
  currentGraphId,
  currentGraphType,
}: WorkflowSubmitFailureContext): boolean {
  return Boolean(
    submittedWorkflowId &&
      currentGraphId &&
      submittedWorkflowId === currentGraphId &&
      currentGraphType === 'workflow',
  );
}

export function workflowSubmitSuccessWorkbenchPage(
  response: Pick<WorkflowRunResponse, 'outputs'>,
): WorkflowSubmitSuccessWorkbenchPage {
  return response.outputs.some(isImageArtifactOutputBinding) ? 'io_inspector' : 'scheduler';
}

export function applyWorkflowToolbarEvent({
  event,
  activeWorkflowRunId,
  waitingForInput,
  edges,
  workflow,
}: WorkflowToolbarEventInput): WorkflowToolbarEventResult {
  const result = applyWorkflowExecutionEvent({
    event,
    activeWorkflowRunId,
    waitingForInput,
    edges,
    workflow: {
      setNodeExecutionState: workflow.setNodeExecutionState,
      updateNodeData() {},
    },
  });

  if (!result.handled) {
    return result;
  }

  switch (event.type) {
    case 'NodeCompleted':
      applyCompletedNodeRuntimeData(event, edges, workflow);
      break;
    case 'NodeStream':
      applyStreamNodeRuntimeData(event, edges, workflow);
      break;
  }

  return {
    ...result,
    handled: true,
  };
}

function isImageArtifactOutputBinding(binding: WorkflowPortBinding): boolean {
  if (!binding.value) {
    return false;
  }

  const portId = binding.port_id.toLowerCase();
  if (portId === 'image' || portId.endsWith('_image') || portId.includes('image_')) {
    return true;
  }

  if (typeof binding.value !== 'object' || Array.isArray(binding.value)) {
    return false;
  }

  const record = binding.value as Record<string, unknown>;
  const payloadKind = nonEmptyStringOrNull(record.payload_kind)?.toLowerCase();
  if (payloadKind === 'image') {
    return true;
  }

  const mediaType = artifactMediaType(record);
  if (mediaType?.toLowerCase().startsWith('image/')) {
    return true;
  }

  return (
    (typeof record.artifact_id === 'string' || typeof record.payload_artifact_id === 'string') &&
    portId.includes('image')
  );
}

function artifactMediaType(record: Record<string, unknown>): string | null {
  const direct = nonEmptyStringOrNull(record.media_type);
  if (direct) {
    return direct;
  }

  const format = record.format;
  if (!format || typeof format !== 'object' || Array.isArray(format)) {
    return null;
  }

  return nonEmptyStringOrNull((format as Record<string, unknown>).media_type);
}

function applyCompletedNodeRuntimeData(
  event: WorkflowEvent<'NodeCompleted'>,
  edges: Edge[],
  workflow: WorkflowToolbarStoreActions,
) {
  const completedNodeRuntimeData = {
    ...event.data.outputs,
    ...(buildAudioRuntimeDataFromCompletedOutputs('audio', 'audio', event.data.outputs) ?? {}),
  };
  workflow.updateNodeRuntimeData(event.data.node_id, completedNodeRuntimeData);

  const outgoingEdges = edges.filter((edge) => edge.source === event.data.node_id);
  for (const edge of outgoingEdges) {
    const sourceHandle = edge.sourceHandle || '';
    const outputValue = event.data.outputs[sourceHandle];
    if (typeof outputValue === 'undefined') {
      continue;
    }

    const targetHandle = edge.targetHandle || '';
    workflow.updateNodeRuntimeData(edge.target, {
      [targetHandle]: outputValue,
      ...(buildAudioRuntimeDataFromCompletedOutputs(
        sourceHandle,
        targetHandle,
        event.data.outputs,
      ) ?? {}),
    });
  }
}

function applyStreamNodeRuntimeData(
  event: WorkflowEvent<'NodeStream'>,
  edges: Edge[],
  workflow: WorkflowToolbarStoreActions,
) {
  const textChunk = parseTextStreamChunk(event.data.chunk);
  const audioChunk = parseAudioStreamChunk(event.data.chunk);
  const outgoingEdges = edges.filter(
    (edge) => edge.source === event.data.node_id && edge.sourceHandle === event.data.port,
  );

  for (const edge of outgoingEdges) {
    if (textChunk) {
      if (textChunk.mode === 'replace') {
        workflow.setStreamContent(edge.target, textChunk.text, textChunk.sequence);
      } else {
        workflow.appendStreamContent(edge.target, textChunk.text, textChunk.sequence);
      }
      continue;
    }

    const targetHandle = edge.targetHandle || 'stream';
    if (audioChunk && targetHandle === 'stream') {
      workflow.updateNodeRuntimeData(edge.target, {
        [targetHandle]: event.data.chunk,
        audio_mime: audioChunk.mimeType,
        stream_sequence: audioChunk.sequence,
        stream_is_final: audioChunk.isFinal,
        stream_artifact_id: audioChunk.artifactId,
        stream_handle: audioChunk.streamHandle,
        stream_byte_length: audioChunk.byteLength,
        stream_available_byte_length: audioChunk.availableByteLength,
        stream_byte_range_start: audioChunk.byteRangeStart,
        stream_byte_range_end_exclusive: audioChunk.byteRangeEndExclusive,
        stream_lifecycle_state: audioChunk.lifecycleState,
        stream_descriptor: audioChunk.descriptor,
      });
      continue;
    }

    workflow.updateNodeRuntimeData(edge.target, {
      [targetHandle]: event.data.chunk,
    });
  }
}

function parseTextStreamChunk(
  chunk: unknown,
): { mode: 'append' | 'replace'; text: string; sequence: number | null } | null {
  if (chunk && typeof chunk === 'object' && 'text' in chunk) {
    const structured = chunk as { mode?: string; text: unknown; sequence?: unknown };
    if (typeof structured.text === 'string') {
      return {
        mode: structured.mode === 'replace' ? 'replace' : 'append',
        text: structured.text,
        sequence: finiteNumberOrNull(structured.sequence),
      };
    }
    return null;
  }

  if (typeof chunk === 'string') {
    return { mode: 'append', text: chunk, sequence: null };
  }

  return null;
}

function parseAudioStreamChunk(chunk: unknown): {
  mode: 'append' | 'replace';
  audioBase64: string | null;
  artifactId: string | null;
  streamHandle: string | null;
  mimeType: string;
  sequence: number | null;
  byteLength: number | null;
  availableByteLength: number | null;
  byteRangeStart: number | null;
  byteRangeEndExclusive: number | null;
  lifecycleState: string | null;
  isFinal: boolean;
  descriptor: unknown;
} | null {
  if (!chunk || typeof chunk !== 'object') return null;
  if (
    !('audio_base64' in chunk) &&
    !('artifact_id' in chunk || 'stream_handle' in chunk || 'descriptor' in chunk)
  ) {
    return null;
  }
  const structured = chunk as {
    mode?: string;
    audio_base64?: unknown;
    artifact_id?: unknown;
    stream_handle?: unknown;
    media_type?: unknown;
    mime_type?: unknown;
    sequence?: unknown;
    byte_length?: unknown;
    available_byte_length?: unknown;
    byte_range_start?: unknown;
    byte_range_end_exclusive?: unknown;
    lifecycle_state?: unknown;
    is_final?: unknown;
    descriptor?: unknown;
  };
  const descriptor =
    structured.descriptor && typeof structured.descriptor === 'object'
      ? (structured.descriptor as Record<string, unknown>)
      : null;
  const descriptorFormat =
    descriptor?.format && typeof descriptor.format === 'object'
      ? (descriptor.format as Record<string, unknown>)
      : null;

  const audioBase64 =
    typeof structured.audio_base64 === 'string' && structured.audio_base64.length > 0
      ? structured.audio_base64
      : null;
  const artifactId = nonEmptyStringOrNull(structured.artifact_id) ?? nonEmptyStringOrNull(descriptor?.artifact_id);
  const streamHandle =
    nonEmptyStringOrNull(structured.stream_handle) ?? nonEmptyStringOrNull(descriptor?.stream_handle);

  if (!audioBase64 && !artifactId && !streamHandle) {
    return null;
  }

  const sequence =
    typeof structured.sequence === 'number' && Number.isFinite(structured.sequence)
      ? structured.sequence
      : null;
  const mediaType = nonEmptyStringOrNull(structured.media_type) ?? nonEmptyStringOrNull(descriptorFormat?.media_type);
  const mimeType = nonEmptyStringOrNull(structured.mime_type) ?? mediaType;
  return {
    mode: structured.mode === 'replace' ? 'replace' : 'append',
    audioBase64,
    artifactId,
    streamHandle,
    mimeType: mimeType ?? 'audio/wav',
    sequence,
    byteLength: finiteNumberOrNull(structured.byte_length) ?? finiteNumberOrNull(descriptor?.byte_length),
    availableByteLength: finiteNumberOrNull(structured.available_byte_length),
    byteRangeStart: finiteNumberOrNull(structured.byte_range_start),
    byteRangeEndExclusive: finiteNumberOrNull(structured.byte_range_end_exclusive),
    lifecycleState:
      nonEmptyStringOrNull(structured.lifecycle_state) ?? nonEmptyStringOrNull(descriptor?.lifecycle_state),
    isFinal: structured.is_final === true,
    descriptor: structured.descriptor,
  };
}

function finiteNumberOrNull(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function nonEmptyStringOrNull(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}
