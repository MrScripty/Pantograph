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
 * Manages dynamic imports of Svelte components with timeout and validation.
 * Provides caching, timeout protection, and pre-render validation.
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

    try {
      // Use cache-busting timestamp for fresh imports
      const timestamp = Date.now();
      const modulePath = `${this.basePath}${path}?t=${timestamp}`;

      this.logger.log('IMPORT_STARTING', { path, modulePath, timeout: this.importTimeout });

      // Create timeout promise
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(
          () => reject(new Error(`Import timeout after ${this.importTimeout}ms`)),
          this.importTimeout
        );
      });

      // Race import against timeout
      const module = (await Promise.race([
        import(/* @vite-ignore */ modulePath),
        timeoutPromise,
      ])) as { default: ComponentType<SvelteComponent> };

      const duration = Date.now() - startTime;

      // Validate the imported module
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

      this.logger.log('IMPORT_SUCCESS', { path, duration });

      return {
        success: true,
        component: module.default,
        error: null,
        duration,
      };
    } catch (error) {
      const duration = Date.now() - startTime;
      const errorMessage = error instanceof Error ? error.message : String(error);

      // Determine if it was a timeout
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
}

/**
 * Create a standalone ImportManager instance.
 */
export function createImportManager(config?: HotloadConfig): ImportManager {
  return new ImportManager(config);
}
