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

export function isCutModifierPressed(event: ModifierEvent): boolean {
  return event.ctrlKey === true || event.metaKey === true || event.key === 'Control' || event.key === 'Meta';
}

export function shouldStartCutGesture(params: {
  enabled: boolean;
  modifierPressed: boolean;
  target: ClosestTarget | null;
}): boolean {
  if (!params.enabled || !params.modifierPressed || !params.target) {
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
