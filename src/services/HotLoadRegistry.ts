/**
 * HotLoadRegistry - Component registry with error isolation
 *
 * This module re-exports from the hotload-sandbox module while maintaining
 * backwards compatibility with existing code that imports from here.
 */

import {
  createHotloadSandbox,
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
const sandbox = createHotloadSandbox({
  logger: {
    log: (event: string, data?: unknown, level?: 'info' | 'warn' | 'error') => {
      Logger.log(event, data, level);
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

// Export the registry instance for backwards compatibility
export const componentRegistry = sandbox.registry;

// Export additional services for advanced usage
export const errorReporter = sandbox.errorReporter;
export const importManager = sandbox.importManager;

// Re-export the ComponentRegistry class for type usage
export { ComponentRegistry } from '$lib/hotload-sandbox';
