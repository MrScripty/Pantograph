import type {
  GeneratedComponent,
  ComponentUpdate,
  Position,
  Size,
  ComponentStatus,
  LoggerInterface,
  HotloadConfig,
} from '../types';
import { defaultLogger } from '../types';
import { ImportManager } from './ImportManager';
import { ErrorReporter } from './ErrorReporter';

/**
 * Registry for managing dynamically loaded components.
 * Tracks component state, handles imports, and notifies subscribers of changes.
 */
export class ComponentRegistry {
  private components: GeneratedComponent[] = [];
  private listeners: Array<(comps: GeneratedComponent[]) => void> = [];
  private logger: LoggerInterface;
  private importManager: ImportManager;
  private errorReporter: ErrorReporter;

  constructor(
    config?: HotloadConfig,
    importManager?: ImportManager,
    errorReporter?: ErrorReporter
  ) {
    this.logger = config?.logger ?? defaultLogger;
    this.importManager = importManager ?? new ImportManager(config);
    this.errorReporter = errorReporter ?? new ErrorReporter(config);
  }

  /**
   * Register a component from source code with position.
   * Attempts to import the component and tracks its status.
   */
  public async registerFromSource(
    id: string,
    source: string,
    path: string,
    position: Position,
    size: Size
  ): Promise<void> {
    // Create initial loading entry
    const loadingComponent: GeneratedComponent = {
      id,
      component: null,
      source,
      path,
      position,
      size,
      status: 'loading',
      props: this.createPositionProps(position, size),
    };

    // Update or add to registry
    const existingIndex = this.components.findIndex((c) => c.id === id);
    if (existingIndex >= 0) {
      this.components[existingIndex] = loadingComponent;
    } else {
      this.components.push(loadingComponent);
    }
    this.notify();

    // Clear any previous errors for this component
    this.errorReporter.clearErrors(id);

    // Attempt to import the component
    const result = await this.importManager.importComponent(path);

    // Validate the component
    const validation = this.importManager.validateComponent(result.component);

    // Determine final status and error
    let status: ComponentStatus = 'ready';
    let error: string | undefined;

    if (!result.success) {
      status = 'error';
      error = result.error ?? 'Import failed';
      this.errorReporter.report(
        id,
        path,
        result.error?.includes('timeout') ? 'timeout' : 'import',
        error,
        source
      );
    } else if (!validation.valid) {
      status = 'error';
      error = validation.error;
      this.errorReporter.report(id, path, 'validation', error ?? 'Validation failed', source);
    }

    // Create final component entry
    const finalComponent: GeneratedComponent = {
      id,
      component: validation.valid ? result.component : null,
      source,
      path,
      position,
      size,
      status,
      error,
      props: this.createPositionProps(position, size),
    };

    // Update in registry
    const idx = this.components.findIndex((c) => c.id === id);
    if (idx >= 0) {
      this.components[idx] = finalComponent;
    }

    this.logger.log(
      status === 'ready' ? 'COMPONENT_REGISTERED' : 'COMPONENT_REGISTRATION_FAILED',
      { id, path, status, error }
    );

    this.notify();
  }

  /**
   * Register a component from a ComponentUpdate object.
   */
  public async registerFromUpdate(update: ComponentUpdate): Promise<void> {
    await this.registerFromSource(
      update.id,
      update.source,
      update.path,
      update.position,
      update.size
    );
  }

  /**
   * Set a render error for a component.
   * Called by the UI when a component fails to render.
   */
  public setRenderError(id: string, errorMessage: string): void {
    const comp = this.components.find((c) => c.id === id);
    if (comp) {
      comp.renderError = errorMessage;
      comp.status = 'error';

      this.errorReporter.report(id, comp.path, 'render', errorMessage, comp.source);

      this.logger.log('COMPONENT_RENDER_ERROR', { id, error: errorMessage }, 'error');
      this.notify();
    }
  }

  /**
   * Clear render error for a component (e.g., on retry).
   */
  public clearRenderError(id: string): void {
    const comp = this.components.find((c) => c.id === id);
    if (comp && comp.renderError) {
      comp.renderError = undefined;
      // Only reset status if there's no import error
      if (!comp.error && comp.component) {
        comp.status = 'ready';
      }
      this.notify();
    }
  }

  /**
   * Update a component's position.
   */
  public updatePosition(id: string, x: number, y: number): void {
    const comp = this.components.find((c) => c.id === id);
    if (comp) {
      comp.position = { x, y };
      comp.props = this.createPositionProps(comp.position, comp.size);
      this.logger.log('COMPONENT_POSITION_UPDATED', { id, x, y });
      this.notify();
    }
  }

  /**
   * Update a component's size.
   */
  public updateSize(id: string, width: number, height: number): void {
    const comp = this.components.find((c) => c.id === id);
    if (comp) {
      comp.size = { width, height };
      comp.props = this.createPositionProps(comp.position, comp.size);
      this.logger.log('COMPONENT_SIZE_UPDATED', { id, width, height });
      this.notify();
    }
  }

