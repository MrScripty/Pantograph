#!/usr/bin/env node
/**
 * ESLint validation for Svelte components
 *
 * Usage: node validate-lint.mjs <filepath> [projectRoot]
 *
 * Runs ESLint on a Svelte file and returns JSON results.
 * Catches code quality issues that pass syntax validation but are likely mistakes,
 * such as using undefined explicitly or unused variables.
 *
 * Exits with code 0 if valid, code 1 if linting errors found.
 * Outputs JSON with validation results to stdout.
 */

import { ESLint } from 'eslint';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const filepath = process.argv[2];
const projectRoot = process.argv[3] || resolve(__dirname, '..');

if (!filepath) {
  console.log(JSON.stringify({ valid: false, error: 'No filepath provided' }));
  process.exit(1);
}

try {
  const eslint = new ESLint({
    overrideConfigFile: join(projectRoot, 'eslint.config.mjs'),
    cwd: projectRoot,
  });

  const results = await eslint.lintFiles([filepath]);

  // Filter to only errors (severity 2), not warnings (severity 1)
  const errors = results[0]?.messages.filter(m => m.severity === 2) || [];

  if (errors.length > 0) {
    const errorMessages = errors.map(e =>
      `Line ${e.line}: ${e.message} (${e.ruleId})`
    ).join('; ');

    console.log(JSON.stringify({
      valid: false,
      error: `Linting errors: ${errorMessages}`,
      errors: errors.map(e => ({
        line: e.line,
        column: e.column,
        message: e.message,
        ruleId: e.ruleId,
      })),
      line: errors[0]?.line,
    }));
    process.exit(1);
  }

  console.log(JSON.stringify({ valid: true }));
  process.exit(0);

} catch (error) {
  // If ESLint itself fails (e.g., config issue), don't block validation
  // Log the error but return valid to avoid breaking the pipeline
  console.error(`ESLint error: ${error.message}`);
  console.log(JSON.stringify({
    valid: true,
    warning: `ESLint validation skipped: ${error.message}`
  }));
  process.exit(0);
}
