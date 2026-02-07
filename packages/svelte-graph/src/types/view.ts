// View and navigation types for the graph editor

export type ViewLevel = 'orchestration' | 'data-graph' | 'group';

export interface BreadcrumbItem {
  id: string;
  name: string;
  level: ViewLevel;
}

export interface ViewportState {
  x: number;
  y: number;
  zoom: number;
}

export interface ZoomTarget {
  nodeId: string;
  position: { x: number; y: number };
  bounds?: { width: number; height: number };
}

export interface AnimationConfig {
  duration: number;
  easing: string;
}

export const DEFAULT_ANIMATION: AnimationConfig = {
  duration: 300,
  easing: 'cubic-bezier(0.4, 0, 0.2, 1)',
};
