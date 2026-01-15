<script lang="ts">
  import { onMount } from 'svelte';
  import Rulers from './components/Rulers.svelte';
  import Canvas from './components/Canvas.svelte';
  import Toolbar from './components/Toolbar.svelte';
  import TopBar from './components/TopBar.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import HotLoadContainer from './components/HotLoadContainer.svelte';
  import ChunkPreview from './components/ChunkPreview.svelte';
  import { Logger } from './services/Logger';
  import { engine } from './services/DrawingEngine';
  import { panelWidth } from './stores/panelStore';

  onMount(() => {
    Logger.log('APP_MOUNTED', { version: '1.0.0-alpha' });

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        engine.undo();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<main class="relative w-screen h-screen overflow-hidden selection:bg-blue-500/30">
  <div class="fixed inset-0 canvas-grid pointer-events-none opacity-40 z-0"></div>

  <Canvas />
  <Rulers />
  <TopBar />
  <Toolbar />
  <SidePanel />

  <HotLoadContainer />

  <!-- Global modals -->
  <ChunkPreview />

  <div
    class="fixed bottom-4 text-[10px] text-neutral-600 uppercase tracking-widest pointer-events-none z-40 transition-[right] duration-300 ease-out"
    style="right: {$panelWidth + 16}px;"
  >
    Zenith System Active
  </div>
</main>
