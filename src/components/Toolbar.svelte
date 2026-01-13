<script lang="ts">
  import { onMount } from 'svelte';
  import { engine } from '../services/DrawingEngine';
  import { COLORS } from '../constants';
  import { panelWidth } from '../stores/panelStore';
  import type { DrawingState } from '../types';

  let state: DrawingState = engine.getState();

  onMount(() => {
    const unsubscribe = engine.subscribe((nextState) => {
      state = nextState;
    });
    return () => unsubscribe();
  });
</script>

<div
  class="fixed bottom-8 left-1/2 flex items-center gap-6 px-6 py-3 bg-neutral-900/80 backdrop-blur-md border border-neutral-700 rounded-full z-50 transition-transform duration-300 ease-out"
  style="transform: translateX(calc(-50% - {$panelWidth / 2}px));"
>
  <button
    class="p-2 rounded-lg transition-colors {state.currentStroke === null ? 'bg-neutral-800 text-blue-400' : 'text-neutral-400'}"
    title="Pen Tool"
  >
    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19l7-7 3 3-7 7-3-3z"></path><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z"></path><path d="M2 2l1.5 1.5"></path><circle cx="11" cy="11" r="2"></circle></svg>
  </button>

  <div class="w-[1px] h-6 bg-neutral-700"></div>

  <div class="flex gap-3">
    {#each COLORS as color}
      <button
        on:click={() => engine.setColor(color)}
        class="w-6 h-6 rounded-full border-2 transition-transform hover:scale-110 {state.currentColor === color ? 'border-white scale-125' : 'border-transparent'}"
        style="background-color: {color};"
      />
    {/each}
  </div>

  <div class="w-[1px] h-6 bg-neutral-700"></div>

  <button
    on:click={() => engine.undo()}
    class="p-2 text-neutral-400 hover:text-white transition-colors"
    title="Undo (Ctrl+Z)"
  >
    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7v6h6"></path><path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path></svg>
  </button>
</div>
