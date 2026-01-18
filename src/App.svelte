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
  import CommitTimeline from './components/CommitTimeline.svelte';
  import NodeGraph from './components/NodeGraph.svelte';
  import WorkflowGraph from './components/WorkflowGraph.svelte';
  import NodePalette from './components/NodePalette.svelte';
  import WorkflowToolbar from './components/WorkflowToolbar.svelte';
  import ArchitectureGraph from './components/ArchitectureGraph.svelte';
  import ArchitectureToolbar from './components/ArchitectureToolbar.svelte';
  import ArchitectureLegend from './components/ArchitectureLegend.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Logger } from './services/Logger';
  import { engine } from './services/DrawingEngine';
  import { panelWidth } from './stores/panelStore';
  import { toggleInteractionMode } from './stores/interactionModeStore';
  import { viewMode, toggleViewMode } from './stores/viewModeStore';
  import { loadWorkspace } from './services/HotLoadRegistry';
  import { undoStore } from './stores/undoStore';

  async function handleComponentUndo() {
    try {
      const result = await invoke<{ success: boolean; message: string }>('undo_component_change');
      if (result.success) {
        Logger.log('COMPONENT_UNDO', { message: result.message });
      }
    } catch (e) {
      Logger.log('COMPONENT_UNDO_FAILED', { error: e instanceof Error ? e.message : String(e) }, 'warn');
    }
  }

  async function handleComponentRedo() {
    try {
      const result = await invoke<{ success: boolean; message: string }>('redo_component_change');
      if (result.success) {
        Logger.log('COMPONENT_REDO', { message: result.message });
      }
    } catch (e) {
      Logger.log('COMPONENT_REDO_FAILED', { error: e instanceof Error ? e.message : String(e) }, 'warn');
    }
  }

  onMount(() => {
    Logger.log('APP_MOUNTED', { version: '1.0.0-alpha' });

    // Load previously generated components from disk
    loadWorkspace().then((count) => {
      if (count > 0) {
        Logger.log('WORKSPACE_RESTORED', { count });
      }
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      const isCtrl = e.ctrlKey || e.metaKey;

      // Handle all Ctrl+Z variants
      if (isCtrl && e.key === 'z') {
        if (e.shiftKey && !e.altKey) {
          // Ctrl+Shift+Z → Unified undo (unhide commits, etc.)
          e.preventDefault();
          undoStore.undo();
        } else if (e.altKey && !e.shiftKey) {
          // Alt+Ctrl+Z → Component undo
          e.preventDefault();
          handleComponentUndo();
        } else if (!e.shiftKey && !e.altKey) {
          // Plain Ctrl+Z → Canvas drawing undo
          e.preventDefault();
          engine.undo();
        }
        return;
      }

      // Ctrl+Y → Component redo
      if (isCtrl && e.key === 'y' && !e.altKey && !e.shiftKey) {
        e.preventDefault();
        handleComponentRedo();
        return;
      }

      // Alt+Ctrl+Y → Unified redo
      if (isCtrl && e.altKey && e.key === 'y') {
        e.preventDefault();
        undoStore.redo();
        return;
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
      <CommitTimeline />
      <Toolbar />
      <ClearButton />
      <HotLoadContainer />
    </div>
  {:else if $viewMode === 'node-graph'}
    <div class="absolute inset-0" transition:fade={{ duration: 200 }}>
      <NodeGraph />
    </div>
  {:else if $viewMode === 'workflow'}
    <div class="absolute inset-0 flex flex-col" transition:fade={{ duration: 200 }}>
      <WorkflowToolbar />
      <div class="flex-1 flex overflow-hidden">
        <NodePalette />
        <div class="flex-1">
          <WorkflowGraph />
        </div>
      </div>
    </div>
  {:else if $viewMode === 'architecture'}
    <div class="absolute inset-0 flex flex-col" transition:fade={{ duration: 200 }}>
      <ArchitectureToolbar />
      <div class="flex-1 flex overflow-hidden">
        <ArchitectureLegend />
        <div class="flex-1">
          <ArchitectureGraph />
        </div>
      </div>
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
    {:else if $viewMode === 'node-graph'}
      Node Graph View (Ctrl+` to switch)
    {:else if $viewMode === 'workflow'}
      Workflow Editor (Ctrl+` to switch)
    {:else if $viewMode === 'architecture'}
      Architecture View (Ctrl+` to switch)
    {/if}
  </div>
</main>
