#!/usr/bin/env node
/**
 * esbuild-based validation for Svelte component imports
 *
 * Usage: node validate-esbuild.mjs <filepath> [projectRoot]
 *
 * Uses esbuild to attempt bundling the script block, catching all
 * import resolution errors. This is the most thorough validation
 * but also the slowest.
 *
 * Exits with code 0 if valid, code 1 if invalid.
 * Outputs JSON with validation results to stdout.
 */

import * as esbuild from 'esbuild';
import { readFileSync, writeFileSync, unlinkSync, existsSync } from 'fs';
import { join, dirname, basename } from 'path';

const filepath = process.argv[2];
const projectRoot = process.argv[3] || process.cwd();

if (!filepath) {
  console.log(JSON.stringify({ valid: false, error: 'No filepath provided' }));
  process.exit(1);
}

/**
 * Extract script content from Svelte file, handling TypeScript
 */
function extractScriptContent(source) {
  // Match <script> or <script lang="ts">
  const scriptRegex = /<script(\s+[^>]*)?>(([\s\S]*?))<\/script>/gi;
  const results = [];
  let match;

  while ((match = scriptRegex.exec(source)) !== null) {
    const attrs = match[1] || '';
    const content = match[2];
    const isTypeScript = attrs.includes('lang="ts"') || attrs.includes("lang='ts'");
    results.push({ content, isTypeScript });
  }

  return results;
}

/**
 * Transform TypeScript-specific syntax that esbuild might not handle in isolation
 */
function prepareForEsbuild(content, isTypeScript) {
  if (!isTypeScript) return content;

  // Remove Svelte-specific $: reactive statements as esbuild won't understand them
  // These are valid in Svelte but not in plain JS/TS
  let prepared = content.replace(/^\s*\$:\s+/gm, '// $: ');

  return prepared;
}

let tempFile = null;

try {
  const source = readFileSync(filepath, 'utf-8');
  const scripts = extractScriptContent(source);

  if (scripts.length === 0 || scripts.every(s => !s.content.trim())) {
    console.log(JSON.stringify({ valid: true }));
    process.exit(0);
  }

  // Combine all scripts (usually just one, but handle module scripts too)
  const hasTypeScript = scripts.some(s => s.isTypeScript);
  const combinedContent = scripts.map(s => prepareForEsbuild(s.content, s.isTypeScript)).join('\n');

  // Check if there are any imports
  if (!combinedContent.includes('import ') && !combinedContent.includes('export ')) {
    console.log(JSON.stringify({ valid: true }));
    process.exit(0);
  }

  // Create temp file for esbuild
  const ext = hasTypeScript ? '.ts' : '.js';
  tempFile = join(dirname(filepath), `.validate-${basename(filepath)}${ext}`);
  writeFileSync(tempFile, combinedContent);

  try {
    // Run esbuild with bundling to check all imports resolve
    await esbuild.build({
      entryPoints: [tempFile],
      bundle: true,
      write: false, // Don't write output, just check for errors
      platform: 'browser',
      format: 'esm',
      logLevel: 'silent',
      // Mark svelte internals and SvelteKit aliases as external
      // These won't resolve in isolation but are valid
      external: [
        'svelte',
        'svelte/*',
        '$lib/*',
        '$app/*',
        '$env/*',
        '$service-worker',
      ],
      // Resolve from project root
      absWorkingDir: projectRoot,
      // Handle path aliases if tsconfig exists
      tsconfigRaw: hasTypeScript ? {} : undefined,
    });

    // Clean up temp file
    unlinkSync(tempFile);
    tempFile = null;

    console.log(JSON.stringify({ valid: true }));
    process.exit(0);

  } catch (buildError) {
    // Clean up temp file
    if (tempFile && existsSync(tempFile)) {
      unlinkSync(tempFile);
      tempFile = null;
    }

    // Get allowed packages for typo suggestions
    const allowed = getAllowedPackages(projectRoot);

    // Parse esbuild errors with typo suggestions
    const errors = buildError.errors?.map(e => {
      const pkg = extractPackageFromError(e.text);
      const suggestions = pkg ? findSimilarPackages(pkg, allowed) : [];

      let message = e.text;
      if (suggestions.length > 0) {
        message += `. Did you mean: ${suggestions.map(s => `"${s}"`).join(', ')}?`;
      }

      return {
        message,
        file: e.location?.file,
        line: e.location?.line,
        column: e.location?.column,
        package: pkg,
        suggestions
      };
    }) || [{ message: buildError.message }];

    // Format for display
    const errorMessages = errors.map(e => e.message).join('; ');

    console.log(JSON.stringify({
      valid: false,
      error: `esbuild validation failed: ${errorMessages}`,
      errors,
      line: errors[0]?.line
    }));
    process.exit(1);
  }

} catch (error) {
  // Clean up temp file on any error
  if (tempFile && existsSync(tempFile)) {
    try {
      unlinkSync(tempFile);
    } catch (e) {
      // Ignore cleanup errors
    }
  }

  console.log(JSON.stringify({
    valid: false,
    error: `esbuild validation error: ${error.message}`
  }));
  process.exit(1);
}

/**
 * Extract package name from esbuild error message
 */
function extractPackageFromError(message) {
  // esbuild errors often look like:
  // Could not resolve "lucid"
  const match = message.match(/Could not resolve "([^"]+)"/);
  if (match) {
    const specifier = match[1];
    // Extract package name (handle scoped packages)
    if (specifier.startsWith('@')) {
      return specifier.split('/').slice(0, 2).join('/');
    }
    return specifier.split('/')[0];
  }
  return undefined;
}

/**
 * Load allowed packages from package.json dependencies
 */
function getAllowedPackages(root) {
  const pkgPath = join(root, 'package.json');
  if (!existsSync(pkgPath)) {
    return new Set(['svelte', 'lucide-svelte']);
  }

  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));
    const packages = new Set([
      ...Object.keys(pkg.dependencies || {}),
      ...Object.keys(pkg.devDependencies || {}),
      // Always allow svelte core packages
      'svelte',
    ]);
    return packages;
  } catch (e) {
    return new Set(['svelte', 'lucide-svelte']);
  }
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
 * Uses multiple strategies: prefix matching, substring matching, and Levenshtein distance
 */
function findSimilarPackages(pkgName, allowed, maxDistance = 3) {
  const suggestions = [];
  const pkgLower = pkgName.toLowerCase();

  for (const candidate of allowed) {
    const candidateLower = candidate.toLowerCase();

    // Strategy 1: Check if input is a prefix of a package name (e.g., "lucid" -> "lucide-svelte")
    if (candidateLower.startsWith(pkgLower) && candidateLower !== pkgLower) {
      suggestions.push({ name: candidate, distance: 0, priority: 1 });
      continue;
    }

    // Strategy 2: Check if input is contained in package name
    if (candidateLower.includes(pkgLower) && candidateLower !== pkgLower) {
      suggestions.push({ name: candidate, distance: 1, priority: 2 });
      continue;
    }

    // Strategy 3: Levenshtein distance for typos
    const distance = levenshteinDistance(pkgLower, candidateLower);
    if (distance <= maxDistance && distance > 0) {
      suggestions.push({ name: candidate, distance, priority: 3 });
    }
  }

  // Sort by priority first, then by distance
  return suggestions
    .sort((a, b) => a.priority - b.priority || a.distance - b.distance)
    .slice(0, 3)
    .map(s => s.name);
}
