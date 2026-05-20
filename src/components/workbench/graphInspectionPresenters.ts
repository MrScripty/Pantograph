import type {
  WorkflowGraph,
  WorkflowGraphDiagnostic,
  WorkflowGraphInspectionProjection,
  WorkflowMetadata,
} from '../../services/workflow/types';
import { buildRunGraphCanvasModel, type RunGraphCanvasModel } from './runGraphPresenters.ts';

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
