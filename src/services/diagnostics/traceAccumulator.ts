import type {
  WorkflowCapabilitiesResponse,
  WorkflowEvent,
  WorkflowGraph,
  WorkflowSessionQueueListResponse,
  WorkflowSessionStatusResponse,
} from '../workflow/types';
import type {
  DiagnosticsEventRecord,
  DiagnosticsNodeTrace,
  DiagnosticsRunTrace,
  DiagnosticsRuntimeSnapshot,
  DiagnosticsSchedulerSnapshot,
  DiagnosticsSnapshot,
  DiagnosticsTab,
  DiagnosticsWorkflowContext,
  WorkflowDiagnosticsState,
} from './types';

export const DEFAULT_DIAGNOSTICS_EVENT_LIMIT = 200;
export const DEFAULT_DIAGNOSTICS_TAB: DiagnosticsTab = 'overview';

export function createEmptyDiagnosticsRuntimeSnapshot(): DiagnosticsRuntimeSnapshot {
  return {
    workflowId: null,
    capturedAtMs: null,
    maxInputBindings: null,
    maxOutputTargets: null,
    maxValueBytes: null,
    runtimeRequirements: null,
    runtimeCapabilities: [],
    models: [],
    lastError: null,
  };
}

export function createEmptyDiagnosticsSchedulerSnapshot(): DiagnosticsSchedulerSnapshot {
  return {
    workflowId: null,
    sessionId: null,
    capturedAtMs: null,
    session: null,
    items: [],
    lastError: null,
  };
}

function toExecutionId(event: WorkflowEvent): string | null {
  return event.data.execution_id ?? null;
}

function toWorkflowId(event: WorkflowEvent, context: DiagnosticsWorkflowContext): string | null {
  if ('workflow_id' in event.data && typeof event.data.workflow_id === 'string') {
    return event.data.workflow_id;
  }
  return context.workflowId ?? null;
}

function toNodeId(event: WorkflowEvent): string | null {
  if ('node_id' in event.data && typeof event.data.node_id === 'string') {
    return event.data.node_id;
  }
  return null;
}

function summarizeEvent(event: WorkflowEvent): string {
  switch (event.type) {
    case 'Started':
      return `Workflow started (${event.data.node_count} nodes)`;
    case 'NodeStarted':
      return `Node ${event.data.node_id} started`;
    case 'NodeProgress':
      return event.data.message?.trim() || `Node ${event.data.node_id} progress ${Math.round(event.data.progress * 100)}%`;
    case 'NodeStream':
      return `Node ${event.data.node_id} streamed on ${event.data.port}`;
    case 'NodeCompleted':
      return `Node ${event.data.node_id} completed`;
    case 'NodeError':
      return `Node ${event.data.node_id} failed: ${event.data.error}`;
    case 'Completed':
      return 'Workflow completed';
    case 'Failed':
      return `Workflow failed: ${event.data.error}`;
    case 'GraphModified':
      return event.data.dirty_tasks?.length
        ? `Graph modified; dirty tasks: ${event.data.dirty_tasks.join(', ')}`
        : 'Graph modified';
    case 'WaitingForInput':
      return event.data.message?.trim() || `Waiting for input on ${event.data.node_id}`;
    case 'IncrementalExecutionStarted':
      return event.data.task_ids.length
        ? `Incremental execution for ${event.data.task_ids.join(', ')}`
        : 'Incremental execution started';
  }
}

function createNodeTrace(nodeId: string, nodeType: string | null): DiagnosticsNodeTrace {
  return {
    nodeId,
    nodeType,
    status: 'running',
    startedAtMs: null,
    endedAtMs: null,
    durationMs: null,
    lastProgress: null,
    lastMessage: null,
    streamEventCount: 0,
    eventCount: 0,
    error: null,
  };
}

function createRunTrace(
  executionId: string,
  workflowId: string | null,
  context: DiagnosticsWorkflowContext,
  timestampMs: number,
  nodeCountAtStart: number,
): DiagnosticsRunTrace {
  return {
    executionId,
    workflowId,
    workflowName: context.workflowName,
    graphFingerprintAtStart: context.graphFingerprint,
    nodeCountAtStart,
    status: 'running',
    startedAtMs: timestampMs,
    endedAtMs: null,
    durationMs: null,
    lastUpdatedAtMs: timestampMs,
    error: null,
    waitingForInput: false,
    eventCount: 0,
    streamEventCount: 0,
    lastDirtyTasks: [],
    lastIncrementalTaskIds: [],
    nodes: {},
    events: [],
  };
}

