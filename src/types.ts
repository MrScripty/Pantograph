import type { SvelteComponent } from 'svelte';

export enum ToolType {
  PEN = 'PEN',
}

export interface Point {
  x: number;
  y: number;
}

export interface Stroke {
  points: Point[];
  color: string;
  tool: ToolType;
}

export interface DrawingState {
  strokes: Stroke[];
  currentStroke: Stroke | null;
  currentColor: string;
  isDrawing: boolean;
}

export interface WideEvent {
  timestamp: number;
  type: string;
  payload: any;
  severity: 'info' | 'warn' | 'error';
}

export interface DynamicComponent {
  id: string;
  component: typeof SvelteComponent;
  props?: Record<string, unknown>;
}
