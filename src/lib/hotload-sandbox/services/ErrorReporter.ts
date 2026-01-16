import type { ComponentError, ErrorType, LoggerInterface, HotloadConfig } from '../types';
import { defaultLogger } from '../types';

/**
 * Service for tracking and reporting component errors.
 * Provides a centralized place to collect, query, and subscribe to errors.
 */
export class ErrorReporter {
  private errors: ComponentError[] = [];
  private listeners: Array<(errors: ComponentError[]) => void> = [];
  private logger: LoggerInterface;
  private onError?: (error: ComponentError) => void;

  constructor(config?: HotloadConfig) {
    this.logger = config?.logger ?? defaultLogger;
    this.onError = config?.onError;
  }

  /**
   * Report a new component error.
   */
  public report(
    componentId: string,
    componentPath: string,
    errorType: ErrorType,
    errorMessage: string,
    source?: string,
    stack?: string
  ): ComponentError {
    const error: ComponentError = {
      componentId,
      componentPath,
      errorType,
      errorMessage,
      timestamp: Date.now(),
      source,
      stack,
    };

    this.errors.push(error);
    this.logger.log('COMPONENT_ERROR', error, 'error');

    // Call external error handler if configured
    this.onError?.(error);

    this.notify();
    return error;
  }

  /**
   * Create and report an error from an Error object.
   */
  public reportFromError(
    componentId: string,
    componentPath: string,
    errorType: ErrorType,
    error: Error | unknown,
    source?: string
  ): ComponentError {
    const errorMessage = error instanceof Error ? error.message : String(error);
    const stack = error instanceof Error ? error.stack : undefined;
    return this.report(componentId, componentPath, errorType, errorMessage, source, stack);
  }

  /**
   * Get all errors for a specific component.
   */
  public getErrorsForComponent(componentId: string): ComponentError[] {
    return this.errors.filter((e) => e.componentId === componentId);
  }

  /**
   * Get the most recent error for a component.
   */
  public getLatestError(componentId: string): ComponentError | undefined {
    const componentErrors = this.getErrorsForComponent(componentId);
    return componentErrors[componentErrors.length - 1];
  }

  /**
   * Get all errors.
   */
  public getAllErrors(): ComponentError[] {
    return [...this.errors];
  }

  /**
   * Get errors by type.
   */
  public getErrorsByType(errorType: ErrorType): ComponentError[] {
    return this.errors.filter((e) => e.errorType === errorType);
  }

  /**
   * Clear errors for a specific component or all errors.
   */
  public clearErrors(componentId?: string): void {
    if (componentId) {
      const before = this.errors.length;
      this.errors = this.errors.filter((e) => e.componentId !== componentId);
      this.logger.log('ERRORS_CLEARED', { componentId, cleared: before - this.errors.length });
    } else {
      const count = this.errors.length;
      this.errors = [];
      this.logger.log('ALL_ERRORS_CLEARED', { cleared: count });
    }
    this.notify();
  }

  /**
   * Check if a component has any errors.
   */
  public hasError(componentId: string): boolean {
    return this.errors.some((e) => e.componentId === componentId);
  }

  /**
   * Subscribe to error changes.
   * @returns Unsubscribe function
   */
  public subscribe(listener: (errors: ComponentError[]) => void): () => void {
    this.listeners.push(listener);
    // Immediately call with current errors
    listener([...this.errors]);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  /**
   * Format an error message suitable for displaying to the agent for retry.
   */
  public formatErrorForAgent(componentId: string): string | null {
    const error = this.getLatestError(componentId);
    if (!error) return null;

    let message = `Component "${componentId}" failed with ${error.errorType} error: ${error.errorMessage}`;

    if (error.source) {
      // Include a snippet of the source for context
      const lines = error.source.split('\n');
      const snippet = lines.slice(0, 10).join('\n');
      message += `\n\nSource snippet:\n${snippet}`;
      if (lines.length > 10) {
        message += `\n... (${lines.length - 10} more lines)`;
      }
    }

    return message;
  }

  private notify(): void {
    const errorsCopy = [...this.errors];
    this.listeners.forEach((l) => l(errorsCopy));
  }
}

/**
 * Create a standalone ErrorReporter instance.
 */
export function createErrorReporter(config?: HotloadConfig): ErrorReporter {
  return new ErrorReporter(config);
}
