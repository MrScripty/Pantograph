<script lang="ts">
  import { componentRegistry } from '../services/HotLoadRegistry';
  import { engine } from '../services/DrawingEngine';
  import { resetPan } from '../stores/canvasStore';
  import { Logger } from '../services/Logger';

  const handleClear = () => {
    // Clear all generated components
    componentRegistry.clear();
    // Clear the drawing
    engine.clearStrokes();
    // Reset canvas pan
    resetPan();
    Logger.log('WORKSPACE_CLEARED', { message: 'All components, drawing, and pan reset' });
  };
</script>

<button
  on:click={handleClear}
  class="fixed bottom-8 left-8 p-3 bg-neutral-900/80 backdrop-blur-md border border-neutral-700 rounded-full z-50 text-neutral-400 hover:text-white hover:border-neutral-600 transition-colors"
  title="Clear workspace"
>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="20"
    height="20"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M3 6h18"></path>
    <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path>
    <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path>
    <line x1="10" y1="11" x2="10" y2="17"></line>
    <line x1="14" y1="11" x2="14" y2="17"></line>
  </svg>
</button>
