#!/usr/bin/env node
/**
 * Critical anti-pattern gate for app/package code.
 *
 * Scans source files under src/ and packages/ for a focused set of
 * high-risk patterns that should always fail CI even while broader lint debt
 * is being paid down.
 */

import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const ROOT = process.cwd();
const SCAN_ROOTS = ['src', 'packages'];
const ALLOWED_EXTENSIONS = new Set(['.svelte', '.ts', '.js', '.mjs', '.cjs']);
const IGNORE_DIRS = new Set([
  'node_modules',
  'dist',
  'target',
  '.git',
  '.svelte-kit',
  '.turbo',
]);
const IGNORE_PATH_PREFIXES = ['src/generated/'];

const RULES = [
  {
    id: 'no-inner-outer-html',
    description: 'Disallow innerHTML/outerHTML assignment',
    regex: /\b(?:innerHTML|outerHTML)\s*=/g,
    fileFilter: () => true,
  },
  {
    id: 'no-insert-adjacent-html',
    description: 'Disallow insertAdjacentHTML',
    regex: /\.insertAdjacentHTML\s*\(/g,
    fileFilter: () => true,
  },
  {
    id: 'no-append-remove-child',
    description: 'Disallow appendChild/removeChild DOM mutation',
    regex: /\.(?:appendChild|removeChild)\s*\(/g,
    fileFilter: () => true,
  },
  {
    id: 'no-svelte-at-html',
    description: 'Disallow Svelte {@html} blocks',
    regex: /\{@html\b/g,
    fileFilter: (file) => file.endsWith('.svelte'),
  },
  {
    id: 'no-eval',
    description: 'Disallow eval',
    regex: /\beval\s*\(/g,
    fileFilter: () => true,
  },
  {
    id: 'no-new-function',
    description: 'Disallow Function constructor',
    regex: /\bnew\s+Function\s*\(/g,
    fileFilter: () => true,
  },
  {
    id: 'no-string-timer-callbacks',
    description: 'Disallow string-based setTimeout/setInterval callbacks',
    regex: /\b(?:setTimeout|setInterval)\s*\(\s*(['"`])/g,
    fileFilter: () => true,
  },
  {
    id: 'no-document-write',
    description: 'Disallow document.write',
    regex: /\bdocument\.write\s*\(/g,
    fileFilter: () => true,
  },
];

async function listFilesRecursively(dir, out = []) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (IGNORE_DIRS.has(entry.name)) continue;
    const abs = join(dir, entry.name);
    if (entry.isDirectory()) {
      await listFilesRecursively(abs, out);
      continue;
    }
    const ext = entry.name.slice(entry.name.lastIndexOf('.'));
    if (ALLOWED_EXTENSIONS.has(ext)) {
      out.push(abs);
    }
  }
  return out;
}

function isIgnoredRelativePath(relPath) {
  return IGNORE_PATH_PREFIXES.some((prefix) =>
    relPath === prefix.slice(0, -1) || relPath.startsWith(prefix)
  );
}

function toLineColumn(text, index) {
  const upToMatch = text.slice(0, index);
  const lines = upToMatch.split('\n');
  const line = lines.length;
  const column = lines[lines.length - 1].length + 1;
  return { line, column };
}

function collectMatches(text, relPath) {
  const violations = [];
  for (const rule of RULES) {
    if (!rule.fileFilter(relPath)) continue;
    rule.regex.lastIndex = 0;
    let match;
    while ((match = rule.regex.exec(text)) !== null) {
      const { line, column } = toLineColumn(text, match.index);
      violations.push({
        file: relPath,
        line,
        column,
        rule: rule.id,
        description: rule.description,
      });
    }
  }
  return violations;
}

async function main() {
  const allFiles = [];
  for (const scanRoot of SCAN_ROOTS) {
    const absRoot = join(ROOT, scanRoot);
    try {
      const files = await listFilesRecursively(absRoot);
      allFiles.push(...files);
    } catch {
      // Ignore missing scan roots.
    }
  }

  const violations = [];
  for (const file of allFiles) {
    const relPath = relative(ROOT, file).replaceAll('\\', '/');
    if (isIgnoredRelativePath(relPath)) continue;
    const content = await readFile(file, 'utf8');
    violations.push(...collectMatches(content, relPath));
  }

  if (violations.length === 0) {
    console.log('critical anti-pattern gate passed');
    process.exit(0);
  }

  console.error('critical anti-pattern gate failed');
  for (const v of violations) {
    console.error(`${v.file}:${v.line}:${v.column} [${v.rule}] ${v.description}`);
  }
  process.exit(1);
}

main().catch((error) => {
  console.error('critical anti-pattern gate failed unexpectedly');
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exit(1);
});
