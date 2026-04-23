import type { NodeDefinition } from '../services/workflow/types';
import { workflowService } from '../services/workflow/WorkflowService';
import {
  clearEdgeInsertPreviewState,
  findEdgeInsertHitTarget,
  isEdgeInsertPreviewRequestCurrent,
  setEdgeInsertHoverTarget,
  setEdgeInsertPreviewPending,
  setEdgeInsertPreviewRejected,
  setEdgeInsertPreviewResolved,
  shouldRefreshEdgeInsertPreview,
  updateEdgeInsertHitPoint,
  type EdgeInsertPreviewState,
} from './edgeInsertInteraction';

interface RefreshEdgeInsertPreviewParams {
  bumpRequestId: () => number;
  containerElement: HTMLElement | undefined;
  definition: NodeDefinition;
  edgeInsertEnabled: boolean;
  externalPaletteDragActive: boolean;
  getRequestId: () => number;
  getState: () => EdgeInsertPreviewState;
  graphRevision: string;
  hitPoint: { x: number; y: number } | null;
  setState: (state: EdgeInsertPreviewState) => void;
}

function clearPreview(params: RefreshEdgeInsertPreviewParams) {
  params.bumpRequestId();
  params.setState(clearEdgeInsertPreviewState());
}

export async function refreshWorkflowGraphEdgeInsertPreview(
  params: RefreshEdgeInsertPreviewParams,
) {
  if (!params.externalPaletteDragActive || !params.edgeInsertEnabled) {
    clearPreview(params);
    return;
  }

  const flowRoot = params.containerElement?.querySelector('.svelte-flow');
  if (!params.hitPoint || !flowRoot || !params.graphRevision) {
    clearPreview(params);
    return;
  }

  const hitTarget = findEdgeInsertHitTarget({
    root: flowRoot,
    hitPoint: params.hitPoint,
    containerRect: flowRoot.getBoundingClientRect(),
  });
  if (!hitTarget) {
    clearPreview(params);
    return;
  }

  if (
    !shouldRefreshEdgeInsertPreview(
      params.getState(),
      hitTarget.edgeId,
      params.definition.node_type,
      params.graphRevision,
    )
  ) {
    params.setState(updateEdgeInsertHitPoint(params.getState(), hitTarget.hitPoint));
    return;
  }

  params.setState(
    setEdgeInsertPreviewPending(
      setEdgeInsertHoverTarget(
        params.getState(),
        hitTarget,
        params.definition.node_type,
        params.graphRevision,
      ),
    ),
  );

  const requestId = params.bumpRequestId();
  try {
    const response = await workflowService.previewNodeInsertOnEdge(
      hitTarget.edgeId,
      params.definition.node_type,
      params.graphRevision,
    );

    if (
      !isEdgeInsertPreviewRequestCurrent({
        requestId,
        activeRequestId: params.getRequestId(),
        state: params.getState(),
        edgeId: hitTarget.edgeId,
        nodeType: params.definition.node_type,
        graphRevision: params.graphRevision,
      })
    ) {
      return;
    }

    if (response.accepted && response.bridge) {
      params.setState(setEdgeInsertPreviewResolved(params.getState(), response.bridge));
      return;
    }

    params.setState(setEdgeInsertPreviewRejected(params.getState(), response.rejection));
  } catch (error) {
    if (
      isEdgeInsertPreviewRequestCurrent({
        requestId,
        activeRequestId: params.getRequestId(),
        state: params.getState(),
        edgeId: hitTarget.edgeId,
        nodeType: params.definition.node_type,
        graphRevision: params.graphRevision,
      })
    ) {
      params.setState(setEdgeInsertPreviewRejected(params.getState()));
    }
    console.error('[WorkflowGraph] Failed to preview edge insertion:', error);
  }
}
