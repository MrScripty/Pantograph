<script lang="ts">
  import { onMount } from 'svelte';
  import SidePanel from './components/SidePanel.svelte';
  import ChunkPreview from './components/ChunkPreview.svelte';
  import WorkbenchShell from './components/workbench/WorkbenchShell.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Logger } from './services/Logger';
  import { loadWorkspace } from './services/HotLoadRegistry';
  import { undoStore } from './stores/undoStore';
  import { loadLastGraph } from './stores/graphSessionStore';
  import { linkModeActive, cancelLinkMode } from './stores/linkStore';
  import { startDiagnosticsStore, stopDiagnosticsStore } from './stores/diagnosticsStore';

  // Set up the @pantograph/svelte-graph context so package components
  // (GenericNode, ReconnectableEdge, etc.) can access stores via useGraphContext().
  import { createGraphContextFromStores } from '@pantograph/svelte-graph';
  import { backend, registry, workflowStores, viewStores, sessionStores } from './stores/storeInstances';

  createGraphContextFromStores(backend, registry, {
    workflow: workflowStores,
    view: viewStores,
    session: sessionStores,
  });

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
    startDiagnosticsStore();

    // Load previously generated components from disk
    loadWorkspace().then((count) => {
      if (count > 0) {
        Logger.log('WORKSPACE_RESTORED', { count });
      }
    });

    // Load the last opened workflow/graph
    loadLastGraph().then(() => {
      Logger.log('GRAPH_SESSION_RESTORED', {});
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      // Cancel link mode on Escape
      if (e.key === 'Escape' && $linkModeActive) {
        e.preventDefault();
        cancelLinkMode();
        return;
      }

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
          // Plain Ctrl+Z → Unified undo
          e.preventDefault();
          undoStore.undo();
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

    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      stopDiagnosticsStore();
    };
  });
</script>

<main class="relative h-screen w-screen overflow-hidden">
  <WorkbenchShell />

  <!-- Always visible developer/admin panel -->
  <SidePanel />

  <!-- Global modals -->
  <ChunkPreview />

  <!-- Link mode overlay - visible in all views -->
  {#if $linkModeActive}
    <div class="link-mode-overlay">
      <!-- Visual dimming only - no click capture -->
      <div class="link-mode-backdrop"></div>
      <div class="link-mode-instructions">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
        </svg>
        <span>Click a highlighted element, or press Escape to cancel</span>
      </div>
    </div>
  {/if}

</main>

<style>
  /* Link mode overlay styles */
  .link-mode-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 9999;
    pointer-events: none;
  }

  .link-mode-backdrop {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.2);
    pointer-events: none; /* Visual only - clicks pass through */
  }

  .link-mode-instructions {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    background: #1e293b;
    border: 1px solid #06b6d4;
    border-radius: 8px;
    color: #e2e8f0;
    font-size: 14px;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
    pointer-events: none;
    opacity: 0.9;
    transition: opacity 0.15s ease;
  }

  /* Fade to nearly invisible when cursor moves anywhere on screen */
  .link-mode-overlay:hover .link-mode-instructions {
    opacity: 0.05;
  }

  .link-mode-instructions :global(svg) {
    color: #06b6d4;
  }

  /* Global style for linkable elements during link mode */
  :global([data-linkable-id].link-mode-highlight) {
    outline: 2px solid #06b6d4 !important;
    outline-offset: 2px;
    animation: link-pulse 1.5s infinite;
    cursor: pointer !important;
    position: relative;
    z-index: 10000;
  }

  @keyframes link-pulse {
    0%, 100% {
      outline-color: #06b6d4;
      box-shadow: 0 0 0 0 rgba(6, 182, 212, 0.4);
    }
    50% {
      outline-color: #22d3ee;
      box-shadow: 0 0 0 4px rgba(6, 182, 212, 0);
    }
  }
</style>
