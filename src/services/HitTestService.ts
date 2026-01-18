import type { Point } from '../types';
import type { DrawingBounds } from './DrawingAnalyzer';

/**
 * Result of a DOM hit test
 */
export interface HitTestResult {
  componentId: string;
  componentPath: string;
  element: HTMLElement;
}

/**
 * Service for detecting which generated components the user drew on/over.
 * Uses DOM hit-testing via document.elementsFromPoint() for accurate detection.
 */
export class HitTestService {
  /**
   * Find all generated components that overlap with the drawing bounds.
   * Uses document.elementsFromPoint() for accurate DOM hit-testing.
   *
   * @param bounds - The bounding box of the user's drawing (in canvas coordinates)
   * @param panOffset - Current canvas pan offset
   * @returns Array of hit test results for components found
   */
  findComponentsInBounds(
    bounds: DrawingBounds,
    panOffset: Point
  ): HitTestResult[] {
    const results: HitTestResult[] = [];
    const seen = new Set<string>();

    // Sample multiple points within bounds for better coverage
    const samplePoints = this.generateSamplePoints(bounds, panOffset);

    for (const point of samplePoints) {
      const elements = document.elementsFromPoint(point.x, point.y);

      for (const el of elements) {
        const componentId = this.findComponentAncestor(el);
        if (componentId && !seen.has(componentId)) {
          seen.add(componentId);
          const wrapper = el.closest('[data-component-path]');
          results.push({
            componentId,
            componentPath: wrapper?.getAttribute('data-component-path') ?? '',
            element: el as HTMLElement,
          });
        }
      }
    }

    return results;
  }

  /**
   * Find a single component at a specific screen point
   *
   * @param screenPoint - Screen coordinates to test
   * @returns Hit test result or null if no component found
   */
  findComponentAtPoint(screenPoint: Point): HitTestResult | null {
    const elements = document.elementsFromPoint(screenPoint.x, screenPoint.y);

    for (const el of elements) {
      const componentId = this.findComponentAncestor(el);
      if (componentId) {
        const wrapper = el.closest('[data-component-path]');
        return {
          componentId,
          componentPath: wrapper?.getAttribute('data-component-path') ?? '',
          element: el as HTMLElement,
        };
      }
    }

    return null;
  }

  /**
   * Find the generated component wrapper ancestor of an element
   */
  private findComponentAncestor(el: Element): string | null {
    const wrapper = el.closest('[data-component-id]');
    return wrapper?.getAttribute('data-component-id') ?? null;
  }

  /**
   * Generate sample points within the drawing bounds for thorough coverage.
   * Converts canvas coordinates to screen coordinates using pan offset.
   *
   * @param bounds - Drawing bounds in canvas coordinates
   * @param pan - Canvas pan offset
   * @returns Array of screen coordinate points to test
   */
  private generateSamplePoints(bounds: DrawingBounds, pan: Point): Point[] {
    const points: Point[] = [];

    // Convert canvas coordinates to screen coordinates
    const screenBounds = {
      x: bounds.minX + pan.x,
      y: bounds.minY + pan.y,
      width: bounds.width,
      height: bounds.height,
    };

    // Center point (most important)
    points.push({
      x: screenBounds.x + screenBounds.width / 2,
      y: screenBounds.y + screenBounds.height / 2,
    });

    // Corners
    points.push({ x: screenBounds.x, y: screenBounds.y }); // Top-left
    points.push({ x: screenBounds.x + screenBounds.width, y: screenBounds.y }); // Top-right
    points.push({ x: screenBounds.x, y: screenBounds.y + screenBounds.height }); // Bottom-left
    points.push({
      x: screenBounds.x + screenBounds.width,
      y: screenBounds.y + screenBounds.height,
    }); // Bottom-right

    // Edge midpoints for better coverage of thin/long components
    points.push({
      x: screenBounds.x + screenBounds.width / 2,
      y: screenBounds.y,
    }); // Top-center
    points.push({
      x: screenBounds.x + screenBounds.width / 2,
      y: screenBounds.y + screenBounds.height,
    }); // Bottom-center
    points.push({
      x: screenBounds.x,
      y: screenBounds.y + screenBounds.height / 2,
    }); // Left-center
    points.push({
      x: screenBounds.x + screenBounds.width,
      y: screenBounds.y + screenBounds.height / 2,
    }); // Right-center

    return points;
  }
}

// Singleton instance for convenience
export const hitTestService = new HitTestService();
