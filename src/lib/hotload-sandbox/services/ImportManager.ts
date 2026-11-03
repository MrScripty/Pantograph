import type { SvelteComponent, ComponentType } from 'svelte';
import type { ImportResult, ImportFailureCode, ValidationResult, LoggerInterface, HotloadConfig } from '../types.ts';
import { defaultLogger } from '../types.ts';
import { getComponentModules, refreshGlobModules, getModuleCount } from './GlobRegistry';
import { fastHash, getValidationState, setValidationState, invalidatePath } from './ValidationCache.ts';
import { invoke } from '@tauri-apps/api/core';

/**
 * Default import timeout in milliseconds.
 */
const DEFAULT_IMPORT_TIMEOUT = 10000;

/**
 * Default base path for generated components.
 */
const DEFAULT_BASE_PATH = '/src/generated/';

/**
 * Result from validate_component Tauri command.
 */
type PreImportValidation =
  | { valid: true }
  | { valid: false; error: string; failureCode: ImportFailureCode };

/**
 * HMR update listeners - called when components are hot-reloaded.
 */
type HmrUpdateListener = (updatedPaths: string[]) => void;
const hmrListeners: Set<HmrUpdateListener> = new Set();

/**
 * Set up HMR handling for generated components.
 * When a component file changes, Vite will trigger this handler.
 *
 * Note: The glob itself is now in GlobRegistry.ts which has its own HMR boundary.
 * This prevents the glob invalidation from cascading up to HotLoadRegistry and beyond.
 */
