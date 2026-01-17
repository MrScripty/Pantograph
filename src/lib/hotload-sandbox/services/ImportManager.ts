import type { SvelteComponent, ComponentType } from 'svelte';
import type { ImportResult, ValidationResult, LoggerInterface, HotloadConfig } from '../types';
import { defaultLogger } from '../types';

/**
 * Default import timeout in milliseconds.
 */
const DEFAULT_IMPORT_TIMEOUT = 10000;

/**
 * Default base path for generated components.
 */
const DEFAULT_BASE_PATH = '/src/generated/';

/**
 * Glob-based component modules for HMR support.
 * Vite tracks these modules and provides native HMR when they change.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let componentModules: Record<string, () => Promise<any>> =
  import.meta.glob('/src/generated/**/*.svelte');

/**
 * HMR update listeners - called when components are hot-reloaded.
 */
type HmrUpdateListener = (updatedPaths: string[]) => void;
const hmrListeners: Set<HmrUpdateListener> = new Set();

/**
 * Set up HMR handling for generated components.
 * When a component file changes, Vite will trigger this handler.
 */
if (import.meta.hot) {
  // Listen for specific file updates via Vite's HMR
  import.meta.hot.on('vite:beforeUpdate', (payload) => {
    // Filter for updates to generated components
    const generatedUpdates = payload.updates
      .filter((update: { path: string }) => update.path.includes('/src/generated/'))
      .map((update: { path: string }) => update.path);

    if (generatedUpdates.length > 0) {
      // Refresh the glob to pick up changes
      componentModules = import.meta.glob('/src/generated/**/*.svelte');
      hmrListeners.forEach(listener => listener(generatedUpdates));
    }
  });
}

/**
 * Subscribe to HMR updates for generated components.
 * Returns an unsubscribe function.
 */
export function subscribeToHmrUpdates(listener: HmrUpdateListener): () => void {
  hmrListeners.add(listener);
  return () => hmrListeners.delete(listener);
}

/**
 * Manages dynamic imports of Svelte components with timeout and validation.
 * Uses import.meta.glob() for native Vite HMR support.
 */
export class ImportManager {
  private cache: Map<string, ImportResult> = new Map();
  private pendingImports: Map<string, Promise<ImportResult>> = new Map();
  private logger: LoggerInterface;
  private importTimeout: number;
  private basePath: string;

  constructor(config?: HotloadConfig) {
    this.logger = config?.logger ?? defaultLogger;
    this.importTimeout = config?.importTimeout ?? DEFAULT_IMPORT_TIMEOUT;
    this.basePath = config?.basePath ?? DEFAULT_BASE_PATH;
  }

  /**
   * Import a component from the generated directory with timeout protection.
   */
  public async importComponent(path: string): Promise<ImportResult> {
    // Check cache first (only return cached if successful)
    const cached = this.cache.get(path);
    if (cached && cached.success) {
      this.logger.log('IMPORT_CACHE_HIT', { path });
      return cached;
    }

    // Check if there's already a pending import for this path
    const pending = this.pendingImports.get(path);
    if (pending) {
      this.logger.log('IMPORT_PENDING_REUSE', { path });
      return pending;
    }

    // Start new import
    const importPromise = this.doImport(path);
    this.pendingImports.set(path, importPromise);

    try {
      const result = await importPromise;
      // Only cache successful imports
      if (result.success) {
        this.cache.set(path, result);
      }
      return result;
    } finally {
      this.pendingImports.delete(path);
    }
  }

  /**
   * Perform the actual import with timeout.
   */
  private async doImport(path: string): Promise<ImportResult> {
    const startTime = Date.now();
    const fullPath = `${this.basePath}${path}`;

    this.logger.log('IMPORT_STARTING', { path, fullPath, timeout: this.importTimeout });

    try {
      // First, try to use the glob-based import (supports HMR)
      const importer = componentModules[fullPath];

      if (importer) {
        // Use glob-based import - this supports HMR
        const timeoutPromise = new Promise<never>((_, reject) => {
          setTimeout(
            () => reject(new Error(`Import timeout after ${this.importTimeout}ms`)),
            this.importTimeout
          );
        });

        const module = await Promise.race([importer(), timeoutPromise]);
        const duration = Date.now() - startTime;

        const validation = this.validateModule(module);
        if (!validation.valid) {
          this.logger.log('IMPORT_VALIDATION_FAILED', { path, error: validation.error }, 'error');
          return {
            success: false,
            component: null,
            error: validation.error ?? 'Module validation failed',
            duration,
          };
        }

        this.logger.log('IMPORT_SUCCESS', { path, duration, method: 'glob' });
        return {
          success: true,
          component: module.default,
          error: null,
          duration,
        };
      }

      // Fallback: If not in glob (new file created after server start),
      // use dynamic import with cache-busting timestamp
      this.logger.log('IMPORT_GLOB_MISS', { path, fullPath }, 'warn');

      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(
          () => reject(new Error(`Import timeout after ${this.importTimeout}ms`)),
          this.importTimeout
        );
      });

