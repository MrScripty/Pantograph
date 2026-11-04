// Wire authority: custom_nodes/authoring.rs (catalog v1) and graph/session_contract.rs.
import type { GraphNode, WorkflowGraph } from './types.ts';
import { invokeWorkflowCommand } from './workflowServiceErrors.ts';

export interface CustomNodeIdentity {
  node_type: string;
  contract_version: string;
  contract_digest: string;
  implementation_digest: string;
  environment_digest: string;
  adapter_version: number;
}
export interface CustomNodeAuthoringEntry {
  identity: CustomNodeIdentity;
  label: string;
  description: string;
  trust: 'trusted_in_process';
  inputs: { id: string; label: string }[];
  outputs: { id: string; label: string }[];
}
export interface NodeLabDraft {
  session_id: string;
  graph_revision: string;
  graph: WorkflowGraph;
  graph_diagnostics: { message: string; blocking_submission: boolean }[];
}
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid NodeLab object');
  return value as Record<string, unknown>;
}
function exact(value: unknown, keys: string[]): Record<string, unknown> {
  const item = record(value);
  if (Object.keys(item).length !== keys.length || keys.some(key => !(key in item))) throw new Error('Invalid NodeLab fields');
  return item;
}
function text(value: unknown): string {
  if (typeof value !== 'string') throw new Error('Invalid NodeLab text');
  return value;
}
function identifier(value: unknown): string {
  const id = text(value);
  if (!id.trim()) throw new Error('Invalid NodeLab identifier');
  return id;
}
function contractId(value: unknown): string {
  const id = identifier(value);
  // NodeTypeId/PortId grammar in pantograph-node-contracts; do not normalize pins.
  if (id.length > 128 || !/^[A-Za-z0-9_.:-]+$/.test(id)) throw new Error('Invalid custom-node contract identifier');
  return id;
}
export function parseCustomNodeIdentity(value: unknown): CustomNodeIdentity {
  const item = exact(value, ['node_type', 'contract_version', 'contract_digest', 'implementation_digest', 'environment_digest', 'adapter_version']);
  if (item.adapter_version !== 1) throw new Error('Unsupported custom-node adapter version');
  for (const key of ['implementation_digest', 'environment_digest']) {
    if (!/^blake3:[0-9a-f]{64}$/.test(text(item[key]))) throw new Error('Invalid custom-node attestation');
  }
  if (!/^[0-9]+\.[0-9]+\.[0-9]+$/.test(text(item.contract_version)) || !/^contract-blake3:[0-9a-f]{64}$/.test(text(item.contract_digest))) throw new Error('Invalid custom-node contract identity');
  return {
    node_type: contractId(item.node_type), contract_version: text(item.contract_version),
    contract_digest: identifier(item.contract_digest), implementation_digest: text(item.implementation_digest),
    environment_digest: text(item.environment_digest), adapter_version: 1,
  };
}
export function parseCustomNodeCatalog(value: unknown): CustomNodeAuthoringEntry[] {
  const catalog = exact(value, ['contract_version', 'nodes']);
  if (catalog.contract_version !== 1) throw new Error('Unsupported NodeLab catalog version');
  if (!Array.isArray(catalog.nodes)) throw new Error('Invalid NodeLab catalog');
  const seen = new Set<string>();
  return catalog.nodes.map(value => {
    const item = exact(value, ['identity', 'label', 'description', 'trust', 'inputs', 'outputs']);
    const identity = parseCustomNodeIdentity(item.identity);
    if (seen.has(identity.node_type)) throw new Error('Duplicate NodeLab type');
    seen.add(identity.node_type);
    if (item.trust !== 'trusted_in_process') throw new Error('Unsupported custom-node trust profile');
    const ports = (value: unknown) => {
      if (!Array.isArray(value) || value.length < 1 || value.length > 8) throw new Error('Invalid NodeLab ports');
      const seen = new Set<string>();
      return value.map(value => {
        const port = exact(value, ['id', 'label']);
        const id = contractId(port.id);
        if (id.startsWith('_')) throw new Error('Invalid reserved custom-node port');
        if (seen.has(id)) throw new Error('Duplicate NodeLab port');
        seen.add(id);
        return { id, label: text(port.label) };
      });
    };
    return { identity, label: text(item.label), description: text(item.description), trust: 'trusted_in_process', inputs: ports(item.inputs), outputs: ports(item.outputs) };
  });
}
function parseGraph(value: unknown): WorkflowGraph {
  const graph = record(value);
  if (!Array.isArray(graph.nodes) || !Array.isArray(graph.edges)) throw new Error('Invalid NodeLab graph');
  const nodes = graph.nodes.map(value => {
    const node = record(value);
    const position = record(node.position);
    if (typeof position.x !== 'number' || !Number.isFinite(position.x) || typeof position.y !== 'number' || !Number.isFinite(position.y)) throw new Error('Invalid NodeLab position');
    return { id: identifier(node.id), node_type: identifier(node.node_type), position: { x: position.x, y: position.y }, data: record(node.data) };
  });
  const edges = graph.edges.map(value => {
    const edge = record(value);
    return { id: identifier(edge.id), source: identifier(edge.source), source_handle: identifier(edge.source_handle), target: identifier(edge.target), target_handle: identifier(edge.target_handle) };
  });
  return { nodes, edges };
}
export function parseNodeLabDraft(value: unknown): NodeLabDraft {
  const draft = record(value);
  const diagnostics = draft.graph_diagnostics === undefined ? [] : draft.graph_diagnostics;
  if (!Array.isArray(diagnostics)) throw new Error('Invalid NodeLab diagnostics');
  return {
    session_id: identifier(draft.session_id), graph_revision: identifier(draft.graph_revision), graph: parseGraph(draft.graph),
    graph_diagnostics: diagnostics.map(value => {
      const item = record(value);
      if (typeof item.blocking_submission !== 'boolean') throw new Error('Invalid NodeLab diagnostic');
      return { message: text(item.message), blocking_submission: item.blocking_submission };
    }),
  };
}

