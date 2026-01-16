<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { engine } from '../services/DrawingEngine';
  import { canvasExport } from '../services/CanvasExport';
  import { panelWidth } from '../stores/panelStore';
  import { interactionMode } from '../stores/interactionModeStore';
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
    ctx.clearRect(0, 0, cssWidth, cssHeight);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.lineWidth = 2;
    state.strokes.forEach(drawStroke);
    if (state.currentStroke) {
      drawStroke(state.currentStroke);
    }
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
      return { x: touch.clientX - rect.left, y: touch.clientY - rect.top };
    }
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  };

  const handleMouseDown = (e: MouseEvent) => {
    engine.startStroke(getPoint(e));
  };

  const handleMouseMove = (e: MouseEvent) => {
    engine.addPoint(getPoint(e));
  };

  const handleMouseUp = () => {
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

    window.addEventListener('resize', handleResize);
    return () => {
      unsubscribe();
      if (unsubscribePanel) unsubscribePanel();
      if (unsubscribeMode) unsubscribeMode();
      window.removeEventListener('resize', handleResize);
    };
  });

  onDestroy(() => {
    if (unsubscribePanel) unsubscribePanel();
    if (unsubscribeMode) unsubscribeMode();
  });
</script>

<canvas
  bind:this={canvas}
  class="fixed top-0 left-0 z-20 touch-none transition-[right] duration-300 ease-out {currentMode === 'draw' ? 'cursor-crosshair pointer-events-auto' : 'cursor-default pointer-events-none'}"
  style="width: 100vw; height: 100vh; clip-path: inset(0 {currentPanelWidth}px 0 0);"
  on:mousedown={handleMouseDown}
  on:mousemove={handleMouseMove}
  on:mouseup={handleMouseUp}
  on:mouseleave={handleMouseUp}
  on:touchstart|preventDefault={handleTouchStart}
  on:touchmove|preventDefault={handleTouchMove}
  on:touchend|preventDefault={handleTouchEnd}
></canvas>
