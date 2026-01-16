#!/usr/bin/env node
/**
 * Import resolution validation for Svelte components
 *
 * Usage: node validate-imports.mjs <filepath> [projectRoot] [allowedPackagesJson]
 *
 * Parses imports from a Svelte file and validates they can be resolved:
 * - Checks npm packages against package.json dependencies
 * - Checks relative imports for file existence
 * - Supports $lib and other aliases
 *
 * Exits with code 0 if valid, code 1 if invalid.
 * Outputs JSON with validation results to stdout.
 */

import { init, parse } from 'es-module-lexer';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname, join } from 'path';

const filepath = process.argv[2];
const projectRoot = process.argv[3] || process.cwd();
const additionalAllowedJson = process.argv[4];

if (!filepath) {
  console.log(JSON.stringify({ valid: false, error: 'No filepath provided' }));
  process.exit(1);
}

// Initialize es-module-lexer
await init;

/**
 * Load allowed packages from package.json dependencies
 */
function getAllowedPackages(root) {
  const pkgPath = join(root, 'package.json');
  if (!existsSync(pkgPath)) {
    return new Set(['svelte', 'svelte/internal', 'svelte/store', 'svelte/motion', 'svelte/transition', 'svelte/animate', 'svelte/easing']);
  }

  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));
    const packages = new Set([
      ...Object.keys(pkg.dependencies || {}),
      ...Object.keys(pkg.devDependencies || {}),
      // Always allow svelte core packages
      'svelte',
      'svelte/internal',
      'svelte/internal/client',
      'svelte/store',
      'svelte/motion',
      'svelte/transition',
      'svelte/animate',
      'svelte/easing',
    ]);
    return packages;
  } catch (e) {
    console.error(`Warning: Failed to parse package.json: ${e.message}`);
    return new Set(['svelte', 'svelte/internal']);
  }
}

/**
 * Extract script content from Svelte file
 */
function extractScriptContent(source) {
  // Match <script> or <script lang="ts">
  const scriptRegex = /<script[^>]*>([\s\S]*?)<\/script>/gi;
  const matches = [];
  let match;

  while ((match = scriptRegex.exec(source)) !== null) {
    matches.push(match[1]);
  }

  return matches.join('\n');
}

/**
 * Calculate Levenshtein distance for typo suggestions
 */
function levenshteinDistance(a, b) {
  const matrix = [];

  for (let i = 0; i <= b.length; i++) {
    matrix[i] = [i];
  }
  for (let j = 0; j <= a.length; j++) {
    matrix[0][j] = j;
  }

  for (let i = 1; i <= b.length; i++) {
    for (let j = 1; j <= a.length; j++) {
      if (b.charAt(i - 1) === a.charAt(j - 1)) {
        matrix[i][j] = matrix[i - 1][j - 1];
      } else {
        matrix[i][j] = Math.min(
          matrix[i - 1][j - 1] + 1,
          matrix[i][j - 1] + 1,
          matrix[i - 1][j] + 1
        );
      }
    }
  }

  return matrix[b.length][a.length];
}

/**
 * Find similar package names for typo suggestions
 */
function findSimilarPackages(pkgName, allowed, maxDistance = 3) {
  const suggestions = [];

  for (const candidate of allowed) {
    // Only check base package names, not subpaths
    const candidateBase = candidate.split('/')[0];
    const distance = levenshteinDistance(pkgName.toLowerCase(), candidateBase.toLowerCase());

    if (distance <= maxDistance && distance > 0) {
      suggestions.push({ name: candidateBase, distance });
    }
  }

  // Sort by distance and return top 3
  return suggestions
    .sort((a, b) => a.distance - b.distance)
    .slice(0, 3)
    .map(s => s.name);
}

/**
 * Validate imports in the source code
 */
function validateImports(scriptContent, filepath, allowed) {
  const errors = [];

  try {
    const [imports] = parse(scriptContent);

    for (const imp of imports) {
      const specifier = imp.n;
      if (!specifier) continue;

      // Skip dynamic imports
      if (imp.d > -1) continue;

      // Relative imports - check file existence
      if (specifier.startsWith('./') || specifier.startsWith('../')) {
        const basePath = resolve(dirname(filepath), specifier);
        const extensions = ['', '.svelte', '.ts', '.js', '.mjs', '.json'];
        const found = extensions.some(ext => existsSync(basePath + ext));

        if (!found) {
          errors.push({
            type: 'relative',
            specifier,
            message: `Cannot resolve relative import: "${specifier}"`,
            line: getLineNumber(scriptContent, imp.ss)
          });
        }
        continue;
      }

      // $lib and other SvelteKit aliases - allow these
      if (specifier.startsWith('$lib/') ||
          specifier.startsWith('$app/') ||
          specifier.startsWith('$env/') ||
          specifier.startsWith('$service-worker')) {
        continue;
      }

      // Extract package name (handle scoped packages)
      const pkgName = specifier.startsWith('@')
        ? specifier.split('/').slice(0, 2).join('/')
        : specifier.split('/')[0];

      // Check if package is allowed
      if (!allowed.has(pkgName)) {
        const suggestions = findSimilarPackages(pkgName, allowed);
        let message = `Unknown package: "${pkgName}"`;

        if (suggestions.length > 0) {
          message += `. Did you mean: ${suggestions.map(s => `"${s}"`).join(', ')}?`;
        }

        errors.push({
          type: 'package',
          specifier,
          package: pkgName,
          message,
          suggestions,
          line: getLineNumber(scriptContent, imp.ss)
        });
      }
    }
  } catch (parseError) {
    // If es-module-lexer fails, the code has syntax errors
    // Those will be caught by the Svelte compiler, so we skip
    return [];
  }

  return errors;
}

/**
 * Get line number from character position
 */
function getLineNumber(source, position) {
  const lines = source.substring(0, position).split('\n');
  return lines.length;
}

// Main execution
try {
  const source = readFileSync(filepath, 'utf-8');
  const scriptContent = extractScriptContent(source);

  if (!scriptContent.trim()) {
    // No script content, nothing to validate
    console.log(JSON.stringify({ valid: true }));
    process.exit(0);
  }

  // Get allowed packages
  const allowed = getAllowedPackages(projectRoot);

  // Add any additional allowed packages from command line
  if (additionalAllowedJson) {
    try {
      const additional = JSON.parse(additionalAllowedJson);
      if (Array.isArray(additional)) {
        additional.forEach(pkg => allowed.add(pkg));
      }
    } catch (e) {
      // Ignore parse errors for additional packages
    }
  }

  const errors = validateImports(scriptContent, filepath, allowed);

  if (errors.length > 0) {
    // Format error message
    const errorMessages = errors.map(e => e.message).join('; ');

    console.log(JSON.stringify({
      valid: false,
      error: `Import validation failed: ${errorMessages}`,
      errors,
      line: errors[0]?.line
    }));
    process.exit(1);
  }

  console.log(JSON.stringify({ valid: true }));
  process.exit(0);

} catch (error) {
  console.log(JSON.stringify({
    valid: false,
    error: `Import validation error: ${error.message}`
  }));
  process.exit(1);
}
