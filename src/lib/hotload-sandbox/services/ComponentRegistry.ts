import type {
  GeneratedComponent,
  ComponentUpdate,
  Position,
  Size,
  ImportFailureCode,
  LoggerInterface,
  HotloadConfig,
} from '../types.ts';
import { defaultLogger } from '../types.ts';
import { ImportManager } from './ImportManager.ts';
import { ErrorReporter } from './ErrorReporter.ts';

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
  // Candidate identity owns publication and retains the failed input for retry.
  private candidates = new Map<string, ComponentUpdate>();

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
    await this.loadCandidate({ id, source, path, position, size });
  }

  private async loadCandidate(update: ComponentUpdate): Promise<boolean> {
    const { id, source, path, position, size } = update;
    const candidate: ComponentUpdate = { id, source, path, position, size };
    this.candidates.set(id, candidate);
    const previous = this.getById(id);
    // A replacement is tentative. Keep the accepted constructor and its source,
    // geometry and props intact until the candidate has passed admission.
    const loading: GeneratedComponent = previous?.component
      ? { ...previous, pendingUpdate: { status: 'loading' } }
      : {
          ...update,
          component: null,
          status: 'loading',
          props: this.createPositionProps(position, size),
        };
    this.publish(loading);
    this.errorReporter.clearErrors(id);

    let result = previous
      ? await this.importManager.reimportComponent(path)
      : await this.importManager.importComponent(path);
    // Another component ID can refresh the same path without superseding this
    // candidate. Join that requested generation; never retry validation here.
    while (!result.success && result.failureCode === 'superseded' && this.candidates.get(id) === candidate) {
      const currentImport = this.importManager.getCurrentImport(path);
      if (!currentImport) break;
      result = await currentImport;
    }
    if (this.candidates.get(id) !== candidate) {
      this.logger.log('COMPONENT_LOAD_SUPERSEDED', { id, path });
      return false;
    }

    const validation = result.success
      ? this.importManager.validateComponent(result.component)
      : null;
    if (!result.success || !validation?.valid) {
      const error = result.error ?? validation?.error ?? 'Component validation failed';
      const failureCode: ImportFailureCode = result.success ? 'validation-invalid' : result.failureCode;
      const current = this.getById(id);
      if (!current) return false;
      this.publish(current.component
        ? { ...current, pendingUpdate: { status: 'error', error, failureCode } }
        : { ...current, status: 'error', error, pendingUpdate: undefined });
      this.errorReporter.report(
        id, path,
        failureCode.startsWith('validation-') ? 'validation' : failureCode === 'import-timeout' ? 'timeout' : 'import',
        error, source,
      );
      this.logger.log('COMPONENT_REGISTRATION_FAILED', { id, path, failureCode, error });
      return false;
    }

    this.candidates.delete(id);
    this.publish({
      ...candidate,
      component: result.component,
      status: 'ready',
      props: this.createPositionProps(candidate.position, candidate.size),
    });
    this.logger.log('COMPONENT_REGISTERED', { id, path });
    return true;
  }

  private publish(component: GeneratedComponent): void {
    const index = this.components.findIndex(c => c.id === component.id);
    if (index >= 0) this.components[index] = component;
    else this.components.push(component);
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
      const candidate = this.candidates.get(id);
      if (candidate) candidate.position = comp.position;
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
      const candidate = this.candidates.get(id);
      if (candidate) candidate.size = comp.size;
      comp.props = this.createPositionProps(comp.position, comp.size);
      this.logger.log('COMPONENT_SIZE_UPDATED', { id, width, height });
      this.notify();
    }
  }

  /**
   * Unregister a component.
   */
  public unregister(id: string): void {
    this.candidates.delete(id);
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
    const candidate = this.candidates.get(id) ?? this.getById(id);
    if (candidate) await this.loadCandidate(candidate);
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
    return this.components.filter((c) => c.status === 'error' || c.pendingUpdate?.status === 'error');
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
    this.candidates.clear();
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
      normalizedPaths.some(p => {
        const candidatePath = this.candidates.get(comp.id)?.path;
        return comp.path === p || comp.path.endsWith(p) || candidatePath === p;
      })
    ).map(component => ({ component, candidate: this.candidates.get(component.id) }));

    if (componentsToRefresh.length === 0) {
      this.logger.log('HMR_NO_MATCHING_COMPONENTS', { updatedPaths });
      return;
    }

    this.logger.log('HMR_REFRESHING_COMPONENTS', {
      count: componentsToRefresh.length,
      ids: componentsToRefresh.map(({ component }) => component.id),
    });

    for (const { component, candidate } of componentsToRefresh) {
      // A queued refresh has no authority after removal, clear, or a newer
      // registration/refresh. Check both identities after earlier items finish.
      if (this.getById(component.id) !== component || this.candidates.get(component.id) !== candidate) {
        this.logger.log('COMPONENT_REFRESH_SUPERSEDED', { id: component.id, path: component.path });
        continue;
      }
      await this.loadCandidate(candidate ?? component);
    }
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

    return this.loadCandidate(this.candidates.get(id) ?? comp);
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
