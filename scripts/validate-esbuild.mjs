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

    // Parse esbuild errors
    const errors = buildError.errors?.map(e => ({
      message: e.text,
      file: e.location?.file,
      line: e.location?.line,
      column: e.location?.column,
      // Extract package name from error message if possible
      package: extractPackageFromError(e.text)
    })) || [{ message: buildError.message }];

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