function getNodeType(
  run: DiagnosticsRunTrace,
  context: DiagnosticsWorkflowContext,
  nodeId: string,
  explicitNodeType: string | null,
): string | null {
  if (explicitNodeType && explicitNodeType.trim().length > 0) {
    return explicitNodeType;
  }
  return run.nodes[nodeId]?.nodeType ?? context.nodeTypesById[nodeId] ?? null;
}

function upsertNodeTrace(
  run: DiagnosticsRunTrace,
  context: DiagnosticsWorkflowContext,
  nodeId: string,
  explicitNodeType: string | null,
): DiagnosticsNodeTrace {
  const current = run.nodes[nodeId];
  if (current) {
    if (!current.nodeType) {
      current.nodeType = getNodeType(run, context, nodeId, explicitNodeType);
    }
    return current;
  }

  const next = createNodeTrace(nodeId, getNodeType(run, context, nodeId, explicitNodeType));
  run.nodes[nodeId] = next;
  return next;
}

function pushRunEvent(
  run: DiagnosticsRunTrace,
  eventRecord: DiagnosticsEventRecord,
  retainedEventLimit: number,
): void {
  run.events.push(eventRecord);
  if (run.events.length > retainedEventLimit) {
    run.events.splice(0, run.events.length - retainedEventLimit);
  }
}

function applyNodeLifecycle(
  run: DiagnosticsRunTrace,
  context: DiagnosticsWorkflowContext,
  event: WorkflowEvent,
  timestampMs: number,
): void {
  const nodeId = toNodeId(event);
  if (!nodeId) {
    return;
  }

  const explicitNodeType = 'node_type' in event.data && typeof event.data.node_type === 'string'
    ? event.data.node_type
    : null;
  const node = upsertNodeTrace(run, context, nodeId, explicitNodeType);
  node.eventCount += 1;

  switch (event.type) {
    case 'NodeStarted':
      node.status = 'running';
      node.startedAtMs ??= timestampMs;
      node.endedAtMs = null;
      node.durationMs = null;
      node.error = null;
      node.lastMessage = null;
      node.lastProgress = null;
      break;
    case 'NodeProgress':
      node.status = 'running';
      node.lastProgress = event.data.progress;
      node.lastMessage = event.data.message ?? null;
      break;
    case 'NodeStream':
      node.status = 'running';
      node.streamEventCount += 1;
      break;
    case 'NodeCompleted':
      node.status = 'completed';
      node.endedAtMs = timestampMs;
      node.durationMs = node.startedAtMs === null ? null : timestampMs - node.startedAtMs;
      node.error = null;
      break;
    case 'NodeError':
      node.status = 'failed';
      node.endedAtMs = timestampMs;
      node.durationMs = node.startedAtMs === null ? null : timestampMs - node.startedAtMs;
      node.error = event.data.error;
      break;
    case 'WaitingForInput':
      node.status = 'waiting';
      node.lastMessage = event.data.message ?? 'Waiting for input';
      break;
    default:
      break;
  }
}

function applyRunLifecycle(
  run: DiagnosticsRunTrace,
  event: WorkflowEvent,
  timestampMs: number,
): void {
  run.lastUpdatedAtMs = timestampMs;
  run.eventCount += 1;

  switch (event.type) {
    case 'Started':
      run.status = 'running';
      run.waitingForInput = false;
      break;
    case 'NodeStream':
      run.streamEventCount += 1;
      break;
    case 'WaitingForInput':
      run.status = 'waiting';
      run.waitingForInput = true;
      break;
    case 'Completed':
      run.status = 'completed';
      run.waitingForInput = false;
      run.endedAtMs = timestampMs;
      run.durationMs = timestampMs - run.startedAtMs;
      break;
    case 'Failed':
      run.status = 'failed';
      run.waitingForInput = false;
      run.error = event.data.error;
      run.endedAtMs = timestampMs;
      run.durationMs = timestampMs - run.startedAtMs;
      break;
    case 'GraphModified':
      run.lastDirtyTasks = [...(event.data.dirty_tasks ?? [])];
      break;
    case 'IncrementalExecutionStarted':
      run.lastIncrementalTaskIds = [...event.data.task_ids];
      break;
    default:
      if (run.status === 'waiting' && event.type === 'NodeStarted') {
        run.status = 'running';
        run.waitingForInput = false;
      }
      break;
  }
}

