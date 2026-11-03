import type {
  WorkflowGraph,
  WorkflowGraphDiagnostic,
  WorkflowGraphInspectionProjection,
  WorkflowMetadata,
} from '../../services/workflow/types';
import { buildRunGraphCanvasModel, type RunGraphCanvasModel } from './runGraphPresenters.ts';
import type { GlyphState } from './ui/statusGlyph.ts';

const SAVED_WORKFLOW_INSPECTION_ROOT = '.pantograph/workflows';
const WORKFLOW_FILE_EXTENSION = '.json';

export interface SavedGraphInspectionOption {
  workflowId: string;
  label: string;
  modified: string;
  inspectionPath: string;
}

export interface SavedGraphInspectionDisplayModel {
  canvas: GraphInspectionCanvasModel;
  diagnostics: WorkflowGraphDiagnostic[];
  selectedNodeId: string | null;
  selectedNodeDiagnostics: WorkflowGraphDiagnostic[];
  artifactControlsEnabled: false;
  hasRunContext: false;
}

export type GraphInspectionCanvasModel = RunGraphCanvasModel;
export type GraphInspectionCanvasNode = GraphInspectionCanvasModel['nodes'][number];

export function buildSavedGraphInspectionOptions(
  workflows: WorkflowMetadata[],
): SavedGraphInspectionOption[] {
  return workflows.flatMap((workflow) => {
    const workflowId = workflow.id?.trim();
    if (!workflowId) {
      return [];
    }
    return [
      {
        workflowId,
        label: workflow.name,
        modified: workflow.modified,
        inspectionPath: `${SAVED_WORKFLOW_INSPECTION_ROOT}/${workflowId}${WORKFLOW_FILE_EXTENSION}`,
      },
    ];
  });
}

export function buildSavedGraphInspectionDisplayModel(
  projection: WorkflowGraphInspectionProjection,
  selectedNodeId: string | null,
): SavedGraphInspectionDisplayModel {
  const selectedNodeDiagnostics = selectedNodeId
    ? diagnosticsForSelectedNode(projection, selectedNodeId)
    : [];

  return {
    canvas: buildGraphInspectionCanvasModel(projection.graph, projection.diagnostics),
    diagnostics: projection.diagnostics,
    selectedNodeId,
    selectedNodeDiagnostics,
    artifactControlsEnabled: false,
    hasRunContext: false,
  };
}

export function resolveSavedWorkflowInspectionPath(
  requestedPath: string | null,
  options: SavedGraphInspectionOption[],
): string | null {
  if (requestedPath && options.some((option) => option.inspectionPath === requestedPath)) {
    return requestedPath;
  }
  return options[0]?.inspectionPath ?? null;
}

export function resolveSavedGraphSelectedNodeId(
  currentNodeId: string | null,
  projection: WorkflowGraphInspectionProjection,
): string | null {
  const nodeIds = projection.graph.nodes.map((node) => node.id);
  if (currentNodeId && nodeIds.includes(currentNodeId)) {
    return currentNodeId;
  }

  const staleNodeId = projection.diagnostics
    .map((diagnostic) => diagnostic.node_id)
    .find((nodeId): nodeId is string => Boolean(nodeId && nodeIds.includes(nodeId)));
  return staleNodeId ?? nodeIds[0] ?? null;
}

export function buildGraphInspectionCanvasModel(
  graph: WorkflowGraph,
  diagnostics: WorkflowGraphDiagnostic[],
): GraphInspectionCanvasModel {
  return buildRunGraphCanvasModel(graph, {}, {}, diagnostics);
}

export function formatSavedGraphNodeAccessibleLabel(
  node: GraphInspectionCanvasNode,
): string {
  const parts = [`${node.id} ${node.nodeType}`];
  if (node.staleBadgeLabel) {
    parts.push(node.staleBadgeLabel);
  }
  return parts.join(', ');
}

