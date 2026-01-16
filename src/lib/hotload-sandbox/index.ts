/**
 * Hotload Sandbox Module
 *
 * A reusable module for safely loading and rendering dynamically generated
 * Svelte components with error isolation, timeout protection, and validation.
 *
 * @example
 * ```typescript
 * import { createHotloadSandbox } from '$lib/hotload-sandbox';
 *
 * const sandbox = createHotloadSandbox({
 *   logger: myLogger,
 *   importTimeout: 10000,
 *   onError: (error) => console.error('Component error:', error),
 * });
 *
 * // Register a component
 * await sandbox.registry.registerFromUpdate(componentUpdate);
 *
 * // In Svelte, use the ComponentContainer
 * // <ComponentContainer registry={sandbox.registry} />
 * ```
 */

// Types
export type {
  LoggerInterface,
  HotloadConfig,
  Position,
  Size,
  ComponentStatus,
  GeneratedComponent,
  ComponentUpdate,
  ErrorType,
  ComponentError,
  ImportResult,
  ValidationResult,
} from './types';

export { defaultLogger } from './types';

// Services
export { ErrorReporter, createErrorReporter } from './services/ErrorReporter';
export { ImportManager, createImportManager } from './services/ImportManager';
export { ComponentRegistry, createComponentRegistry } from './services/ComponentRegistry';

// Components
export { default as SafeComponent } from './components/SafeComponent.svelte';
export { default as ErrorPlaceholder } from './components/ErrorPlaceholder.svelte';
export { default as ComponentContainer } from './components/ComponentContainer.svelte';

// Factory function
import type { HotloadConfig } from './types';
import { ErrorReporter } from './services/ErrorReporter';
import { ImportManager } from './services/ImportManager';
import { ComponentRegistry } from './services/ComponentRegistry';

/**
 * Result of createHotloadSandbox containing all the services and components needed.
 */
export interface HotloadSandbox {
  /** The component registry for managing components */
  registry: ComponentRegistry;
  /** The import manager for loading components */
  importManager: ImportManager;
  /** The error reporter for tracking errors */
  errorReporter: ErrorReporter;
  /** The configuration used */
  config: HotloadConfig;
}

/**
 * Create a complete hotload sandbox with all services configured.
 *
 * @param config - Configuration options
 * @returns An object containing registry, importManager, and errorReporter
 *
 * @example
 * ```typescript
 * const sandbox = createHotloadSandbox({
 *   logger: myLogger,
 *   importTimeout: 15000,
 *   basePath: '/src/generated/',
 *   onError: (error) => {
 *     console.error('Component error:', error);
 *     // Send to error tracking service, etc.
 *   },
 * });
 *
 * // Use the registry to manage components
 * sandbox.registry.registerFromUpdate(update);
 *
 * // Query errors
 * const errors = sandbox.errorReporter.getAllErrors();
 *
 * // Clear cache if needed
 * sandbox.importManager.clearCache();
 * ```
 */
export function createHotloadSandbox(config?: HotloadConfig): HotloadSandbox {
  const resolvedConfig: HotloadConfig = {
    importTimeout: 10000,
    basePath: '/src/generated/',
    ...config,
  };

  const errorReporter = new ErrorReporter(resolvedConfig);
  const importManager = new ImportManager(resolvedConfig);
  const registry = new ComponentRegistry(resolvedConfig, importManager, errorReporter);

  return {
    registry,
    importManager,
    errorReporter,
    config: resolvedConfig,
  };
}
