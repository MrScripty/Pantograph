/**
 * Drawing Feature Module
 *
 * Canvas drawing system for creating visual inputs to the LLM.
 */

// Components
export { default as Canvas } from '../../components/Canvas.svelte';
export { default as Rulers } from '../../components/Rulers.svelte';
export { default as Toolbar } from '../../components/Toolbar.svelte';
export { default as ClearButton } from '../../components/ClearButton.svelte';

// Services
export { DrawingEngine, engine } from '../../services/DrawingEngine';
export {
  calculateBounds,
  findElementAtPosition,
  getDrawingCenter,
  doesDrawingOverlap,
  findTargetComponent,
  suggestComponentPosition,
} from '../../services/DrawingAnalyzer';
export type { DrawingBounds, ComponentPosition } from '../../services/DrawingAnalyzer';
export { canvasExport } from '../../services/CanvasExport';

// Stores
export { canvasPan, setPan, adjustPan, resetPan } from '../../stores/canvasStore';
export type { PanOffset } from '../../stores/canvasStore';
