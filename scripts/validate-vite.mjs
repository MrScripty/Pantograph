#!/usr/bin/env node
/**
 * Vite-based import validation for Svelte components
 *
 * Usage: node validate-vite.mjs <filepath> [projectRoot]
 *
 * Uses Vite's module resolution to check if imports can be resolved.
 * This is the most accurate validation method as it uses the actual bundler.
 *
 * Note: This creates a minimal Vite server instance which adds some overhead.
 *
 * Exits with code 0 if valid, code 1 if invalid.
 * Outputs JSON with validation results to stdout.
 */

import { createServer } from 'vite';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname, join } from 'path';
import { fileURLToPath } from 'url';

const filepath = process.argv[2];
const projectRoot = process.argv[3] || process.cwd();

if (!filepath) {
  console.log(JSON.stringify({ valid: false, error: 'No filepath provided' }));
  process.exit(1);
}

/**
 * Extract script content from Svelte file
 */
function extractScriptContent(source) {
  const scriptRegex = /<script[^>]*>([\s\S]*?)<\/script>/gi;
  const matches = [];
  let match;

  while ((match = scriptRegex.exec(source)) !== null) {
    matches.push(match[1]);
  }

  return matches.join('\n');
}

/**
 * Extract import specifiers from script content
 */
function extractImports(scriptContent) {
  const imports = [];

  // Match various import patterns
  const patterns = [
    // import X from 'Y'
    /import\s+(?:[\w*{}\s,]+)\s+from\s+['"]([^'"]+)['"]/g,
    // import 'Y' (side-effect imports)
    /import\s+['"]([^'"]+)['"]/g,
    // export ... from 'Y'
    /export\s+(?:[\w*{}\s,]+)\s+from\s+['"]([^'"]+)['"]/g,
  ];

  for (const pattern of patterns) {
    let match;
    while ((match = pattern.exec(scriptContent)) !== null) {
      imports.push(match[1]);
    }
  }

  // Deduplicate
  return [...new Set(imports)];
}

/**
 * Get line number for an import specifier
 */
function getLineNumber(source, specifier) {
  const lines = source.split('\n');
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].includes(`'${specifier}'`) || lines[i].includes(`"${specifier}"`)) {
      return i + 1;
    }
  }
  return undefined;
}

let server = null;

try {
  const source = readFileSync(filepath, 'utf-8');
  const scriptContent = extractScriptContent(source);

  if (!scriptContent.trim()) {
    console.log(JSON.stringify({ valid: true }));
    process.exit(0);
  }

  const importSpecifiers = extractImports(scriptContent);

  if (importSpecifiers.length === 0) {
    console.log(JSON.stringify({ valid: true }));
    process.exit(0);
  }

  // Check if Vite config exists
  const viteConfigPaths = [
    join(projectRoot, 'vite.config.ts'),
    join(projectRoot, 'vite.config.js'),
    join(projectRoot, 'vite.config.mjs'),
  ];

  const hasViteConfig = viteConfigPaths.some(p => existsSync(p));

  // Create a minimal Vite server for resolution
  server = await createServer({
    root: projectRoot,
    configFile: hasViteConfig ? undefined : false, // Use config if exists, otherwise disable
    server: {
      middlewareMode: true,
    },
    logLevel: 'silent',
    optimizeDeps: {
      disabled: true, // Don't run dependency optimization
    },
  });

  const errors = [];

  // Resolve each import
  for (const specifier of importSpecifiers) {
    // Skip SvelteKit aliases - they won't resolve outside of SvelteKit
    if (specifier.startsWith('$lib/') ||
        specifier.startsWith('$app/') ||
        specifier.startsWith('$env/') ||
        specifier.startsWith('$service-worker')) {
      continue;
    }

    // Skip relative imports - Vite handles these differently
    if (specifier.startsWith('./') || specifier.startsWith('../')) {
      // Check file existence directly
      const basePath = resolve(dirname(filepath), specifier);
      const extensions = ['', '.svelte', '.ts', '.js', '.mjs', '.json'];
      const found = extensions.some(ext => existsSync(basePath + ext));

      if (!found) {
        errors.push({
          specifier,
          message: `Cannot resolve relative import: "${specifier}"`,
          line: getLineNumber(scriptContent, specifier)
        });
      }
      continue;
    }

    try {
      // Use Vite's resolver
      const resolved = await server.pluginContainer.resolveId(specifier, filepath);

      if (!resolved) {
        errors.push({
          specifier,
          message: `Failed to resolve: "${specifier}"`,
          line: getLineNumber(scriptContent, specifier)
        });
      }
    } catch (e) {
      errors.push({
        specifier,
        message: `Resolution error for "${specifier}": ${e.message}`,
        line: getLineNumber(scriptContent, specifier)
      });
    }
  }

  await server.close();
  server = null;

  if (errors.length > 0) {
    const errorMessages = errors.map(e => e.message).join('; ');

    console.log(JSON.stringify({
      valid: false,
      error: `Vite import validation failed: ${errorMessages}`,
      errors,
      line: errors[0]?.line
    }));
    process.exit(1);
  }

  console.log(JSON.stringify({ valid: true }));
  process.exit(0);

} catch (error) {
  // Clean up server if it was created
  if (server) {
    try {
      await server.close();
    } catch (e) {
      // Ignore cleanup errors
    }
  }

  console.log(JSON.stringify({
    valid: false,
    error: `Vite validation error: ${error.message}`
  }));
  process.exit(1);
}
