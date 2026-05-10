import type {
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
  canvas: RunGraphCanvasModel;
  diagnostics: WorkflowGraphDiagnostic[];
  selectedNodeId: string | null;
  selectedNodeDiagnostics: WorkflowGraphDiagnostic[];
  artifactControlsEnabled: false;
  hasRunContext: false;
}

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
    canvas: buildRunGraphCanvasModel(projection.graph, {}, {}, projection.diagnostics),
    diagnostics: projection.diagnostics,
    selectedNodeId,
    selectedNodeDiagnostics,
    artifactControlsEnabled: false,
    hasRunContext: false,
  };
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
