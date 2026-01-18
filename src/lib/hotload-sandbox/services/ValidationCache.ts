/**
 * Validation cache for generated components.
 * Uses fast FNV-1a hashing to detect file changes and avoid unnecessary re-validation.
 */

/**
 * Fast FNV-1a hash (non-cryptographic)
 * ~0.1ms for typical component (~1KB)
 */
export function fastHash(str: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < str.length; i++) {
    hash ^= str.charCodeAt(i);
    hash = (hash * 0x01000193) >>> 0;
  }
  return hash.toString(16);
}

interface CacheEntry {
  hash: string;
  valid: boolean;
  error?: string;
}

const cache = new Map<string, CacheEntry>();

/**
 * Check if a file's validation state is cached and matches the current content hash.
 * Returns null if not cached or hash mismatch (needs re-validation).
 */
export function getValidationState(path: string, contentHash: string): CacheEntry | null {
  const entry = cache.get(path);
  if (entry && entry.hash === contentHash) {
    return entry;
  }
  return null; // Hash mismatch or not cached
}

/**
 * Store validation state for a file.
 */
export function setValidationState(path: string, hash: string, valid: boolean, error?: string): void {
  cache.set(path, { hash, valid, error });
}

/**
 * Clear the entire validation cache.
 * Called on HMR refresh to ensure changed files are re-validated.
 */
export function clearValidationCache(): void {
  cache.clear();
}

/**
 * Remove a specific path from the cache.
 * Useful when a file is deleted.
 */
export function invalidatePath(path: string): void {
  cache.delete(path);
}
