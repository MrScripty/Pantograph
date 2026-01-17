<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import Rulers from './components/Rulers.svelte';
  import Canvas from './components/Canvas.svelte';
  import Toolbar from './components/Toolbar.svelte';
  import TopBar from './components/TopBar.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import HotLoadContainer from './components/HotLoadContainer.svelte';
  import ChunkPreview from './components/ChunkPreview.svelte';
  import ClearButton from './components/ClearButton.svelte';
  import NodeGraph from './components/NodeGraph.svelte';
  import { Logger } from './services/Logger';
  import { engine } from './services/DrawingEngine';
  import { panelWidth } from './stores/panelStore';
  import { toggleInteractionMode } from './stores/interactionModeStore';
  import { viewMode, toggleViewMode } from './stores/viewModeStore';

  onMount(() => {
    Logger.log('APP_MOUNTED', { version: '1.0.0-alpha' });

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        engine.undo();
      }
      // Toggle between canvas and node-graph views with Ctrl+`
      if (e.ctrlKey && e.key === '`') {
        e.preventDefault();
        toggleViewMode();
        return;
      }
      // Toggle between draw and interact modes with Tab key (only in canvas view)
      if (e.key === 'Tab' && $viewMode === 'canvas') {
        e.preventDefault();
        toggleInteractionMode();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<main class="relative w-screen h-screen overflow-hidden selection:bg-blue-500/30">
  <div class="fixed inset-0 canvas-grid pointer-events-none opacity-40 z-0"></div>

  {#if $viewMode === 'canvas'}
    <div class="absolute inset-0" transition:fade={{ duration: 200 }}>
      <Canvas />
      <Rulers />
      <Toolbar />
      <ClearButton />
      <HotLoadContainer />
    </div>
  {:else}
    <div class="absolute inset-0" transition:fade={{ duration: 200 }}>
      <NodeGraph />
    </div>
  {/if}

  <!-- Always visible components -->
  <TopBar />
  <SidePanel />

  <!-- Global modals -->
  <ChunkPreview />

  <div
    class="fixed bottom-4 text-[10px] text-neutral-600 uppercase tracking-widest pointer-events-none z-40 transition-[right] duration-300 ease-out"
    style="right: {$panelWidth + 16}px;"
  >
    {#if $viewMode === 'canvas'}
      Zenith System Active
    {:else}
      Node Graph View (Ctrl+` to switch)
    {/if}
  </div>
</main>