  /**
   * Unregister a component.
   */
  public unregister(id: string): void {
    const before = this.components.length;
    this.components = this.components.filter((c) => c.id !== id);
    if (this.components.length < before) {
      this.errorReporter.clearErrors(id);
      this.logger.log('COMPONENT_UNREGISTERED', { id });
      this.notify();
    }
  }

  /**
   * Retry loading a component.
   */
  public async retry(id: string): Promise<void> {
    const comp = this.components.find((c) => c.id === id);
    if (comp) {
      // Clear cache to force re-import
      this.importManager.clearCache(comp.path);
      this.errorReporter.clearErrors(id);

      // Re-register
      await this.registerFromSource(id, comp.source, comp.path, comp.position, comp.size);
    }
  }

  /**
   * Subscribe to component changes.
   * @returns Unsubscribe function
   */
  public subscribe(listener: (comps: GeneratedComponent[]) => void): () => void {
    this.listeners.push(listener);
    listener([...this.components]);
    return () => {
      this.listeners = this.listeners.filter((cb) => cb !== listener);
    };
  }

  /**
   * Get all registered components.
   */
  public getAll(): GeneratedComponent[] {
    return [...this.components];
  }

  /**
   * Get a component by ID.
   */
  public getById(id: string): GeneratedComponent | undefined {
    return this.components.find((c) => c.id === id);
  }

  /**
   * Get all components with errors.
   */
  public getErrored(): GeneratedComponent[] {
    return this.components.filter((c) => c.status === 'error');
  }

  /**
   * Get the ErrorReporter instance.
   */
  public getErrorReporter(): ErrorReporter {
    return this.errorReporter;
  }

  /**
   * Get the ImportManager instance.
   */
  public getImportManager(): ImportManager {
    return this.importManager;
  }

  /**
   * Clear all components.
   */
  public clear(): void {
    this.components = [];
    this.errorReporter.clearErrors();
    this.importManager.clearCache();
    this.logger.log('REGISTRY_CLEARED');
    this.notify();
  }

  /**
   * Refresh components that match the given paths.
   * Called when HMR detects file changes to re-import updated components.
   * @param updatedPaths - Full paths of updated files (e.g., '/src/generated/Button.svelte')
   */
  public async refreshByPaths(updatedPaths: string[]): Promise<void> {
    // Normalize paths - remove base path prefix if present
    const normalizedPaths = updatedPaths.map(p => {
      // Handle both full paths and relative paths
      if (p.startsWith('/src/generated/')) {
        return p.replace('/src/generated/', '');
      }
      return p;
    });

    // Find components that match any of the updated paths
    const componentsToRefresh = this.components.filter(comp =>
      normalizedPaths.some(p => comp.path === p || comp.path.endsWith(p))
    );

    if (componentsToRefresh.length === 0) {
      this.logger.log('HMR_NO_MATCHING_COMPONENTS', { updatedPaths });
      return;
    }

    this.logger.log('HMR_REFRESHING_COMPONENTS', {
      count: componentsToRefresh.length,
      ids: componentsToRefresh.map(c => c.id),
    });

    // Refresh each component
    for (const comp of componentsToRefresh) {
      // Clear cache to force fresh import
      this.importManager.clearCache(comp.path);
      this.errorReporter.clearErrors(comp.id);

      // Re-import the component
      const result = await this.importManager.reimportComponent(comp.path);

      if (result.success && result.component) {
        // Update the component in registry
        comp.component = result.component;
        comp.status = 'ready';
        comp.error = undefined;
        comp.renderError = undefined;

        this.logger.log('HMR_COMPONENT_REFRESHED', { id: comp.id, path: comp.path });
      } else {
        comp.status = 'error';
        comp.error = result.error ?? 'HMR refresh failed';

        this.logger.log('HMR_COMPONENT_REFRESH_FAILED', {
          id: comp.id,
          path: comp.path,
          error: result.error,
        }, 'error');
      }
    }

    // Notify subscribers of the changes
    this.notify();
  }

  /**
   * Refresh a single component by ID.
   * Useful for manual refresh or retry operations.
   */
  public async refreshById(id: string): Promise<boolean> {
    const comp = this.components.find(c => c.id === id);
    if (!comp) {
      this.logger.log('REFRESH_COMPONENT_NOT_FOUND', { id }, 'warn');
      return false;
    }

    await this.refreshByPaths([comp.path]);
    return this.getById(id)?.status === 'ready';
  }

  private createPositionProps(position: Position, size: Size): Record<string, unknown> {
    return {
      style: `position: absolute; left: ${position.x}px; top: ${position.y}px; width: ${size.width}px; height: ${size.height}px;`,
    };
  }

  private notify(): void {
    const componentsCopy = [...this.components];
    this.listeners.forEach((listener) => listener(componentsCopy));
  }
}

/**
 * Create a standalone ComponentRegistry instance.
 */
export function createComponentRegistry(
  config?: HotloadConfig,
  importManager?: ImportManager,
  errorReporter?: ErrorReporter
): ComponentRegistry {
  return new ComponentRegistry(config, importManager, errorReporter);
}