export function savedGraphNodeFocusDomId(nodeId: string): string {
  const encoded = Array.from(nodeId)
    .map((character) => {
      if (/^[A-Za-z0-9_-]$/.test(character)) {
        return character;
      }
      const codePoint = character.codePointAt(0) ?? 0;
      return `_${codePoint.toString(16).padStart(4, '0')}_`;
    })
    .join('');
  return `saved-graph-node-${encoded || 'node'}`;
}

export function isSavedGraphNodeSelectionKey(key: string): boolean {
  return key === 'Enter' || key === ' ';
}

function diagnosticsForSelectedNode(
  projection: WorkflowGraphInspectionProjection,
  selectedNodeId: string,
): WorkflowGraphDiagnostic[] {
  if (projection.selected_node?.node.id === selectedNodeId) {
    return projection.selected_node.diagnostics;
  }
  return projection.diagnostics.filter((diagnostic) => diagnostic.node_id === selectedNodeId);
}

// --- Graph page: edit and run modes ---

export type GraphPageMode = 'edit' | 'run';

/** A mode the user picked explicitly; it holds only while the same run is selected. */
export interface GraphPageModeRequest {
  runId: string | null;
  mode: GraphPageMode;
}

export interface GraphPageModeInput {
  /** The shared run context (workbenchStore), or null when no run is selected. */
  activeRun: { workflow_run_id: string; workflow_id?: string | null } | null;
  /** Saved workflow open in the editor; null for system graphs or while loading. */
  loadedWorkflowId: string | null;
  request: GraphPageModeRequest | null;
}

export interface GraphPageModeResolution {
  mode: GraphPageMode;
  /** A run is selected, so run mode can be entered. */
  runAvailable: boolean;
  /** Edit mode while the selected run belongs to another workflow: offer "Show <wf> as run". */
  workflowMismatch: boolean;
}

/**
 * Which mode the Graph page shows. Default: run mode when the selected run is
 * of the workflow open in the editor, edit mode otherwise. An explicit choice
 * made for the current run wins; selecting another run returns to the default.
 */
export function resolveGraphPageMode(input: GraphPageModeInput): GraphPageModeResolution {
  const run = input.activeRun;
  if (!run) {
    return { mode: 'edit', runAvailable: false, workflowMismatch: false };
  }
  const runWorkflowId = run.workflow_id?.trim() || null;
  const sameWorkflow = runWorkflowId !== null && runWorkflowId === input.loadedWorkflowId;
  const requested =
    input.request && input.request.runId === run.workflow_run_id ? input.request.mode : null;
  const mode: GraphPageMode = requested ?? (sameWorkflow ? 'run' : 'edit');
  return {
    mode,
    runAvailable: true,
    workflowMismatch: mode === 'edit' && runWorkflowId !== null && !sameWorkflow,
  };
}

/** Last characters of a run id, as shown in the mode switch. */
export function runIdSuffix(runId: string, length = 6): string {
  return runId.length <= length ? runId : runId.slice(-length);
}

export function formatRunModeLabel(runId: string | null): string {
  return runId ? `As run ${runIdSuffix(runId)}` : 'As a run…';
}

/** Edit-mode summary; `version` is null for read-only system graphs, which have none. */
export function formatEditDimensionLabel(
  version: string | null,
  nodeCount: number,
  edgeCount: number,
): string {
  const counts = `${pluralize(nodeCount, 'node')}, ${pluralize(edgeCount, 'edge')}`;
  return version ? `Editing v${version}, ${counts}` : `Viewing, ${counts}`;
}

export function formatRunDimensionLabel(
  nodeCount: number,
  runId: string,
  statusLabel: string,
): string {
  return `${pluralize(nodeCount, 'node')} as run ${runIdSuffix(runId)} ran them, ${statusLabel.toLowerCase()}`;
}

export function formatEditCurrentLabel(version: string | null): string {
  return version ? `Edit current v${version}` : 'Edit current version';
}

