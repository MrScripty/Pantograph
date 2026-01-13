import type { SvelteComponent } from 'svelte';
import { Logger } from './Logger';

export interface CompiledComponent {
  id: string;
  source: string;
  component: typeof SvelteComponent | null;
  error: string | null;
}

/**
 * Runtime component "compiler" for dynamically loaded components.
 *
 * Note: True runtime Svelte compilation in the browser is complex and requires
 * bundling the Svelte compiler. This service provides a simplified approach:
 *
 * 1. For development: Components are written to src/generated/ and Vite's HMR
 *    handles the recompilation and hot-reload.
 *
 * 2. For production: A simpler approach where the backend writes components
 *    and we use dynamic imports to load them.
 *
 * The actual Svelte compilation happens through Vite's build process.
 */
class RuntimeCompilerClass {
  private compiledCache: Map<string, CompiledComponent> = new Map();
  private pendingImports: Map<string, Promise<CompiledComponent>> = new Map();

  /**
   * Attempt to dynamically import a component from the generated directory.
   * This works with Vite's dynamic import capabilities.
   */
  public async importComponent(path: string): Promise<CompiledComponent> {
    // Check cache first
    const cached = this.compiledCache.get(path);
    if (cached) {
      return cached;
    }

    // Check if there's already a pending import
    const pending = this.pendingImports.get(path);
    if (pending) {
      return pending;
    }

    // Create new import promise
    const importPromise = this.doImport(path);
    this.pendingImports.set(path, importPromise);

    try {
      const result = await importPromise;
      this.compiledCache.set(path, result);
      return result;
    } finally {
      this.pendingImports.delete(path);
    }
  }

  private async doImport(path: string): Promise<CompiledComponent> {
    const id = path.replace('.svelte', '').replace(/[/\\]/g, '_');

    try {
      // Use direct dynamic import with cache-busting timestamp
      // This allows importing components created at runtime
      // The absolute path from root works with Vite's dev server
      const timestamp = Date.now();
      const modulePath = `/src/generated/${path}?t=${timestamp}`;

      Logger.log('COMPONENT_IMPORTING', { path, modulePath });

      const module = (await import(/* @vite-ignore */ modulePath)) as { default: typeof SvelteComponent };

      Logger.log('COMPONENT_IMPORTED', { path, id });

      return {
        id,
        source: '', // Source not available through dynamic import
        component: module.default,
        error: null,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      Logger.log('COMPONENT_IMPORT_ERROR', { path, error: errorMessage }, 'error');

      return {
        id,
        source: '',
        component: null,
        error: errorMessage,
      };
    }
  }

  /**
   * Clear the cache for a specific component or all components
   */
  public clearCache(path?: string) {
    if (path) {
      this.compiledCache.delete(path);
    } else {
      this.compiledCache.clear();
    }
  }

  /**
   * Check if a component is cached
   */
  public isCached(path: string): boolean {
    return this.compiledCache.has(path);
  }

  /**
   * Get all cached component IDs
   */
  public getCachedIds(): string[] {
    return Array.from(this.compiledCache.keys());
  }

  /**
   * Create a simple component from source string.
   * This creates a wrapper component that renders the HTML directly.
   * Note: This is a fallback for when dynamic imports aren't working.
   */
  public createSimpleComponent(
    id: string,
    source: string
  ): CompiledComponent {
    // For now, we can't truly compile Svelte at runtime without the compiler
    // This would require bundling svelte/compiler which is large
    Logger.log('SIMPLE_COMPONENT_CREATED', { id, sourceLength: source.length });

    return {
      id,
      source,
      component: null, // Would need runtime compilation
      error: 'Runtime compilation not available - use dynamic imports instead',
    };
  }
}

export const RuntimeCompiler = new RuntimeCompilerClass();

/**
 * Alternative: Create a basic HTML component wrapper
 * This can render static HTML from the LLM's output
 */
export function createHtmlWrapper(html: string): string {
  // Extract and process the template part of a Svelte component
  // This is a very basic approach for when we can't compile
  const templateMatch = html.match(/<template[^>]*>([\s\S]*?)<\/template>/);
  if (templateMatch) {
    return templateMatch[1];
  }

  // If no template tags, assume the whole thing is HTML
  // Remove script and style blocks
  return html
    .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '')
    .replace(/<style[^>]*>[\s\S]*?<\/style>/gi, '')
    .trim();
}
