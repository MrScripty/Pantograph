<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { engine } from '../services/DrawingEngine';
  import { AgentService } from '../services/AgentService';
  import { LLMService } from '../services/LLMService';
  import { COLORS } from '../constants';
  import { panelWidth } from '../stores/panelStore';
  import { interactionMode, toggleInteractionMode } from '../stores/interactionModeStore';
  import type { DrawingState } from '../types';
  import { Logger } from '../services/Logger';
  import { componentRegistry, importManager } from '../services/HotLoadRegistry';
  import { refreshGlobModules } from '$lib/hotload-sandbox/services/GlobRegistry';

  interface VersionResult {
    success: boolean;
    message: string;
    affected_file: string | null;
  }

  let state: DrawingState = $state(engine.getState());
  let currentMode: 'draw' | 'interact' = $state('draw');

  onMount(() => {
    const unsubscribe = engine.subscribe((nextState) => {
      state = nextState;
    });
    const unsubscribeMode = interactionMode.subscribe((mode) => {
      currentMode = mode;
    });

    return () => {
      unsubscribe();
      unsubscribeMode();
    };
  });

  const handleClear = () => {
    // Clear both the canvas and the activity log
    engine.clearStrokes();
    AgentService.clearActivityLog();
    LLMService.clearHistory();
  };

  /**
   * Refresh a component after a git history operation (undo/redo).
   * This clears the cache and re-imports the affected component.
   */
  async function refreshComponentAfterGitOp(affectedFile: string) {
    // Small delay to let Vite's module graph update after the file change
    // The hotUpdate hook in our Vite plugin handles the file deletion/creation,
    // but we need a moment for the glob to refresh
    await new Promise(resolve => setTimeout(resolve, 100));

    // Refresh glob to detect new/deleted files
    refreshGlobModules();

    // Clear import cache for the affected file
    importManager.clearCache(affectedFile);

    // Check if file still exists in glob (it may have been deleted by undo)
    const knownPaths = importManager.getKnownPaths();
    const fileExists = knownPaths.some(p => p === affectedFile || p.endsWith(affectedFile));

    if (fileExists) {
      // File exists - refresh the component to show the reverted version
      await componentRegistry.refreshByPaths([affectedFile]);
      Logger.log('COMPONENT_REFRESHED_AFTER_GIT_OP', { affectedFile });
    } else {
      // File was deleted (undo of a create operation) - unregister components using this path
      const components = componentRegistry.getAll();
      const toUnregister = components.filter(c =>
        c.path === affectedFile || c.path.endsWith(affectedFile)
      );
      for (const comp of toUnregister) {
        componentRegistry.unregister(comp.id);
        Logger.log('COMPONENT_UNREGISTERED_AFTER_GIT_OP', { id: comp.id, path: affectedFile });
      }
    }
  }

  const handleComponentUndo = async () => {
    try {
      const result = await invoke<VersionResult>('undo_component_change');
      if (result.success) {
        Logger.info('Component change undone', result.message);
        if (result.affected_file) {
          await refreshComponentAfterGitOp(result.affected_file);
        }
      }
    } catch (e) {
      Logger.debug('Component undo', e instanceof Error ? e.message : String(e));
    }
  };

  const handleComponentRedo = async () => {
    try {
      const result = await invoke<VersionResult>('redo_component_change');
      if (result.success) {
        Logger.info('Component change redone', result.message);
        if (result.affected_file) {
          await refreshComponentAfterGitOp(result.affected_file);
        }
      }
    } catch (e) {
      Logger.debug('Component redo', e instanceof Error ? e.message : String(e));
    }
  };
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
        onclick={() => engine.setColor(color)}
        class="w-6 h-6 rounded-full border-2 transition-transform hover:scale-110 {state.currentColor === color ? 'border-white scale-125' : 'border-transparent'}"
        style="background-color: {color};"
        aria-label="Select {color} color"
      ></button>
    {/each}
  </div>

  <div class="w-[1px] h-6 bg-neutral-700"></div>

  <button
    onclick={() => engine.undo()}
    class="p-2 text-neutral-400 hover:text-white transition-colors"
    title="Undo Drawing (Ctrl+Z)"
  >
    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7v6h6"></path><path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path></svg>
  </button>

  <button
    onclick={handleClear}
    class="p-2 text-neutral-400 hover:text-red-400 transition-colors"
    title="Clear Canvas & History"
  >
    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"></path><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>
  </button>

  <div class="w-[1px] h-6 bg-neutral-700"></div>

  <!-- Component Undo/Redo (git versioning) -->
  <div class="flex items-center gap-1">
    <button
      onclick={handleComponentUndo}
      class="p-2 text-neutral-400 hover:text-white transition-colors"
      title="Undo Component Change (Alt+Ctrl+Z)"
    >
      <!-- Undo2 icon -->
      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11"/></svg>
    </button>
    <button
      onclick={handleComponentRedo}
      class="p-2 text-neutral-400 hover:text-white transition-colors"
      title="Redo Component Change (Ctrl+Shift+Z)"
    >
      <!-- Redo2 icon -->
      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 14 5-5-5-5"/><path d="M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13"/></svg>
    </button>
  </div>

  <div class="w-[1px] h-6 bg-neutral-700"></div>

  <!-- Draw/Interact Mode Toggle -->
  <button
    onclick={toggleInteractionMode}
    class="relative flex items-center gap-1 p-1 bg-neutral-800 rounded-full"
    title="Toggle Draw/Interact Mode (Tab)"
  >
    <!-- Draw mode icon (pen) -->
    <div
      class="p-1.5 rounded-full transition-all duration-200 {currentMode === 'draw' ? 'bg-blue-500 text-white' : 'text-neutral-400'}"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19l7-7 3 3-7 7-3-3z"></path><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z"></path><path d="M2 2l1.5 1.5"></path><circle cx="11" cy="11" r="2"></circle></svg>
    </div>
    <!-- Interact mode icon (cursor) -->
    <div
      class="p-1.5 rounded-full transition-all duration-200 {currentMode === 'interact' ? 'bg-blue-500 text-white' : 'text-neutral-400'}"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z"></path><path d="M13 13l6 6"></path></svg>
    </div>
  </button>
</div>
