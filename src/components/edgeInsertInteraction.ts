import type {
  ConnectionRejection,
  EdgeInsertionBridge,
} from '../services/workflow/types';
export const EDGE_INSERT_HIT_RADIUS_PX = 24;
export const EDGE_INSERT_SAMPLE_STEP_PX = 20;

export type Point = {
  x: number;
  y: number;
};

type EdgePathLookupElement = {
  dataset: {
    id?: string;
  };
  querySelector(selector: string): Element | null;
};

type EdgePathLookupRoot = {
  querySelectorAll(selector: string): Iterable<EdgePathLookupElement>;
};

type EdgePathLike = SVGPathElement & {
  getTotalLength(): number;
  getPointAtLength(length: number): Point;
  getScreenCTM(): DOMMatrix | DOMMatrixReadOnly | null;
};

type Matrix2D = {
  a: number;
  b: number;
  c: number;
  d: number;
  e: number;
  f: number;
};

export interface EdgeInsertHitTarget {
  edgeId: string;
  hitPoint: Point;
  distance: number;
}

export interface EdgeInsertPreviewState {
  edgeId: string | null;
  nodeType: string | null;
  graphRevision: string | null;
  hitPoint: Point | null;
  pending: boolean;
  bridge: EdgeInsertionBridge | null;
  rejection: ConnectionRejection | null;
}

export interface EdgeInsertEdgeLike {
  id: string;
  data?: Record<string, unknown> | null;
}

export function createEdgeInsertPreviewState(): EdgeInsertPreviewState {
  return {
    edgeId: null,
    nodeType: null,
    graphRevision: null,
    hitPoint: null,
    pending: false,
    bridge: null,
    rejection: null,
  };
}

export function clearEdgeInsertPreviewState(): EdgeInsertPreviewState {
  return createEdgeInsertPreviewState();
}

export function shouldRefreshEdgeInsertPreview(
  state: EdgeInsertPreviewState,
  edgeId: string,
  nodeType: string,
  graphRevision: string,
): boolean {
  return (
    state.edgeId !== edgeId ||
    state.nodeType !== nodeType ||
    state.graphRevision !== graphRevision ||
    (state.bridge === null && state.rejection === null && !state.pending)
  );
}

export function setEdgeInsertHoverTarget(
  state: EdgeInsertPreviewState,
  target: EdgeInsertHitTarget,
  nodeType: string,
  graphRevision: string,
): EdgeInsertPreviewState {
  return {
    edgeId: target.edgeId,
    nodeType,
    graphRevision,
    hitPoint: target.hitPoint,
    pending: false,
    bridge: null,
    rejection: null,
  };
}

export function updateEdgeInsertHitPoint(
  state: EdgeInsertPreviewState,
  hitPoint: Point,
): EdgeInsertPreviewState {
  if (!state.edgeId) {
    return state;
  }

  return {
    ...state,
    hitPoint,
  };
}

export function setEdgeInsertPreviewPending(
  state: EdgeInsertPreviewState,
): EdgeInsertPreviewState {
  if (!state.edgeId || !state.nodeType || !state.graphRevision || !state.hitPoint) {
    return state;
  }

  return {
    ...state,
    pending: true,
    bridge: null,
    rejection: null,
  };
}

export function setEdgeInsertPreviewResolved(
  state: EdgeInsertPreviewState,
  bridge: EdgeInsertionBridge,
): EdgeInsertPreviewState {
  return {
    ...state,
    pending: false,
    bridge,
    rejection: null,
  };
}

export function setEdgeInsertPreviewRejected(
  state: EdgeInsertPreviewState,
  rejection?: ConnectionRejection,
): EdgeInsertPreviewState {
  return {
    ...state,
    pending: false,
    bridge: null,
    rejection: rejection ?? null,
  };
}

export function isEdgeInsertPreviewRequestCurrent(params: {
  requestId: number;
  activeRequestId: number;
  state: EdgeInsertPreviewState;
  edgeId: string;
  nodeType: string;
  graphRevision: string;
}): boolean {
  return (
    params.requestId === params.activeRequestId &&
    params.state.edgeId === params.edgeId &&
    params.state.nodeType === params.nodeType &&
    params.state.graphRevision === params.graphRevision
  );
}

export function shouldClearEdgeInsertPreviewForGraphState(params: {
  state: EdgeInsertPreviewState;
  edgeInsertEnabled: boolean;
  externalPaletteDragActive: boolean;
  currentGraphRevision: string;
}): boolean {
  if (!params.state.edgeId) {
    return false;
  }

  return (
    !params.edgeInsertEnabled ||
    !params.externalPaletteDragActive ||
    !params.currentGraphRevision ||
    params.state.graphRevision !== params.currentGraphRevision
  );
}

