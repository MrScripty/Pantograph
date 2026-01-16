import type { SvelteComponent, ComponentType } from 'svelte';

// ============================================================================
// Configuration Interfaces
// ============================================================================

/**
 * Logger interface for pluggable logging.
 * Implement this to integrate with your app's logging system.
 */
export interface LoggerInterface {
  log(event: string, data?: unknown, level?: 'info' | 'warn' | 'error'): void;
}

/**
 * Default console-based logger implementation.
 */
export const defaultLogger: LoggerInterface = {
  log(event: string, data?: unknown, level: 'info' | 'warn' | 'error' = 'info') {
    const prefix = `[hotload-sandbox] ${event}`;
    if (level === 'error') {
      console.error(prefix, data);
    } else if (level === 'warn') {
      console.warn(prefix, data);
    } else {
      console.log(prefix, data);
    }
  },
};

/**
 * Configuration for the hotload sandbox module.
 */
export interface HotloadConfig {
  /** Logger instance for logging events. Defaults to console. */
  logger?: LoggerInterface;
  /** Timeout in ms for dynamic imports. Default: 10000 (10 seconds) */
  importTimeout?: number;
  /** Callback when a component error occurs */
  onError?: (error: ComponentError) => void;
  /** Base path for generated components. Default: '/src/generated/' */
  basePath?: string;
}

// ============================================================================
// Component Types
// ============================================================================

/**
 * Position of a component on the canvas.
 */
export interface Position {
  x: number;
  y: number;
}

/**
 * Size of a component.
 */
export interface Size {
  width: number;
  height: number;
}

/**
 * Status of a component in the registry.
 */
export type ComponentStatus = 'loading' | 'ready' | 'error';

/**
 * A component registered in the hotload system.
 */
export interface GeneratedComponent {
  /** Unique identifier for the component */
  id: string;
  /** The Svelte component constructor, or null if failed to load */
  component: ComponentType<SvelteComponent> | null;
  /** Original source code */
  source: string;
  /** File path relative to generated directory */
  path: string;
  /** Position on canvas */
  position: Position;
  /** Size of component area */
  size: Size;
  /** Props to pass to the component */
  props?: Record<string, unknown>;
  /** Current status */
  status: ComponentStatus;
  /** Import/compile error message */
  error?: string;
  /** Runtime render error message */
  renderError?: string;
}

/**
 * Update payload from the backend when a component is created/modified.
 */
export interface ComponentUpdate {
  id: string;
  path: string;
  position: Position;
  size: Size;
  source: string;
}

// ============================================================================
// Error Types
// ============================================================================

/**
 * Types of errors that can occur during component lifecycle.
 */
export type ErrorType = 'import' | 'validation' | 'render' | 'timeout';

/**
 * Detailed error information for a component.
 */
export interface ComponentError {
  /** ID of the component that errored */
  componentId: string;
  /** File path of the component */
  componentPath: string;
  /** Type of error */
  errorType: ErrorType;
  /** Human-readable error message */
  errorMessage: string;
  /** When the error occurred */
  timestamp: number;
  /** Source code that caused the error (if available) */
  source?: string;
  /** Stack trace (if available) */
  stack?: string;
}

// ============================================================================
// Import Result Types
// ============================================================================

/**
 * Result of attempting to import a component.
 */
export interface ImportResult {
  /** Whether the import succeeded */
  success: boolean;
  /** The component constructor if successful */
  component: ComponentType<SvelteComponent> | null;
  /** Error message if failed */
  error: string | null;
  /** Time taken to import in ms */
  duration: number;
}

/**
 * Validation result for pre-render checks.
 */
export interface ValidationResult {
  /** Whether the component is valid */
  valid: boolean;
  /** Error message if invalid */
  error?: string;
  /** Warnings that don't prevent rendering */
  warnings?: string[];
}
