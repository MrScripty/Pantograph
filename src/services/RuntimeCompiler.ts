/**
 * RuntimeCompiler - Dynamic component importing with timeout protection
 *
 * This module re-exports from the hotload-sandbox module while maintaining
 * backwards compatibility with existing code that imports from here.
 */

import { ImportManager, type ImportResult } from '$lib/hotload-sandbox';

// Create a default instance for backwards compatibility
const defaultImportManager = new ImportManager({
  importTimeout: 10000,
  basePath: '/src/generated/',
});

// Export as RuntimeCompiler for backwards compatibility
export const RuntimeCompiler = {
  importComponent: (path: string) => defaultImportManager.importComponent(path),
  clearCache: (path?: string) => defaultImportManager.clearCache(path),
  isCached: (path: string) => defaultImportManager.isCached(path),
  getCachedIds: () => defaultImportManager.getCachedPaths(),
  validateComponent: (component: unknown) => defaultImportManager.validateComponent(component),
};

// Re-export types
export type { ImportResult };
export type CompiledComponent = ImportResult & { id: string; source: string };

// Re-export the ImportManager class for advanced usage
export { ImportManager } from '$lib/hotload-sandbox';

// Re-export the createHtmlWrapper utility (keeping original functionality)
export function createHtmlWrapper(html: string): string {
  const templateMatch = html.match(/<template[^>]*>([\s\S]*?)<\/template>/);
  if (templateMatch) {
    return templateMatch[1];
  }

  return html
    .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '')
    .replace(/<style[^>]*>[\s\S]*?<\/style>/gi, '')
    .trim();
}
