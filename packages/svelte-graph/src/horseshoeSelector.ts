import type { InsertableNodeTypeCandidate } from './types/workflow.js';

export const HORSESHOE_VISIBLE_COUNT = 5;

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
