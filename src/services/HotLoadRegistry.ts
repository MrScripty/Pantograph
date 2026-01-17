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
