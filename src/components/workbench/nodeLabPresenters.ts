// Pure projections for the Node Editor (NodeLab) page. Data authority stays in
// nodeLabController.ts and services/workflow/customNodeAuthoring.ts; these
// functions only shape the controller's view for the redesign's layout.
import type { CustomNodeAuthoringEntry, NodeLabDraft } from '../../services/workflow/customNodeAuthoring.ts';
import type { GraphNode } from '../../services/workflow/types.ts';
import type { NodeLabView } from './nodeLabController.ts';
import type { GlyphState } from './ui/statusGlyph.ts';
import type { KeyValueRow } from './ui/types.ts';

export interface NodeLabStatusRow { label: string; value: string }
export function buildNodeLabStatusRows(): NodeLabStatusRow[] {
  return [
    { label: 'Graph authoring', value: 'Host-admitted nodes' },
    { label: 'Local Agent', value: 'Unavailable' },
    { label: 'Runtime Publishing', value: 'Unsupported' },
  ];
}
export function nodeLabTrustMessage(): string {
  return 'These nodes are supplied by this host and run as trusted in-process code. NodeLab creates workflows; it cannot install or publish code.';
}

/** The search field appears only once the list is long enough to need it. */
export const NODE_LAB_SEARCH_MIN_ENTRIES = 6;
export function nodeLabShowsSearch(catalog: CustomNodeAuthoringEntry[] | null): boolean {
  return (catalog?.length ?? 0) >= NODE_LAB_SEARCH_MIN_ENTRIES;
}

function plural(count: number, one: string, many: string): string {
  return `${count} ${count === 1 ? one : many}`;
}

/** Node types of the custom (pinned) nodes in a draft graph. */
function draftCustomNodes(draft: NodeLabDraft | null): GraphNode[] {
  if (!draft) return [];
  return draft.graph.nodes.filter(node => {
    const identity = node.data.custom_node_identity;
    return identity !== null && typeof identity === 'object';
  });
}

function nodeLabel(node: GraphNode): string {
  return typeof node.data.label === 'string' ? node.data.label : node.id;
}

export interface NodeLabCatalogRow {
  entry: CustomNodeAuthoringEntry;
  nodeType: string;
  label: string;
  glyph: GlyphState;
  /** Port summary, the first sub-line. */
  statusLabel: string;
  /** The description, the second sub-line. */
  detail: string;
  /** Right-hand marker: "in draft" for the nodes the open draft pins. */
  time: string;
  title: string;
}
export interface NodeLabCatalogList { rows: NodeLabCatalogRow[]; matched: number; total: number }

/** Filters by label, node type and description; marks the nodes the open draft uses. */
export function buildNodeLabCatalogRows(
  catalog: CustomNodeAuthoringEntry[] | null,
  draft: NodeLabDraft | null,
  query: string,
): NodeLabCatalogList {
  const entries = catalog ?? [];
  const needle = query.trim().toLowerCase();
  const inDraft = new Set(draftCustomNodes(draft).map(node => node.node_type));
  const rows = entries
    .filter(entry => !needle || [entry.label, entry.identity.node_type, entry.description].some(text => text.toLowerCase().includes(needle)))
    .map(entry => {
      const used = inDraft.has(entry.identity.node_type);
      const ports = `${plural(entry.inputs.length, 'string input', 'string inputs')}, ${plural(entry.outputs.length, 'string output', 'string outputs')}`;
      return {
        entry,
        nodeType: entry.identity.node_type,
        label: entry.label,
        glyph: (used ? 'succeeded' : 'idle') as GlyphState,
        statusLabel: ports,
        detail: entry.description,
        time: used ? 'in draft' : '',
        title: `${ports}. Trusted in-process${used ? ', used by the open draft' : ''}. ${entry.description}`,
      };
    });
  return { rows, matched: rows.length, total: entries.length };
}

/**
 * The admitted node the right panel previews: the row the user picked, else
 * the node the open draft uses, else the first one. Resolved against the whole
 * catalog, so a search that hides the row keeps it; a node the catalog no
 * longer lists is dropped.
 */
export function resolveNodeLabSelection(
  catalog: CustomNodeAuthoringEntry[] | null,
  selectedNodeType: string | null,
  draft: NodeLabDraft | null,
): CustomNodeAuthoringEntry | null {
  if (!catalog?.length) return null;
  const byType = (type: string | null | undefined) => (type ? catalog.find(entry => entry.identity.node_type === type) : undefined);
  return byType(selectedNodeType) ?? byType(draftCustomNodes(draft)[0]?.node_type) ?? catalog[0];
}

