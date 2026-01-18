/**
 * HotLoadRegistry - Component registry with error isolation and HMR support
 *
 * This module re-exports from the hotload-sandbox module while maintaining
 * backwards compatibility with existing code that imports from here.
 *
 * HMR (Hot Module Replacement) is automatically wired up to refresh
 * generated components when their files change, without full page reload.
 */

import {
  createHotloadSandbox,
  subscribeToHmrUpdates,
  type ComponentUpdate as SandboxComponentUpdate,
  type GeneratedComponent as SandboxGeneratedComponent,
  type Position as SandboxPosition,
  type Size as SandboxSize,
  type ComponentError,
} from '$lib/hotload-sandbox';
import { Logger } from './Logger';

// Re-export types for backwards compatibility
export type Position = SandboxPosition;
export type Size = SandboxSize;
export type ComponentUpdate = SandboxComponentUpdate;

// Extended GeneratedComponent type with backwards-compatible fields
export interface GeneratedComponent extends SandboxGeneratedComponent {
  // All fields from SandboxGeneratedComponent plus any legacy fields
}

// Create the sandbox instance with Pantograph's logger
// Use HMR data to preserve state across hot reloads
function createSandbox() {
  return createHotloadSandbox({
    logger: {
      log: (event: string, data?: unknown, level?: 'info' | 'warn' | 'error') => {
        Logger.log(event, (data ?? {}) as Record<string, unknown>, level);
      },
    },
    importTimeout: 10000,
    basePath: '/src/generated/',
    onError: (error: ComponentError) => {
      // Log errors to the activity log
      Logger.log('HOTLOAD_ERROR', {
        componentId: error.componentId,
        type: error.errorType,
        message: error.errorMessage,
      }, 'error');
    },
  });
}

// Preserve sandbox across HMR - check if we have previous instance data
let sandbox = import.meta.hot?.data?.sandbox ?? createSandbox();

// Store sandbox in HMR data for preservation across hot reloads
if (import.meta.hot) {
  import.meta.hot.data.sandbox = sandbox;
  // Accept HMR updates for this module
  import.meta.hot.accept();
}

// Export the registry instance for backwards compatibility
export const componentRegistry = sandbox.registry;

// Export additional services for advanced usage
export const errorReporter = sandbox.errorReporter;
export const importManager = sandbox.importManager;

// Re-export the ComponentRegistry class for type usage
export { ComponentRegistry } from '$lib/hotload-sandbox';

/**
 * Wire up HMR to automatically refresh components when their files change.
 * This prevents full page reloads and preserves application state.
 */
subscribeToHmrUpdates(async (updatedPaths: string[]) => {
  Logger.log('HMR_UPDATE_DETECTED', { paths: updatedPaths });

  // Refresh all components whose files were updated
  await componentRegistry.refreshByPaths(updatedPaths);
});

// LocalStorage key for component positions
const POSITIONS_STORAGE_KEY = 'pantograph-component-positions';

interface StoredPosition {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Save component positions to localStorage.
 * Called when components are moved or resized.
 */
export function saveComponentPositions(): void {
  const components = componentRegistry.getAll();
  const positions: Record<string, StoredPosition> = {};

  for (const comp of components) {
    positions[comp.path] = {
      x: comp.position.x,
      y: comp.position.y,
      width: comp.size.width,
      height: comp.size.height,
    };
  }

  try {
    localStorage.setItem(POSITIONS_STORAGE_KEY, JSON.stringify(positions));
    Logger.log('POSITIONS_SAVED', { count: Object.keys(positions).length });
  } catch (e) {
    Logger.log('POSITIONS_SAVE_FAILED', { error: e instanceof Error ? e.message : String(e) }, 'warn');
  }
}

/**
 * Load saved component positions from localStorage.
 */
function loadStoredPositions(): Record<string, StoredPosition> {
  try {
    const stored = localStorage.getItem(POSITIONS_STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (e) {
    Logger.log('POSITIONS_LOAD_FAILED', { error: e instanceof Error ? e.message : String(e) }, 'warn');
  }
  return {};
}

/**
 * Load generated components from disk and register them in the workspace.
 * Called on app startup to restore previous session's components.
 */
export async function loadWorkspace(): Promise<number> {
  // Dynamic import to avoid circular dependencies
  const { invoke } = await import('@tauri-apps/api/core');

  interface GeneratedComponentInfo {
    path: string;
    content: string;
  }

  try {
    const components = await invoke<GeneratedComponentInfo[]>('list_generated_components');

    if (components.length === 0) {
      Logger.log('WORKSPACE_EMPTY', {});
      return 0;
    }

    // Load saved positions
    const savedPositions = loadStoredPositions();

    // Default position offset for laying out components
    const defaultWidth = 400;
    const defaultHeight = 300;
    const margin = 20;
    let offsetX = 100;
    let offsetY = 100;

    for (const comp of components) {
      const id = `restored-${comp.path.replace(/[^a-zA-Z0-9]/g, '-')}`;

      // Use saved position or calculate a default
      const saved = savedPositions[comp.path];
      const position: Position = saved
        ? { x: saved.x, y: saved.y }
        : { x: offsetX, y: offsetY };
      const size: Size = saved
        ? { width: saved.width, height: saved.height }
        : { width: defaultWidth, height: defaultHeight };

      // Advance default position for next component without saved position
      if (!saved) {
        offsetX += defaultWidth + margin;
        if (offsetX > 1200) {
          offsetX = 100;
          offsetY += defaultHeight + margin;
        }
      }

      await componentRegistry.registerFromSource(
        id,
        comp.content,
        comp.path,
        position,
        size
      );
    }

    Logger.log('WORKSPACE_LOADED', { count: components.length });
    return components.length;
  } catch (e) {
    Logger.log('WORKSPACE_LOAD_FAILED', { error: e instanceof Error ? e.message : String(e) }, 'error');
    return 0;
  }
}

// Debounce timer for position saving
let savePositionsTimer: ReturnType<typeof setTimeout> | null = null;

/**
 * Subscribe to component changes and auto-save positions.
 * Uses debouncing to avoid excessive localStorage writes.
 */
componentRegistry.subscribe(() => {
  // Debounce to avoid saving on every minor update
  if (savePositionsTimer) {
    clearTimeout(savePositionsTimer);
  }
  savePositionsTimer = setTimeout(() => {
    saveComponentPositions();
  }, 1000);
});