/** "Saved 20:04", "Unsaved changes" or "Not saved yet" for the toolbar status. */
export function formatWorkflowSaveLine(input: {
  isDirty: boolean;
  savedModifiedAt: string | null;
}): string {
  if (input.isDirty) {
    return 'Unsaved changes';
  }
  if (!input.savedModifiedAt) {
    return 'Not saved yet';
  }
  const modified = new Date(input.savedModifiedAt);
  if (Number.isNaN(modified.getTime())) {
    return 'Saved';
  }
  const hh = String(modified.getHours()).padStart(2, '0');
  const mm = String(modified.getMinutes()).padStart(2, '0');
  return `Saved ${hh}:${mm}`;
}

const SHORT_MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** Modified time in the workflow list: "20:04" today, "Oct 29" otherwise. */
export function formatWorkflowModifiedShort(modifiedAt: string, nowMs: number): string {
  const modified = new Date(modifiedAt);
  if (Number.isNaN(modified.getTime())) {
    return '';
  }
  const now = new Date(nowMs);
  const sameDay =
    modified.getFullYear() === now.getFullYear() &&
    modified.getMonth() === now.getMonth() &&
    modified.getDate() === now.getDate();
  if (sameDay) {
    return `${String(modified.getHours()).padStart(2, '0')}:${String(modified.getMinutes()).padStart(2, '0')}`;
  }
  return `${SHORT_MONTHS[modified.getMonth()]} ${modified.getDate()}`;
}

/** Editor node execution state (svelte-graph) to the shared glyph and its words. */
export function editNodeExecutionGlyph(state: string | null | undefined): {
  glyph: GlyphState;
  label: string;
} {
  switch (state) {
    case 'running':
      return { glyph: 'running', label: 'Running' };
    case 'waiting':
      return { glyph: 'waiting', label: 'Waiting' };
    case 'success':
      return { glyph: 'succeeded', label: 'Done' };
    case 'error':
      return { glyph: 'failed', label: 'Failed' };
    default:
      return { glyph: 'idle', label: 'Idle' };
  }
}

export interface PaletteDefinitionLike {
  node_type: string;
  label: string;
  description: string;
  inputs: unknown[];
  outputs: unknown[];
}

export interface NodePaletteItem<D extends PaletteDefinitionLike> {
  definition: D;
  label: string;
  /** "in:out" port counts, for example "2:1". */
  ports: string;
  portsTitle: string;
}

export interface NodePaletteCategory<D extends PaletteDefinitionLike> {
  key: string;
  label: string;
  count: number;
  expanded: boolean;
  items: NodePaletteItem<D>[];
}

/**
 * Palette tree: categories in catalogue order with their matching node count.
 * While searching, every category with matches is open so results are visible.
 */
export function buildNodePaletteTree<D extends PaletteDefinitionLike>(
  byCategory: ReadonlyMap<string, readonly D[]>,
  query: string,
  expandedCategories: ReadonlySet<string>,
): NodePaletteCategory<D>[] {
  const needle = query.trim().toLowerCase();
  const categories: NodePaletteCategory<D>[] = [];
  for (const [category, definitions] of byCategory) {
    const matches = needle
      ? definitions.filter(
          (definition) =>
            definition.label.toLowerCase().includes(needle) ||
            definition.description.toLowerCase().includes(needle),
        )
      : [...definitions];
    if (matches.length === 0) {
      continue;
    }
    categories.push({
      key: category,
      label: formatPaletteCategoryLabel(category),
      count: matches.length,
      expanded: needle.length > 0 || expandedCategories.has(category),
      items: matches.map((definition) => ({
        definition,
        label: definition.label,
        ports: `${definition.inputs.length}:${definition.outputs.length}`,
        portsTitle: `${pluralize(definition.inputs.length, 'input')}, ${pluralize(definition.outputs.length, 'output')}`,
      })),
    });
  }
  return categories;
}

export function formatPaletteCategoryLabel(category: string): string {
  const words = category.replaceAll('_', ' ').trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function pluralize(count: number, noun: string): string {
  return count === 1 ? `1 ${noun}` : `${count} ${noun}s`;
}