export function createWorkflowDiagnosticsState(
  retainedEventLimit = DEFAULT_DIAGNOSTICS_EVENT_LIMIT,
): WorkflowDiagnosticsState {
  return {
    panelOpen: false,
    activeTab: DEFAULT_DIAGNOSTICS_TAB,
    selectedRunId: null,
    selectedNodeId: null,
    currentSessionId: null,
    currentWorkflowId: null,
    currentWorkflowName: null,
    currentGraphFingerprint: null,
    currentGraphNodeCount: 0,
    currentGraphEdgeCount: 0,
    runsById: {},
    runOrder: [],
    runtime: createEmptyDiagnosticsRuntimeSnapshot(),
    scheduler: createEmptyDiagnosticsSchedulerSnapshot(),
    retainedEventLimit,
  };
}

export function createWorkflowDiagnosticsContext(): DiagnosticsWorkflowContext {
  return {
    workflowId: null,
    workflowName: null,
    graphFingerprint: null,
    graph: null,
    nodeTypesById: {},
  };
}

export function updateWorkflowContext(
  current: DiagnosticsWorkflowContext,
  update: Partial<DiagnosticsWorkflowContext>,
): DiagnosticsWorkflowContext {
  return {
    workflowId: 'workflowId' in update ? update.workflowId ?? null : current.workflowId,
    workflowName: 'workflowName' in update ? update.workflowName ?? null : current.workflowName,
    graphFingerprint: 'graphFingerprint' in update
      ? update.graphFingerprint ?? null
      : current.graphFingerprint,
    graph: 'graph' in update ? update.graph ?? null : current.graph,
    nodeTypesById: 'nodeTypesById' in update ? update.nodeTypesById ?? {} : current.nodeTypesById,
  };
}

export function deriveNodeTypesById(graph: WorkflowGraph | null): Record<string, string> {
  if (!graph) {
    return {};
  }

  return Object.fromEntries(graph.nodes.map(node => [node.id, node.node_type]));
}

export function recordWorkflowEvent(
  state: WorkflowDiagnosticsState,
  context: DiagnosticsWorkflowContext,
  event: WorkflowEvent,
  timestampMs: number,
): void {
  const executionId = toExecutionId(event);
  if (!executionId) {
    return;
  }

  const workflowId = toWorkflowId(event, context);
  const existingRun = state.runsById[executionId];
  const run = existingRun ?? createRunTrace(
    executionId,
    workflowId,
    context,
    timestampMs,
    event.type === 'Started' ? event.data.node_count : context.graph?.nodes.length ?? 0,
  );

  if (!existingRun) {
    state.runsById[executionId] = run;
    state.runOrder = [executionId, ...state.runOrder.filter(id => id !== executionId)];
    state.selectedRunId ??= executionId;
  } else {
    state.runOrder = [executionId, ...state.runOrder.filter(id => id !== executionId)];
  }

  if (!run.workflowId) {
    run.workflowId = workflowId;
  }
  if (!run.workflowName && context.workflowName) {
    run.workflowName = context.workflowName;
  }
  if (!run.graphFingerprintAtStart && context.graphFingerprint) {
    run.graphFingerprintAtStart = context.graphFingerprint;
  }

  applyRunLifecycle(run, event, timestampMs);
  applyNodeLifecycle(run, context, event, timestampMs);

  const eventRecord: DiagnosticsEventRecord = {
    id: `${executionId}-${run.eventCount}`,
    sequence: run.eventCount,
    timestampMs,
    type: event.type,
    executionId,
    workflowId,
    nodeId: toNodeId(event),
    summary: summarizeEvent(event),
    payload: event.data as DiagnosticsEventRecord['payload'],
  };

  pushRunEvent(run, eventRecord, state.retainedEventLimit);
}

