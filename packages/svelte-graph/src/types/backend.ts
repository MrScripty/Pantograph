// WorkflowBackend interface — transport-agnostic abstraction over graph operations
// Replaces direct Tauri invoke calls with a pluggable backend

import type {
  NodeDefinition,
  WorkflowGraph,
  WorkflowFile,
  WorkflowMetadata,
  WorkflowEvent,
  GraphNode,
  GraphEdge,
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  EdgeInsertionPreviewResponse,
  InsertNodePositionHint,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
} from './workflow.js';
import type { NodeGroup, PortMapping, CreateGroupResult } from './groups.js';

/** A single option for a port dropdown (e.g., model selection) */
export interface PortOption {
  value: string | number | boolean;
  label: string;
  description?: string;
  metadata?: Record<string, unknown>;
}

/** Result from querying port options */
export interface PortOptionsResult {
  options: PortOption[];
  total_count: number;
  searchable: boolean;
}

/** Query parameters for port options */
export interface PortOptionsQuery {
  search?: string;
  limit?: number;
  offset?: number;
}

/** Undo/redo state from the backend */
export interface UndoRedoState {
  canUndo: boolean;
  canRedo: boolean;
  undoCount: number;
}

/**
 * Transport-agnostic interface for workflow graph operations.
 *
 * Pantograph implements this as TauriWorkflowBackend (mapping each method
 * to a Tauri invoke call). Other consumers provide their own implementation
 * (e.g., HTTP, WebSocket, in-memory mock).
 */
export interface WorkflowBackend {
  // --- Node Definitions ---

  /** Get all available node type definitions */
  getNodeDefinitions(): Promise<NodeDefinition[]>;

  /** Validate whether a connection between two port types is allowed */
  validateConnection(sourceType: string, targetType: string): Promise<boolean>;

  // --- Session Management ---

  /** Create an editing session (enables undo/redo). Returns session ID. */
  createSession(graph: WorkflowGraph): Promise<string>;

  /** Run an existing session by demanding outputs from terminal nodes.
   *  Consumers should prefer this over executeWorkflow when an active edit
   *  session already owns the graph being rendered. */
  runSession(sessionId: string): Promise<void>;

  /** Clean up a session when done */
  removeSession(sessionId: string): Promise<void>;

  // --- Execution ---

  /** Execute a workflow from a raw graph snapshot (fire-and-forget, results
   *  come via events). Prefer runSession for normal editor flows. */
  executeWorkflow(graph: WorkflowGraph): Promise<void>;

  // --- Graph Mutation (session-scoped) ---

  /** Add a node. Returns the updated graph for frontend sync. */
  addNode(node: GraphNode, sessionId: string): Promise<WorkflowGraph>;

  /** Remove a node and any attached edges. Returns the updated graph for frontend sync. */
  removeNode(nodeId: string, sessionId: string): Promise<WorkflowGraph>;

  /** Add an edge. Returns the updated graph for frontend sync. */
  addEdge(edge: GraphEdge, sessionId: string): Promise<WorkflowGraph>;

  /** Discover eligible targets and insertable node types for an active source anchor. */
  getConnectionCandidates(
    sourceAnchor: ConnectionAnchor,
    sessionId: string,
    graphRevision?: string,
  ): Promise<ConnectionCandidatesResponse>;

  /** Commit a connection using a revision-aware, backend-owned eligibility check. */
  connectAnchors(
    sourceAnchor: ConnectionAnchor,
    targetAnchor: ConnectionAnchor,
    sessionId: string,
    graphRevision: string,
  ): Promise<ConnectionCommitResponse>;

  /** Insert a compatible node type at a position hint and connect the active source anchor to it. */
  insertNodeAndConnect(
    sourceAnchor: ConnectionAnchor,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
    preferredInputPortId?: string,
  ): Promise<InsertNodeConnectionResponse>;

  /** Preview whether a dragged node type can replace an existing edge with two valid edges. */
  previewNodeInsertOnEdge(
    edgeId: string,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
  ): Promise<EdgeInsertionPreviewResponse>;

  /** Atomically replace an existing edge by inserting a node type between its endpoints. */
  insertNodeOnEdge(
    edgeId: string,
    nodeType: string,
    sessionId: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
  ): Promise<InsertNodeOnEdgeResponse>;

  /** Remove an edge. Returns the updated graph for frontend sync. */
  removeEdge(edgeId: string, sessionId: string): Promise<WorkflowGraph>;

  /** Update a node's data. Returns the updated graph for frontend sync. */
  updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
    sessionId: string,
  ): Promise<WorkflowGraph>;

  /** Update a node's position. Returns the updated graph for frontend sync. */
  updateNodePosition(
    nodeId: string,
    position: { x: number; y: number },
    sessionId: string,
  ): Promise<WorkflowGraph>;

  /** Get the current graph state from a session */
  getExecutionGraph(sessionId: string): Promise<WorkflowGraph>;

  // --- Undo/Redo ---

  /** Get current undo/redo capability state */
  getUndoRedoState(sessionId: string): Promise<UndoRedoState>;

  /** Undo the last graph modification. Returns restored graph. */
  undo(sessionId: string): Promise<WorkflowGraph>;

  /** Redo the last undone modification. Returns restored graph. */
  redo(sessionId: string): Promise<WorkflowGraph>;

  // --- Persistence ---

  /** Save a workflow. Returns the file path. */
  saveWorkflow(name: string, graph: WorkflowGraph): Promise<string>;

  /** Load a workflow from a path */
  loadWorkflow(path: string): Promise<WorkflowFile>;

  /** List all available workflows */
  listWorkflows(): Promise<WorkflowMetadata[]>;

  /** Delete a workflow by name */
  deleteWorkflow(name: string): Promise<void>;

  // --- Node Groups ---

  /** Create a node group from selected nodes */
  createGroup(
    name: string,
    selectedNodeIds: string[],
    graph: WorkflowGraph,
  ): Promise<CreateGroupResult>;

  /** Update port mappings for an existing group */
  updateGroupPorts(
    group: NodeGroup,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
  ): Promise<NodeGroup>;

  // --- Port Options ---

  /** Query available options for a port dropdown (e.g., model selection).
   *  Optional — returns empty result if not implemented. */
  queryPortOptions?(
    nodeType: string,
    portId: string,
    query?: PortOptionsQuery,
  ): Promise<PortOptionsResult>;

  // --- Events ---

  /** Subscribe to workflow execution events. Returns an unsubscribe function. */
  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void;
}
