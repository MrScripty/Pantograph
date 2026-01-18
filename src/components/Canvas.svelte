<script lang="ts">
  import { onMount } from 'svelte';
  import { engine } from '../services/DrawingEngine';
  import { canvasExport } from '../services/CanvasExport';
  import { panelWidth } from '../stores/panelStore';
  import { interactionMode } from '../stores/interactionModeStore';
  import { canvasPan, adjustPan, type PanOffset } from '../stores/canvasStore';
  import type { DrawingState, Point, Stroke } from '../types';

  let canvas: HTMLCanvasElement | null = null;
  let ctx: CanvasRenderingContext2D | null = null;
  let state: DrawingState = engine.getState();
  let cssWidth = 0;
  let cssHeight = 0;
  let currentPanelWidth = 20;
  let currentMode: 'draw' | 'interact' = 'draw';
  let unsubscribePanel: (() => void) | null = null;
  let unsubscribeMode: (() => void) | null = null;
  let unsubscribePan: (() => void) | null = null;

  // Pan state
  let currentPan: PanOffset = { x: 0, y: 0 };
  let isPanning = false;
  let panStart: Point = { x: 0, y: 0 };

  const drawStroke = (stroke: Stroke) => {
    if (!ctx || stroke.points.length < 1) return;
    ctx.strokeStyle = stroke.color;
    ctx.beginPath();
    ctx.moveTo(stroke.points[0].x, stroke.points[0].y);
    for (let i = 1; i < stroke.points.length; i += 1) {
      ctx.lineTo(stroke.points[i].x, stroke.points[i].y);
    }
    ctx.stroke();
  };

  const resizeCanvas = () => {
    if (!canvas || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    // Canvas always uses full window size for consistent coordinates
    cssWidth = window.innerWidth;
    cssHeight = window.innerHeight;
    canvas.width = cssWidth * dpr;
    canvas.height = cssHeight * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  };

  const render = () => {
    if (!ctx) return;
    ctx.save();
    ctx.clearRect(0, 0, cssWidth, cssHeight);
    // Apply pan offset
    ctx.translate(currentPan.x, currentPan.y);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.lineWidth = 2;
    state.strokes.forEach(drawStroke);
    if (state.currentStroke) {
      drawStroke(state.currentStroke);
    }
    ctx.restore();
  };

  const syncState = (nextState: DrawingState) => {
    state = nextState;
    render();
  };

  const getPoint = (e: MouseEvent | TouchEvent): Point => {
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    if ('touches' in e) {
      const touch = e.touches[0] ?? e.changedTouches[0];
      if (!touch) return { x: 0, y: 0 };
      // Account for pan offset when drawing
      return {
        x: touch.clientX - rect.left - currentPan.x,
        y: touch.clientY - rect.top - currentPan.y,
      };
    }
    // Account for pan offset when drawing
    return {
      x: e.clientX - rect.left - currentPan.x,
      y: e.clientY - rect.top - currentPan.y,
    };
  };

  const getScreenPoint = (e: MouseEvent): Point => {
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  };

  const handleMouseDown = (e: MouseEvent) => {
    // Middle mouse button (button 1) starts panning
    if (e.button === 1) {
      e.preventDefault();
      isPanning = true;
      panStart = getScreenPoint(e);
      return;
    }
    // Left mouse button draws
    if (e.button === 0) {
      engine.startStroke(getPoint(e));
    }
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (isPanning) {
      const current = getScreenPoint(e);
      const dx = current.x - panStart.x;
      const dy = current.y - panStart.y;
      adjustPan(dx, dy);
      panStart = current;
      return;
    }
    engine.addPoint(getPoint(e));
  };

  const handleMouseUp = (e: MouseEvent) => {
    if (isPanning) {
      isPanning = false;
      return;
    }
    engine.endStroke();
  };

  const handleTouchStart = (e: TouchEvent) => {
    engine.startStroke(getPoint(e));
  };

  const handleTouchMove = (e: TouchEvent) => {
    engine.addPoint(getPoint(e));
  };

  const handleTouchEnd = () => {
    engine.endStroke();
  };

  onMount(() => {
    if (!canvas) return undefined;
    ctx = canvas.getContext('2d');
    if (!ctx) return undefined;

    canvasExport.setCanvas(canvas);

    resizeCanvas();
    render();

    const unsubscribe = engine.subscribe(syncState);
    const handleResize = () => {
      resizeCanvas();
      render();
    };

    // Subscribe to panel width for reactivity in template
    unsubscribePanel = panelWidth.subscribe((w) => {
      currentPanelWidth = w;
    });

    // Subscribe to interaction mode for pointer-events toggle
    unsubscribeMode = interactionMode.subscribe((mode) => {
      currentMode = mode;
    });

    // Subscribe to pan offset for canvas translation
    unsubscribePan = canvasPan.subscribe((pan) => {
      currentPan = pan;
      render();
    });

    window.addEventListener('resize', handleResize);
    return () => {
      unsubscribe();
      if (unsubscribePanel) unsubscribePanel();
      if (unsubscribeMode) unsubscribeMode();
      if (unsubscribePan) unsubscribePan();
      window.removeEventListener('resize', handleResize);
    };
  });

  // Note: Cleanup is handled by the onMount return function above.
  // No need for onDestroy since onMount's cleanup runs on component destruction.
</script>

<canvas
  bind:this={canvas}
  class="fixed top-0 left-0 z-20 touch-none transition-[right] duration-300 ease-out {isPanning ? 'cursor-grabbing' : currentMode === 'draw' ? 'cursor-crosshair pointer-events-auto' : 'cursor-default pointer-events-none'}"
  style="width: 100vw; height: 100vh; clip-path: inset(0 {currentPanelWidth}px 0 0);"
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onmouseleave={handleMouseUp}
  onauxclick={(e) => e.preventDefault()}
  ontouchstart={(e) => { e.preventDefault(); handleTouchStart(e); }}
  ontouchmove={(e) => { e.preventDefault(); handleTouchMove(e); }}
  ontouchend={(e) => { e.preventDefault(); handleTouchEnd(); }}
></canvas>
