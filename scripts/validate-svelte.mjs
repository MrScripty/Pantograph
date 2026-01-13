#!/usr/bin/env node
/**
 * Svelte component validation script
 *
 * Usage: node validate-svelte.mjs <filepath>
 *
 * Reads a Svelte file and attempts to compile it.
 * Exits with code 0 if valid, code 1 if invalid.
 * On error, outputs JSON with error details to stdout.
 */

import { compile } from 'svelte/compiler';
import { readFileSync } from 'fs';

const filepath = process.argv[2];

if (!filepath) {
  console.error(JSON.stringify({ error: 'No filepath provided' }));
  process.exit(1);
}

try {
  const source = readFileSync(filepath, 'utf-8');

  // Try to compile with Svelte 5 runes mode
  const result = compile(source, {
    filename: filepath,
    generate: 'client',
    runes: true,  // Enable Svelte 5 runes mode
    dev: true,    // Enable dev mode for better error messages
  });

  // Check for warnings that might indicate issues
  if (result.warnings && result.warnings.length > 0) {
    const criticalWarnings = result.warnings.filter(w =>
      w.code === 'options-deprecated-accessors' ||
      w.code === 'export-let-props'
    );

    if (criticalWarnings.length > 0) {
      console.log(JSON.stringify({
        valid: false,
        error: criticalWarnings[0].message,
        location: criticalWarnings[0].start
      }));
      process.exit(1);
    }
  }

  // Compilation succeeded
  console.log(JSON.stringify({ valid: true }));
  process.exit(0);

} catch (error) {
  // Compilation failed
  const output = {
    valid: false,
    error: error.message || String(error),
  };

  // Extract position info if available
  if (error.start) {
    output.line = error.start.line;
    output.column = error.start.column;
  }

  console.log(JSON.stringify(output));
  process.exit(1);
}
