import type { Stroke, Point } from '../types';
import { componentRegistry } from './HotLoadRegistry';

export interface DrawingBounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  width: number;
  height: number;
  centerX: number;
  centerY: number;
}

export interface ComponentPosition {
  id: string;
  name: string;
  path: string;
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

/**
 * Calculate the bounding box of all strokes
 */
export function calculateBounds(strokes: Stroke[]): DrawingBounds | null {
  if (strokes.length === 0) {
    return null;
  }

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const stroke of strokes) {
    for (const point of stroke.points) {
      minX = Math.min(minX, point.x);
      minY = Math.min(minY, point.y);
      maxX = Math.max(maxX, point.x);
      maxY = Math.max(maxY, point.y);
    }
  }

  // Handle case where all points are the same
  if (minX === Infinity || minY === Infinity) {
    return null;
  }

  const width = maxX - minX;
  const height = maxY - minY;

  return {
    minX,
    minY,
    maxX,
    maxY,
    width: Math.max(width, 10), // Minimum size
    height: Math.max(height, 10),
    centerX: (minX + maxX) / 2,
    centerY: (minY + maxY) / 2,
  };
}

/**
 * Find the component at a given position
 * Returns the topmost component at that position
 */
export function findElementAtPosition(
  x: number,
  y: number,
  components: ComponentPosition[]
): string | null {
  // Sort by z-index or order (last is on top) and find first match
  for (let i = components.length - 1; i >= 0; i--) {
    const comp = components[i];
    const { bounds } = comp;

    if (
      x >= bounds.x &&
      x <= bounds.x + bounds.width &&
      y >= bounds.y &&
      y <= bounds.y + bounds.height
    ) {
      return comp.id;
    }
  }

  return null;
}

/**
 * Get the center point of the drawing
 */
export function getDrawingCenter(strokes: Stroke[]): Point | null {
  const bounds = calculateBounds(strokes);
  if (!bounds) return null;

  return {
    x: bounds.centerX,
    y: bounds.centerY,
  };
}

/**
 * Determine if a drawing overlaps with a component
 */
export function doesDrawingOverlap(
  bounds: DrawingBounds,
  component: ComponentPosition
): boolean {
  const compBounds = component.bounds;

  return !(
    bounds.maxX < compBounds.x ||
    bounds.minX > compBounds.x + compBounds.width ||
    bounds.maxY < compBounds.y ||
    bounds.minY > compBounds.y + compBounds.height
  );
}

/**
 * Find which existing component the drawing is targeting (overlapping with)
 */
export function findTargetComponent(
  strokes: Stroke[],
  components: ComponentPosition[]
): string | null {
  const bounds = calculateBounds(strokes);
  if (!bounds) return null;

  // First, check if the center of the drawing is inside any component
  const centerTarget = findElementAtPosition(bounds.centerX, bounds.centerY, components);
  if (centerTarget) {
    return centerTarget;
  }

  // Otherwise, check for any overlap
  for (const comp of components) {
    if (doesDrawingOverlap(bounds, comp)) {
      return comp.id;
    }
  }

  return null;
}

/**
 * Calculate suggested position for a new component based on drawing
 */
export function suggestComponentPosition(
  bounds: DrawingBounds
): { x: number; y: number; width: number; height: number } {
  return {
    x: bounds.minX,
    y: bounds.minY,
    width: bounds.width,
    height: bounds.height,
  };
}