export class NodeLabService {
  // App-scoped recovery ownership for transport/close failures, never an execution registry.
  private readonly pendingCloses = new Set<string>();
  get pendingCleanupCount(): number { return this.pendingCloses.size; }
  // A failed replacement keeps the old draft under its still-active controller.
  retainActiveSession(id: string): void { this.pendingCloses.delete(id); }
  async retryCleanup(): Promise<void> {
    const errors: unknown[] = [];
    for (const id of [...this.pendingCloses]) { try { await this.close(id); } catch (error) { errors.push(error); } }
    if (errors.length) throw new AggregateError(errors, 'Some NodeLab sessions could not be closed');
  }
  private requireCleanupResolved(): void {
    if (this.pendingCloses.size) throw new Error('Close previous NodeLab sessions before creating or reopening a draft');
  }
  private readonly invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
  constructor(invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown> = invokeWorkflowCommand) { this.invoke = invoke; }
  async catalog(): Promise<CustomNodeAuthoringEntry[]> {
    return parseCustomNodeCatalog(await this.invoke('get_custom_node_authoring_catalog'));
  }
  private async ownedDraft(response: unknown): Promise<NodeLabDraft> {
    const id = identifier(record(response).session_id);
    try { return parseNodeLabDraft(response); }
    catch (error) { await this.close(id); throw error; }
  }
  async create(identity: CustomNodeIdentity): Promise<NodeLabDraft> {
    this.requireCleanupResolved();
    return this.ownedDraft(await this.invoke('create_custom_node_draft', { request: { identity } }));
  }
  async reopen(path: string): Promise<NodeLabDraft> {
    this.requireCleanupResolved();
    const saved = record(await this.invoke('load_workflow', { path }));
    const graph = parseGraph(saved.graph);
    const session = record(await this.invoke('create_workflow_execution_session', { graph, workflowId: path }));
    const id = identifier(session.session_id);
    try { return await this.snapshot(id); }
    catch (error) { await this.close(id); throw error; }
  }
  async snapshot(id: string): Promise<NodeLabDraft> {
    const draft = parseNodeLabDraft(await this.invoke('get_node_lab_draft', { executionId: id }));
    if (draft.session_id !== id) throw new Error('NodeLab session correlation mismatch');
    return draft;
  }
  async update(id: string, node: GraphNode, value: string): Promise<NodeLabDraft> {
    // Source values are authored text; custom identities/definitions never use this path.
    if (node.node_type !== 'text-input') throw new Error('NodeLab only edits text sources');
    const draft = parseNodeLabDraft(await this.invoke('update_node_data', { executionId: id, nodeId: node.id, data: { text: value } }));
    if (draft.session_id !== id) throw new Error('NodeLab session correlation mismatch');
    return draft;
  }
  async save(id: string, name: string): Promise<{ draft: NodeLabDraft; path: string }> {
    const draft = await this.snapshot(id);
    const path = identifier(await this.invoke('save_workflow', { name, graph: draft.graph }));
    return { draft, path };
  }
  async close(id: string): Promise<void> {
    try { await this.invoke('remove_execution', { executionId: id }); this.pendingCloses.delete(id); }
    catch (error) {
      // Already absent is a terminal close outcome; other failures retain exact ownership.
      if (error && typeof error === 'object' && 'code' in error && error.code === 'session_not_found') { this.pendingCloses.delete(id); return; }
      this.pendingCloses.add(id); throw error;
    }
  }
}

// Workbench lifetime owner: failed closes survive NodeLab page navigation.
export const nodeLabService = new NodeLabService();