if (import.meta.hot) {
  // Listen for specific file updates via Vite's HMR
  import.meta.hot.on('vite:beforeUpdate', (payload) => {
    // Filter for updates to generated components
    const generatedUpdates = payload.updates
      .filter((update: { path: string }) => update.path.includes('/src/generated/'))
      .map((update: { path: string }) => update.path);

    if (generatedUpdates.length > 0) {
      // Refresh the glob to pick up changes (via GlobRegistry)
      refreshGlobModules();
      hmrListeners.forEach(listener => listener(generatedUpdates));
    }
  });

  // Listen for file deletions from our Vite plugin (generated-components-hmr)
  import.meta.hot.on('generated-component-deleted', (data: { file: string }) => {
    console.log('[HMR] Generated component deleted:', data.file);
    // Refresh the glob to remove the deleted file reference
    refreshGlobModules();
    // Notify listeners about the deletion
    hmrListeners.forEach(listener => listener([data.file]));
  });

  // Listen for new file creations from our Vite plugin
  import.meta.hot.on('generated-component-created', (data: { file: string }) => {
    console.log('[HMR] Generated component created:', data.file);
    // Refresh the glob to include the new file
    refreshGlobModules();
    // Notify listeners about the new file
    hmrListeners.forEach(listener => listener([data.file]));
  });

  // Listen for file updates from our Vite plugin (used during git undo/redo)
  import.meta.hot.on('generated-component-updated', (data: { file: string }) => {
    console.log('[HMR] Generated component updated:', data.file);
    // Clear validation cache for this file so it gets re-validated
    invalidatePath(data.file);
    // Refresh the glob in case file list changed
    refreshGlobModules();
    // Notify listeners about the update
    hmrListeners.forEach(listener => listener([data.file]));
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
  private latestImports = new Map<string, { promise: Promise<ImportResult>; pending: boolean }>();
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
    const pending = this.latestImports.get(path);
    if (pending?.pending) {
      this.logger.log('IMPORT_PENDING_REUSE', { path });
      return pending.promise;
    }

    // Start new import
    // Defer execution until this invocation owns the pending slot. Invalidating
    // the slot revokes publication/import admission, not the underlying IPC work.
    const importPromise: Promise<ImportResult> = Promise.resolve().then(() => this.doImport(
      path,
      () => this.latestImports.get(path)?.promise === importPromise,
    ));
    this.latestImports.set(path, { promise: importPromise, pending: true });

    try {
      const result = await importPromise;
      // Only cache successful imports
      if (result.success && this.latestImports.get(path)?.promise === importPromise) {
        this.cache.set(path, result);
      }
      return result;
    } finally {
      if (this.latestImports.get(path)?.promise === importPromise) {
        this.latestImports.get(path)!.pending = false;
      }
    }
  }

  /**
   * Join the latest already-requested generation, including its terminal failure.
   * This does not start a retry. Component-ID consumers use it when a shared
   * path was refreshed while their own candidate is still current.
   */
  public getCurrentImport(path: string): Promise<ImportResult> | undefined {
    return this.latestImports.get(path)?.promise;
  }

  /**
   * Validate a component file before importing.
   * Uses hash-based caching to avoid unnecessary re-validation.
   */
  private async validateBeforeImport(fullPath: string): Promise<PreImportValidation> {
    try {
      // Fetch the file content to compute hash
      const response = await fetch(fullPath);
      if (!response.ok) {
        return {
          valid: false,
          failureCode: 'validation-unavailable',
          error: `Component source could not be read (HTTP ${response.status}). Restore the source and retry.`,
        };
      }

      const content = await response.text();
      const contentHash = fastHash(content);

      // Check validation cache
      const cached = getValidationState(fullPath, contentHash);
      if (cached) {
        this.logger.log('VALIDATION_CACHE_HIT', { path: fullPath, valid: cached.valid });
        return cached.valid
          ? { valid: true }
          : { valid: false, failureCode: 'validation-invalid', error: cached.error ?? 'Component failed pre-import validation' };
      }

      // Hash differs or not cached - validate via Tauri
      this.logger.log('VALIDATION_RUNNING', { path: fullPath });

      // Pass the relative path - the Tauri backend will resolve to absolute path
      const result = await invoke<unknown>('validate_component', {
        relativePath: fullPath,
      });

      // The Rust command serializes { valid: bool, error: Option<String> }.
      // Decode the response before it can authorize executing a module. Extra
      // fields are ignored for additive diagnostics; no truthy coercion is valid.
      if (
        !result || typeof result !== 'object' || Array.isArray(result) ||
        !('valid' in result) || typeof result.valid !== 'boolean' ||
        ('error' in result && result.error != null && typeof result.error !== 'string')
      ) {
        return {
          valid: false,
          failureCode: 'validation-response-invalid',
          error: 'Component validation returned an invalid response. Restore the validation service and retry.',
        };
      }
      const error = 'error' in result && typeof result.error === 'string' ? result.error : undefined;
      setValidationState(fullPath, contentHash, result.valid, error);

      this.logger.log('VALIDATION_COMPLETE', {
        path: fullPath,
        valid: result.valid,
        error,
      });

      return result.valid
        ? { valid: true }
        : { valid: false, failureCode: 'validation-invalid', error: error ?? 'Component failed pre-import validation' };
    } catch (error) {
      this.logger.log('VALIDATION_UNAVAILABLE', { path: fullPath, error: String(error) }, 'warn');
      // Unavailability is not evidence that this candidate is valid. Do not
      // cache it: the next explicit retry must contact validation again.
      return {
        valid: false,
        failureCode: 'validation-unavailable',
        error: 'Component validation is unavailable. Restore the source and desktop validation service, then retry.',
      };
    }
  }

  /**
   * Perform the actual import with timeout.
   */
  private async doImport(path: string, isCurrent: () => boolean): Promise<ImportResult> {
    const startTime = Date.now();
    const fullPath = `${this.basePath}${path}`;

    this.logger.log('IMPORT_STARTING', { path, fullPath, timeout: this.importTimeout });

    try {
      // Validate the component before importing to prevent Vite freeze
      const preValidation = await this.validateBeforeImport(fullPath);
      if (!isCurrent()) return this.supersededResult(startTime);
      if (!preValidation.valid) {
        const duration = Date.now() - startTime;
        this.logger.log('IMPORT_BLOCKED_BY_VALIDATION', { path, error: preValidation.error }, 'error');
        return {
          success: false,
          component: null,
          error: preValidation.error,
          failureCode: preValidation.failureCode,
          duration,
        };
      }

      // First, try to use the glob-based import (supports HMR)
      const importer = getComponentModules()[fullPath];

      if (importer) {
        // Use glob-based import - this supports HMR
        const module = await this.importWithTimeout(importer);
        if (!isCurrent()) return this.supersededResult(startTime);
        const duration = Date.now() - startTime;

        const validation = this.validateModule(module);
        if (!validation.valid) {
          this.logger.log('IMPORT_VALIDATION_FAILED', { path, error: validation.error }, 'error');
          return {
            success: false,
            component: null,
            error: validation.error ?? 'Module validation failed',
            failureCode: 'validation-invalid',
            duration,
          };
        }

        const validatedModule = module as { default: ComponentType<SvelteComponent> };

        this.logger.log('IMPORT_SUCCESS', { path, duration, method: 'glob' });
        return {
          success: true,
          component: validatedModule.default,
          error: null,
          duration,
        };
      }

      // Fallback: If not in glob (new file created after server start),
      // use dynamic import with cache-busting timestamp
      this.logger.log('IMPORT_GLOB_MISS', { path, fullPath }, 'warn');

      const timestampedPath = `${fullPath}?t=${Date.now()}`;
      const module = await this.importWithTimeout(
        () => import(/* @vite-ignore */ timestampedPath),
      );
      if (!isCurrent()) return this.supersededResult(startTime);

      const duration = Date.now() - startTime;

      const validation = this.validateModule(module);
      if (!validation.valid) {
        this.logger.log('IMPORT_VALIDATION_FAILED', { path, error: validation.error }, 'error');
        return {
          success: false,
          component: null,
          error: validation.error ?? 'Module validation failed',
          failureCode: 'validation-invalid',
          duration,
        };
      }

      this.logger.log('IMPORT_SUCCESS', { path, duration, method: 'dynamic' });

      // Try to refresh glob to include the new file for future HMR
      this.refreshGlob();

      return {
        success: true,
        component: (module as { default: ComponentType<SvelteComponent> }).default,
        error: null,
        duration,
      };
    } catch (error) {
      if (!isCurrent()) return this.supersededResult(startTime);
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
        failureCode: isTimeout ? 'import-timeout' : 'import-failed',
        duration,
      };
    }
  }

  private supersededResult(startTime: number): ImportResult {
    return {
      success: false,
      component: null,
      error: 'Component load was superseded by a newer request.',
      failureCode: 'superseded',
      duration: Date.now() - startTime,
    };
  }

  private async importWithTimeout(importer: () => Promise<unknown>): Promise<unknown> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        importer(),
        new Promise<never>((_, reject) => {
          timer = setTimeout(
            () => reject(new Error(`Import timeout after ${this.importTimeout}ms`)),
            this.importTimeout,
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }

  /**
   * Refresh the glob to pick up newly created files.
   * This is called after a dynamic import fallback.
   */
  private refreshGlob(): void {
    try {
      refreshGlobModules();
      this.logger.log('GLOB_REFRESHED', { count: getModuleCount() });
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
      this.latestImports.delete(path);
      this.logger.log('CACHE_CLEARED', { path });
    } else {
      const count = this.cache.size;
      this.cache.clear();
      this.latestImports.clear();
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
    return Object.keys(getComponentModules()).map(p => p.replace(this.basePath, ''));
  }
}

/**
 * Create a standalone ImportManager instance.
 */
export function createImportManager(config?: HotloadConfig): ImportManager {
  return new ImportManager(config);
}
