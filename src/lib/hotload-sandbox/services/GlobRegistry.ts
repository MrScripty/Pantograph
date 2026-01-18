/**
 * GlobRegistry - Isolated glob module with HMR boundary
 *
 * This module isolates the import.meta.glob from the rest of the module graph.
 * When new files are added to /src/generated/, only THIS module gets invalidated,
 * not the entire HotLoadRegistry/ComponentContainer chain.
 *
 * The key insight is that import.meta.hot.accept() creates an HMR boundary that
 * prevents module invalidation from cascading to importers. By putting the glob
 * in its own module with accept(), we stop the cascade at this module.
 */

import { clearValidationCache } from './ValidationCache';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let componentModules: Record<string, () => Promise<any>> =
  import.meta.glob('/src/generated/**/*.svelte');

/**
 * Get the current glob result for component modules.
 * Returns a record of paths to dynamic import functions.
 */
export function getComponentModules(): Record<string, () => Promise<any>> {
  return componentModules;
}

/**
 * Refresh the glob to pick up new files.
 * Called when HMR detects changes in /src/generated/.
 * Also clears the validation cache to ensure changed files are re-validated.
 */
export function refreshGlobModules(): void {
  componentModules = import.meta.glob('/src/generated/**/*.svelte');
  // Clear validation cache so changed files will be re-validated
  clearValidationCache();
}

/**
 * Get the count of known modules.
 */
export function getModuleCount(): number {
  return Object.keys(componentModules).length;
}

/**
 * Check if a path is known to the glob.
 */
export function hasModule(fullPath: string): boolean {
  return fullPath in componentModules;
}

// HMR boundary - accept updates for THIS module only
// This is the critical part that prevents cascade to importers
if (import.meta.hot) {
  // Self-accepting module prevents cascade to importers
  import.meta.hot.accept();

  // Preserve the current glob state across HMR of this specific module
  if (import.meta.hot.data?.componentModules) {
    componentModules = import.meta.hot.data.componentModules;
  }

  // Save state before this module is disposed
  import.meta.hot.dispose((data) => {
    data.componentModules = componentModules;
  });
}
