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

export function linesIntersect(
  a1: Point,
  a2: Point,
  b1: Point,
  b2: Point,
): boolean {
  const det = (a2.x - a1.x) * (b2.y - b1.y) - (b2.x - b1.x) * (a2.y - a1.y);
  if (det === 0) return false;

  const lambda = ((b2.y - b1.y) * (b2.x - a1.x) + (b1.x - b2.x) * (b2.y - a1.y)) / det;
  const gamma = ((a1.y - a2.y) * (b2.x - a1.x) + (a2.x - a1.x) * (b2.y - a1.y)) / det;

  return 0 < lambda && lambda < 1 && 0 < gamma && gamma < 1;
}

export function lineIntersectsPath(
  p1: Point,
  p2: Point,
  path: SVGPathElement,
  containerRect: DOMRect | null,
): boolean {
  const screenMatrix = path.getScreenCTM();
  if (!screenMatrix || !containerRect) {
    return false;
  }

  const pathLength = path.getTotalLength();
  const samples = 20;

  for (let i = 0; i < samples; i++) {
    const t1 = (i / samples) * pathLength;
    const t2 = ((i + 1) / samples) * pathLength;

    const point1 = path.getPointAtLength(t1);
    const point2 = path.getPointAtLength(t2);
    const containerPoint1 = toContainerRelativePoint(
      applyMatrixToPoint(point1, screenMatrix),
      containerRect,
    );
    const containerPoint2 = toContainerRelativePoint(
      applyMatrixToPoint(point2, screenMatrix),
      containerRect,
    );

    if (linesIntersect(p1, p2, containerPoint1, containerPoint2)) {
      return true;
    }
  }

  return false;
}
