type ModifierEvent = {
  ctrlKey?: boolean;
  metaKey?: boolean;
  key?: string;
};

type ClosestTarget = {
  closest(selector: string): Element | null;
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

type Point = {
  x: number;
  y: number;
};

type Matrix2D = {
  a: number;
  b: number;
  c: number;
  d: number;
  e: number;
  f: number;
};

export function isCutModifierPressed(event: ModifierEvent): boolean {
  return event.ctrlKey === true || event.metaKey === true || event.key === 'Control' || event.key === 'Meta';
}

export function shouldStartCutGesture(params: {
  canEdit: boolean;
  modifierPressed: boolean;
  target: ClosestTarget | null;
}): boolean {
  if (!params.canEdit || !params.modifierPressed || !params.target) {
    return false;
  }

  return !params.target.closest('.svelte-flow__node') && !params.target.closest('.svelte-flow__handle');
}

export function findRenderedEdgePath(
  root: EdgePathLookupRoot,
  edgeId: string,
): SVGPathElement | null {
  for (const edgeElement of root.querySelectorAll('.svelte-flow__edge[data-id]')) {
    if (edgeElement.dataset.id !== edgeId) {
      continue;
    }

    return edgeElement.querySelector('.react-flow__edge-path') as SVGPathElement | null;
  }

  return null;
}

export function applyMatrixToPoint(point: Point, matrix: Matrix2D): Point {
  return {
    x: matrix.a * point.x + matrix.c * point.y + matrix.e,
    y: matrix.b * point.x + matrix.d * point.y + matrix.f,
  };
}

export function toContainerRelativePoint(
  point: Point,
  containerRect: Pick<DOMRect, 'left' | 'top'>,
): Point {
  return {
    x: point.x - containerRect.left,
    y: point.y - containerRect.top,
  };
}