export function buildNodeLabIdentityRows(entry: CustomNodeAuthoringEntry): KeyValueRow[] {
  const { identity } = entry;
  return [
    { key: 'Node type', value: identity.node_type, mono: true },
    { key: 'Contract version', value: identity.contract_version, mono: true },
    { key: 'Contract digest', value: identity.contract_digest, mono: true },
    { key: 'Implementation', value: identity.implementation_digest, mono: true },
    { key: 'Environment', value: identity.environment_digest, mono: true },
    { key: 'Adapter', value: `v${identity.adapter_version}`, mono: true },
    { key: 'Trust', value: 'Trusted in-process' },
  ];
}

export interface NodeLabDraftSource { node: GraphNode; label: string; text: string }
export interface NodeLabDraftProjection {
  countsText: string;
  customNodes: { id: string; label: string; nodeType: string }[];
  sources: NodeLabDraftSource[];
  outputs: { id: string; label: string }[];
}
export function projectNodeLabDraft(draft: NodeLabDraft): NodeLabDraftProjection {
  return {
    countsText: `${plural(draft.graph.nodes.length, 'node', 'nodes')} · ${plural(draft.graph.edges.length, 'connection', 'connections')}`,
    customNodes: draftCustomNodes(draft).map(node => ({ id: node.id, label: nodeLabel(node), nodeType: node.node_type })),
    sources: draft.graph.nodes
      .filter(node => node.node_type === 'text-input')
      .map(node => ({ node, label: nodeLabel(node), text: typeof node.data.text === 'string' ? node.data.text : '' })),
    outputs: draft.graph.nodes
      .filter(node => node.node_type === 'text-output')
      .map(node => ({ id: node.id, label: nodeLabel(node) })),
  };
}

export interface NodeLabCheck { glyph: GlyphState; text: string }
/** Contract validity of the open draft, in the words #95 uses. */
export function buildNodeLabChecks(draft: NodeLabDraft): NodeLabCheck[] {
  if (!draft.graph_diagnostics.length) return [{ glyph: 'succeeded', text: 'Graph contract valid' }];
  return draft.graph_diagnostics.map(diagnostic => ({
    glyph: diagnostic.blocking_submission ? 'failed' : 'warn',
    text: `${diagnostic.blocking_submission ? 'Blocks execution: ' : ''}${diagnostic.message}`,
  }));
}

/** Page summary for the dimension line: catalog state, then draft state. */
export function nodeLabSummaryLine(view: NodeLabView): string {
  let catalog: string;
  if (view.catalog === null) catalog = view.error && !view.busy ? 'Admitted nodes unavailable' : 'Loading admitted nodes';
  else if (view.catalog.length === 0) catalog = 'No admitted nodes on this host';
  else catalog = plural(view.catalog.length, 'admitted node', 'admitted nodes');
  if (!view.draft) return `${catalog} · no draft open`;
  const blocking = view.draft.graph_diagnostics.filter(item => item.blocking_submission).length;
  const parts = [`draft of ${plural(view.draft.graph.nodes.length, 'node', 'nodes')}`];
  parts.push(blocking ? plural(blocking, 'blocking check', 'blocking checks') : 'contract valid');
  if (view.savedPath) parts.push('saved');
  return `${catalog} · ${parts.join(', ')}`;
}

export interface NodeLabPlaceholder { question: string; sentence: string }
/** Centre panel text while no draft is open. */
export function nodeLabDraftPlaceholder(catalog: CustomNodeAuthoringEntry[] | null): NodeLabPlaceholder {
  if (catalog && catalog.length === 0) {
    return {
      question: 'This host has no admitted custom nodes.',
      sentence: 'A host maintainer must register a trusted implementation before creating a new draft. You can still reopen a saved workflow above.',
    };
  }
  return {
    question: 'No workflow draft is open.',
    sentence: 'Create a draft from an admitted node on the left, or reopen a saved workflow above.',
  };
}

export type NodeLabAction = 'loadCatalog' | 'create' | 'reopen' | 'update' | 'save' | 'retryCleanup';
export type NodeLabRegion = 'catalog' | 'draft' | 'sessions';
/** The controller keeps one error; the page shows it next to the control that caused it. */
export function nodeLabErrorRegion(action: NodeLabAction): NodeLabRegion {
  if (action === 'loadCatalog' || action === 'create') return 'catalog';
  if (action === 'retryCleanup') return 'sessions';
  return 'draft';
}

export function nodeLabCleanupMessage(pending: number): string {
  return `${pending} previous NodeLab sessions could not be closed. Close them before creating or reopening a draft.`;
}