export function getCommittableEdgeInsertPreview(
  state: EdgeInsertPreviewState,
  nodeType: string,
): EdgeInsertPreviewState | null {
  if (!state.edgeId || state.nodeType !== nodeType || !state.graphRevision || !state.bridge) {
    return null;
  }

  return {
    ...state,
  };
}

export function applyEdgeInsertPreviewActiveFlag<EdgeLike extends EdgeInsertEdgeLike>(
  edges: EdgeLike[],
  previewEdgeId: string | null,
): { edges: EdgeLike[]; changed: boolean } {
  let changed = false;

  const nextEdges = edges.map((edge) => {
    const edgeData = edge.data ?? {};
    const isPreviewActive = edge.id === previewEdgeId;
    const hasPreviewFlag = edgeData.edgeInsertPreviewActive === true;

    if (isPreviewActive === hasPreviewFlag) {
      return edge;
    }

    changed = true;
    const nextData = { ...edgeData };
    if (isPreviewActive) {
      nextData.edgeInsertPreviewActive = true;
    } else {
      delete nextData.edgeInsertPreviewActive;
    }

    return {
      ...edge,
      data: nextData,
    };
  });

  return {
    edges: nextEdges,
    changed,
  };
}

function distanceBetweenPoints(left: Point, right: Point): number {
  return Math.hypot(left.x - right.x, left.y - right.y);
}

function applyMatrixToPoint(point: Point, matrix: Matrix2D): Point {
  return {
    x: matrix.a * point.x + matrix.c * point.y + matrix.e,
    y: matrix.b * point.x + matrix.d * point.y + matrix.f,
  };
}

function toContainerRelativePoint(
  point: Point,
  containerRect: Pick<DOMRect, 'left' | 'top'>,
): Point {
  return {
    x: point.x - containerRect.left,
    y: point.y - containerRect.top,
  };
}

export function sampleClosestEdgeDistance(params: {
  path: EdgePathLike;
  hitPoint: Point;
  containerRect: Pick<DOMRect, 'left' | 'top'>;
  sampleStepPx?: number;
}): number | null {
  const matrix = params.path.getScreenCTM();
  if (!matrix) {
    return null;
  }

  const totalLength = params.path.getTotalLength();
  if (!Number.isFinite(totalLength) || totalLength <= 0) {
    return null;
  }

  const sampleStepPx = params.sampleStepPx ?? EDGE_INSERT_SAMPLE_STEP_PX;
  const sampleCount = Math.max(1, Math.ceil(totalLength / sampleStepPx));
  let bestDistance = Number.POSITIVE_INFINITY;

  for (let index = 0; index <= sampleCount; index += 1) {
    const pathDistance = Math.min(totalLength, index * sampleStepPx);
    const screenPoint = applyMatrixToPoint(
      params.path.getPointAtLength(pathDistance),
      matrix,
    );
    const containerPoint = toContainerRelativePoint(screenPoint, params.containerRect);
    bestDistance = Math.min(
      bestDistance,
      distanceBetweenPoints(containerPoint, params.hitPoint),
    );
  }

  return Number.isFinite(bestDistance) ? bestDistance : null;
}

export function findEdgeInsertHitTarget(params: {
  root: EdgePathLookupRoot;
  hitPoint: Point;
  containerRect: Pick<DOMRect, 'left' | 'top'>;
  thresholdPx?: number;
}): EdgeInsertHitTarget | null {
  const thresholdPx = params.thresholdPx ?? EDGE_INSERT_HIT_RADIUS_PX;
  let bestMatch: EdgeInsertHitTarget | null = null;

  for (const edgeElement of params.root.querySelectorAll('.svelte-flow__edge[data-id]')) {
    const edgeId = edgeElement.dataset.id;
    if (!edgeId) {
      continue;
    }

    const path = edgeElement.querySelector('.react-flow__edge-path') as EdgePathLike | null;
    if (!path) {
      continue;
    }

    const distance = sampleClosestEdgeDistance({
      path,
      hitPoint: params.hitPoint,
      containerRect: params.containerRect,
    });
    if (distance === null || distance > thresholdPx) {
      continue;
    }

    if (bestMatch === null || distance < bestMatch.distance) {
      bestMatch = {
        edgeId,
        hitPoint: params.hitPoint,
        distance,
      };
    }
  }

  return bestMatch;
}
