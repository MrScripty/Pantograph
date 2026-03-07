import type { InsertableNodeTypeCandidate } from './types/workflow.js';

export const HORSESHOE_VISIBLE_COUNT = 5;
export const HORSESHOE_OUTER_RADIUS = 126;
export const HORSESHOE_SELECTION_RADIUS = 64;
export const HORSESHOE_START_ANGLE = -150;
export const HORSESHOE_END_ANGLE = -30;

export interface HorseshoePoint {
  x: number;
  y: number;
}

export interface HorseshoeItemPosition extends HorseshoePoint {
  angle: number;
}

export interface HorseshoeVisibleItem<T> {
  item: T;
  index: number;
  slot: number;
}

export interface HorseshoeWindow<T> {
  visibleItems: HorseshoeVisibleItem<T>[];
  hiddenBefore: number;
  hiddenAfter: number;
}

export function clampHorseshoeIndex(index: number, itemCount: number): number {
  if (itemCount <= 0) return 0;
  return Math.max(0, Math.min(index, itemCount - 1));
}

export function rotateHorseshoeIndex(
  currentIndex: number,
  delta: number,
  itemCount: number,
): number {
  return clampHorseshoeIndex(currentIndex + delta, itemCount);
}

export function getHorseshoeItemPosition(
  slot: number,
  itemCount: number,
): HorseshoeItemPosition {
  if (itemCount <= 1) {
    return {
      x: 0,
      y: -HORSESHOE_OUTER_RADIUS,
      angle: -90,
    };
  }

  const step = (HORSESHOE_END_ANGLE - HORSESHOE_START_ANGLE) / (itemCount - 1);
  const angle = HORSESHOE_START_ANGLE + step * slot;
  const radians = (angle * Math.PI) / 180;
  return {
    x: Math.cos(radians) * HORSESHOE_OUTER_RADIUS,
    y: Math.sin(radians) * HORSESHOE_OUTER_RADIUS,
    angle,
  };
}

export function getHorseshoeWindow<T>(
  items: T[],
  selectedIndex: number,
  visibleCount = HORSESHOE_VISIBLE_COUNT,
): HorseshoeWindow<T> {
  if (items.length === 0) {
    return {
      visibleItems: [],
      hiddenBefore: 0,
      hiddenAfter: 0,
    };
  }

  const clampedIndex = clampHorseshoeIndex(selectedIndex, items.length);
  const count = Math.min(Math.max(1, visibleCount), items.length);
  const half = Math.floor(count / 2);

  let start = Math.max(0, clampedIndex - half);
  let end = start + count;

  if (end > items.length) {
    end = items.length;
    start = Math.max(0, end - count);
  }

  return {
    visibleItems: items.slice(start, end).map((item, offset) => ({
      item,
      index: start + offset,
      slot: offset,
    })),
    hiddenBefore: start,
    hiddenAfter: Math.max(0, items.length - end),
  };
}

function normalizeSearchQuery(query: string): string {
  return query.trim().toLowerCase();
}

function matchScore(candidate: InsertableNodeTypeCandidate, normalizedQuery: string): number {
  if (!normalizedQuery) return Number.POSITIVE_INFINITY;

  const label = candidate.label.toLowerCase();
  const nodeType = candidate.node_type.toLowerCase();
  const category = candidate.category.toLowerCase();

  if (label.startsWith(normalizedQuery)) return 0;
  if (nodeType.startsWith(normalizedQuery)) return 100;
  if (category.startsWith(normalizedQuery)) return 200;

  const labelIndex = label.indexOf(normalizedQuery);
  if (labelIndex >= 0) return 300 + labelIndex;

  const nodeTypeIndex = nodeType.indexOf(normalizedQuery);
  if (nodeTypeIndex >= 0) return 500 + nodeTypeIndex;

  const categoryIndex = category.indexOf(normalizedQuery);
  if (categoryIndex >= 0) return 700 + categoryIndex;

  return Number.POSITIVE_INFINITY;
}

export function findBestInsertableMatchIndex(
  items: InsertableNodeTypeCandidate[],
  query: string,
  fallbackIndex = 0,
): number {
  if (items.length === 0) return 0;

  const normalizedQuery = normalizeSearchQuery(query);
  if (!normalizedQuery) {
    return clampHorseshoeIndex(fallbackIndex, items.length);
  }

  let bestIndex = clampHorseshoeIndex(fallbackIndex, items.length);
  let bestScore = Number.POSITIVE_INFINITY;

  for (const [index, item] of items.entries()) {
    const score = matchScore(item, normalizedQuery);
    if (score < bestScore) {
      bestScore = score;
      bestIndex = index;
    }
  }

  return bestScore === Number.POSITIVE_INFINITY
    ? clampHorseshoeIndex(fallbackIndex, items.length)
    : bestIndex;
}

export function findNearestVisibleHorseshoeIndex<T>(
  items: T[],
  selectedIndex: number,
  pointerPosition: HorseshoePoint,
  anchorPosition: HorseshoePoint,
  visibleCount = HORSESHOE_VISIBLE_COUNT,
  selectionRadius = HORSESHOE_SELECTION_RADIUS,
): number | null {
  if (items.length === 0) return null;

  const window = getHorseshoeWindow(items, selectedIndex, visibleCount);
  const maxDistanceSquared = selectionRadius * selectionRadius;
  let nearestIndex: number | null = null;
  let nearestDistanceSquared = maxDistanceSquared;

  for (const entry of window.visibleItems) {
    const position = getHorseshoeItemPosition(entry.slot, window.visibleItems.length);
    const dx = anchorPosition.x + position.x - pointerPosition.x;
    const dy = anchorPosition.y + position.y - pointerPosition.y;
    const distanceSquared = dx * dx + dy * dy;

    if (distanceSquared <= nearestDistanceSquared) {
      nearestIndex = entry.index;
      nearestDistanceSquared = distanceSquared;
    }
  }

  return nearestIndex;
}