export function setDiagnosticsPanelOpen(
  state: WorkflowDiagnosticsState,
  panelOpen: boolean,
): WorkflowDiagnosticsState {
  return {
    ...state,
    panelOpen,
  };
}

export function setDiagnosticsCurrentSessionId(
  state: WorkflowDiagnosticsState,
  sessionId: string | null,
): WorkflowDiagnosticsState {
  return {
    ...state,
    currentSessionId: sessionId,
  };
}

export function setDiagnosticsTab(
  state: WorkflowDiagnosticsState,
  activeTab: DiagnosticsTab,
): WorkflowDiagnosticsState {
  return {
    ...state,
    activeTab,
  };
}

export function selectDiagnosticsRun(
  state: WorkflowDiagnosticsState,
  selectedRunId: string | null,
): WorkflowDiagnosticsState {
  return {
    ...state,
    selectedRunId,
    selectedNodeId: null,
  };
}

export function selectDiagnosticsNode(
  state: WorkflowDiagnosticsState,
  selectedNodeId: string | null,
): WorkflowDiagnosticsState {
  return {
    ...state,
    selectedNodeId,
  };
}

export function updateDiagnosticsStateWorkflowContext(
  state: WorkflowDiagnosticsState,
  context: DiagnosticsWorkflowContext,
): WorkflowDiagnosticsState {
  return {
    ...state,
    currentWorkflowId: context.workflowId,
    currentWorkflowName: context.workflowName,
    currentGraphFingerprint: context.graphFingerprint,
    currentGraphNodeCount: context.graph?.nodes.length ?? 0,
    currentGraphEdgeCount: context.graph?.edges.length ?? 0,
  };
}

export function updateDiagnosticsStateRuntimeSnapshot(
  state: WorkflowDiagnosticsState,
  workflowId: string | null,
  capabilities: WorkflowCapabilitiesResponse | null,
  capturedAtMs: number,
  lastError: string | null,
): WorkflowDiagnosticsState {
  if (!workflowId) {
    return {
      ...state,
      runtime: createEmptyDiagnosticsRuntimeSnapshot(),
    };
  }

  return {
    ...state,
    runtime: {
      workflowId,
      capturedAtMs,
      maxInputBindings: capabilities?.max_input_bindings ?? null,
      maxOutputTargets: capabilities?.max_output_targets ?? null,
      maxValueBytes: capabilities?.max_value_bytes ?? null,
      runtimeRequirements: capabilities?.runtime_requirements ?? null,
      runtimeCapabilities: capabilities?.runtime_capabilities ?? [],
      models: capabilities?.models ?? [],
      lastError,
    },
  };
}

export function updateDiagnosticsStateSchedulerSnapshot(
  state: WorkflowDiagnosticsState,
  workflowId: string | null,
  sessionId: string | null,
  sessionStatus: WorkflowSessionStatusResponse | null,
  sessionQueue: WorkflowSessionQueueListResponse | null,
  capturedAtMs: number,
  lastError: string | null,
): WorkflowDiagnosticsState {
  if (!sessionId) {
    return {
      ...state,
      scheduler: createEmptyDiagnosticsSchedulerSnapshot(),
    };
  }

  return {
    ...state,
    scheduler: {
      workflowId,
      sessionId,
      capturedAtMs,
      session: sessionStatus?.session ?? null,
      items: sessionQueue?.items ?? [],
      lastError,
    },
  };
}

export function clearDiagnosticsHistory(
  state: WorkflowDiagnosticsState,
): WorkflowDiagnosticsState {
  return {
    ...state,
    selectedRunId: null,
    selectedNodeId: null,
    runsById: {},
    runOrder: [],
  };
}

export function createDiagnosticsSnapshot(
  state: WorkflowDiagnosticsState,
): DiagnosticsSnapshot {
  const selectedRun = state.selectedRunId ? state.runsById[state.selectedRunId] ?? null : null;
  const selectedNode = selectedRun && state.selectedNodeId
    ? selectedRun.nodes[state.selectedNodeId] ?? null
    : null;

  return {
    state,
    selectedRun,
    selectedNode,
  };
}
