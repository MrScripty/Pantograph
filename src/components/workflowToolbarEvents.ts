import type { Edge } from '@xyflow/svelte';
import { applyWorkflowExecutionEvent } from '../../packages/svelte-graph/src/stores/workflowExecutionEvents.ts';
import {
  claimWorkflowExecutionIdFromEvent,
  isWorkflowEventRelevantToExecution,
} from '../../packages/svelte-graph/src/workflowEventOwnership.ts';

import {
  buildAudioRuntimeDataFromCompletedOutputs,
} from './nodes/workflow/audioOutputState.ts';
import type { NodeExecutionState, WorkflowEvent } from '../services/workflow/types.ts';

interface WorkflowToolbarStoreActions {
  setNodeExecutionState: (
    nodeId: string,
    state: NodeExecutionState,
    message?: string,
  ) => void;
  updateNodeRuntimeData: (nodeId: string, data: Record<string, unknown>) => void;
  appendStreamContent: (nodeId: string, chunk: string) => void;
  setStreamContent: (nodeId: string, content: string) => void;
}

export interface WorkflowToolbarEventInput {
  event: WorkflowEvent;
  activeExecutionId: string | null;
  waitingForInput: boolean;
  edges: Edge[];
  workflow: WorkflowToolbarStoreActions;
}

export interface WorkflowToolbarEventResult {
  activeExecutionId: string | null;
  waitingForInput: boolean;
  handled: boolean;
  shouldCleanup: boolean;
}

export function applyWorkflowToolbarEvent({
  event,
  activeExecutionId,
  waitingForInput,
  edges,
  workflow,
}: WorkflowToolbarEventInput): WorkflowToolbarEventResult {
  const claimedExecutionId = claimWorkflowExecutionIdFromEvent(event, activeExecutionId);
  if (!isWorkflowEventRelevantToExecution(event, claimedExecutionId)) {
    return {
      activeExecutionId: claimedExecutionId,
      waitingForInput,
      handled: false,
      shouldCleanup: false,
    };
  }

  const result = applyWorkflowExecutionEvent({
    event,
    activeExecutionId: claimedExecutionId,
    waitingForInput,
    edges,
    workflow: {
      setNodeExecutionState: workflow.setNodeExecutionState,
      updateNodeData() {},
    },
  });

  if (!result.handled && event.type !== 'NodeStream') {
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
        workflow.setStreamContent(edge.target, textChunk.text);
      } else {
        workflow.appendStreamContent(edge.target, textChunk.text);
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
      });
      continue;
    }

    workflow.updateNodeRuntimeData(edge.target, {
      [targetHandle]: event.data.chunk,
    });
  }
}

function parseTextStreamChunk(chunk: unknown): { mode: 'append' | 'replace'; text: string } | null {
  if (chunk && typeof chunk === 'object' && 'text' in chunk) {
    const structured = chunk as { mode?: string; text: unknown };
    if (typeof structured.text === 'string') {
      return {
        mode: structured.mode === 'replace' ? 'replace' : 'append',
        text: structured.text,
      };
    }
    return null;
  }

  if (typeof chunk === 'string') {
    return { mode: 'append', text: chunk };
  }

  return null;
}

function parseAudioStreamChunk(chunk: unknown): {
  mode: 'append' | 'replace';
  audioBase64: string;
  mimeType: string;
  sequence: number | null;
  isFinal: boolean;
} | null {
  if (!chunk || typeof chunk !== 'object') return null;
  if (!('audio_base64' in chunk)) return null;
  const structured = chunk as {
    mode?: string;
    audio_base64: unknown;
    mime_type?: unknown;
    sequence?: unknown;
    is_final?: unknown;
  };
  if (typeof structured.audio_base64 !== 'string' || structured.audio_base64.length === 0) {
    return null;
  }
  const sequence =
    typeof structured.sequence === 'number' && Number.isFinite(structured.sequence)
      ? structured.sequence
      : null;
  return {
    mode: structured.mode === 'replace' ? 'replace' : 'append',
    audioBase64: structured.audio_base64,
    mimeType:
      typeof structured.mime_type === 'string' && structured.mime_type.length > 0
        ? structured.mime_type
        : 'audio/wav',
    sequence,
    isFinal: structured.is_final === true,
  };
}
