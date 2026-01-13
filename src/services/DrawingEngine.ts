import { DrawingState, Stroke, Point, ToolType } from '../types';
import { Logger } from './Logger';

export class DrawingEngine {
  private state: DrawingState = {
    strokes: [],
    currentStroke: null,
    currentColor: '#ffffff',
    isDrawing: false,
  };

  private listeners: Array<(state: DrawingState) => void> = [];

  public subscribe(callback: (state: DrawingState) => void) {
    this.listeners.push(callback);
    callback({ ...this.state });
    return () => {
      this.listeners = this.listeners.filter((listener) => listener !== callback);
    };
  }

  private notify() {
    this.listeners.forEach((listener) => listener({ ...this.state }));
  }

  public setColor(color: string) {
    this.state.currentColor = color;
    Logger.log('COLOR_CHANGED', { color });
    this.notify();
  }

  public startStroke(point: Point) {
    this.state.isDrawing = true;
    this.state.currentStroke = {
      points: [point],
      color: this.state.currentColor,
      tool: ToolType.PEN,
    };
    Logger.log('STROKE_STARTED', { point, color: this.state.currentColor });
    this.notify();
  }

  public addPoint(point: Point) {
    if (!this.state.isDrawing || !this.state.currentStroke) return;

    this.state.currentStroke.points.push(point);
    this.notify();
  }

  public endStroke() {
    if (this.state.currentStroke) {
      this.state.strokes.push(this.state.currentStroke);
      Logger.log('STROKE_COMPLETED', { pointCount: this.state.currentStroke.points.length });
    }
    this.state.isDrawing = false;
    this.state.currentStroke = null;
    this.notify();
  }

  public undo() {
    if (this.state.strokes.length > 0) {
      const removed = this.state.strokes.pop();
      Logger.log('UNDO_ACTION', { removedStrokeColor: removed?.color });
      this.notify();
    }
  }

  public clearStrokes() {
    this.state.strokes = [];
    this.state.currentStroke = null;
    this.state.isDrawing = false;
    Logger.log('STROKES_CLEARED', { message: 'All strokes cleared after UI generation' });
    this.notify();
  }

  public getState() {
    return { ...this.state };
  }
}

export const engine = new DrawingEngine();