      // Use timestamp for cache-busting on new files
      const timestampedPath = `${fullPath}?t=${Date.now()}`;
      const module = (await Promise.race([
        import(/* @vite-ignore */ timestampedPath),
        timeoutPromise,
      ])) as { default: ComponentType<SvelteComponent> };

      const duration = Date.now() - startTime;

      const validation = this.validateModule(module);
      if (!validation.valid) {
        this.logger.log('IMPORT_VALIDATION_FAILED', { path, error: validation.error }, 'error');
        return {
          success: false,
          component: null,
          error: validation.error ?? 'Module validation failed',
          duration,
        };
      }

      this.logger.log('IMPORT_SUCCESS', { path, duration, method: 'dynamic' });

      // Try to refresh glob to include the new file for future HMR
      this.refreshGlob();

      return {
        success: true,
        component: module.default,
        error: null,
        duration,
      };
    } catch (error) {
      const duration = Date.now() - startTime;
      const errorMessage = error instanceof Error ? error.message : String(error);
      const isTimeout = errorMessage.includes('timeout');

      this.logger.log(
        isTimeout ? 'IMPORT_TIMEOUT' : 'IMPORT_ERROR',
        { path, error: errorMessage, duration },
        'error'
      );

      return {
        success: false,
        component: null,
        error: errorMessage,
        duration,
      };
    }
  }

  /**
   * Refresh the glob to pick up newly created files.
   * This is called after a dynamic import fallback.
   */
  private refreshGlob(): void {
    try {
      componentModules = import.meta.glob('/src/generated/**/*.svelte');
      this.logger.log('GLOB_REFRESHED', { count: Object.keys(componentModules).length });
    } catch (e) {
      this.logger.log('GLOB_REFRESH_FAILED', { error: String(e) }, 'warn');
    }
  }

  /**
   * Validate that an imported module is a valid Svelte component.
   */
  private validateModule(module: unknown): ValidationResult {
    if (!module || typeof module !== 'object') {
      return { valid: false, error: 'Module is not an object' };
    }

    const mod = module as Record<string, unknown>;

    if (!('default' in mod)) {
      return { valid: false, error: 'Module has no default export' };
    }

    const defaultExport = mod.default;

    // Check if it's a valid component type
    // Svelte 5 components can be functions or objects with specific properties
    if (typeof defaultExport === 'function') {
      return { valid: true };
    }

    if (typeof defaultExport === 'object' && defaultExport !== null) {
      // Svelte 5 compiled components may be objects with render method or $$ property
      const comp = defaultExport as Record<string, unknown>;
      if ('$$' in comp || 'render' in comp || typeof comp === 'function') {
        return { valid: true };
      }
    }

    // Check for common mistakes
    if (typeof defaultExport === 'string') {
      return {
        valid: false,
        error: `Default export is a string ("${(defaultExport as string).slice(0, 50)}..."), not a component. Did you assign a string to a variable and try to use it as <Component />?`,
      };
    }

    if (typeof defaultExport === 'number') {
      return {
        valid: false,
        error: `Default export is a number (${defaultExport}), not a component.`,
      };
    }

    if (defaultExport === null) {
      return {
        valid: false,
        error: 'Default export is null, not a component.',
      };
    }

    return {
      valid: false,
      error: `Default export is not a valid Svelte component (got ${typeof defaultExport})`,
    };
  }

  /**
   * Validate a component before rendering.
   * Use this for additional pre-render checks.
   */
  public validateComponent(component: unknown): ValidationResult {
    if (!component) {
      return { valid: false, error: 'Component is null or undefined' };
    }

    if (typeof component === 'string') {
      return {
        valid: false,
        error: `Cannot render a string as a component. Got: "${component.slice(0, 50)}..."`,
      };
    }

    if (typeof component === 'number') {
      return {
        valid: false,
        error: `Cannot render a number as a component. Got: ${component}`,
      };
    }

    if (typeof component !== 'function' && typeof component !== 'object') {
      return {
        valid: false,
        error: `Invalid component type: ${typeof component}. Expected a Svelte component.`,
      };
    }

    return { valid: true };
  }

  /**
   * Clear the cache for a specific component or all components.
   */
  public clearCache(path?: string): void {
    if (path) {
      this.cache.delete(path);
      this.logger.log('CACHE_CLEARED', { path });
    } else {
      const count = this.cache.size;
      this.cache.clear();
      this.logger.log('CACHE_CLEARED_ALL', { count });
    }
  }

  /**
   * Check if a component is cached.
   */
  public isCached(path: string): boolean {
    return this.cache.has(path);
  }

  /**
   * Get all cached component paths.
   */
  public getCachedPaths(): string[] {
    return Array.from(this.cache.keys());
  }

  /**
   * Force re-import a component (bypasses cache).
   */
  public async reimportComponent(path: string): Promise<ImportResult> {
    this.clearCache(path);
    return this.importComponent(path);
  }

  /**
   * Get all component paths known to the glob.
   */
  public getKnownPaths(): string[] {
    return Object.keys(componentModules).map(p => p.replace(this.basePath, ''));
  }
}

/**
 * Create a standalone ImportManager instance.
 */
export function createImportManager(config?: HotloadConfig): ImportManager {
  return new ImportManager(config);
}
