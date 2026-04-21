#!/usr/bin/env node
import { readdir, readFile } from 'node:fs/promises';
import { extname, join, relative } from 'node:path';

const ROOT = process.cwd();
const SCAN_ROOTS = ['src', 'packages/svelte-graph/src'];
const IGNORE_DIRS = new Set(['node_modules', 'dist', 'target', '.git', '.svelte-kit']);
const INTERACTIVE_SEMANTIC_TAGS = new Set(['button', 'a', 'input', 'select', 'textarea']);

async function listSvelteFiles(dir, out = []) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (IGNORE_DIRS.has(entry.name)) continue;
    const abs = join(dir, entry.name);
    if (entry.isDirectory()) {
      await listSvelteFiles(abs, out);
    } else if (extname(entry.name) === '.svelte') {
      out.push(abs);
    }
  }
  return out;
}

function lineForIndex(text, index) {
  return text.slice(0, index).split('\n').length;
}

function startTag(element) {
  return element.match(/^<[\w:-]+\b[\s\S]*?>/)?.[0] ?? '';
}

function stripMarkup(text) {
  return text
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<script\b[\s\S]*?<\/script>/g, '')
    .replace(/<style\b[\s\S]*?<\/style>/g, '')
    .replace(/<[\s\S]*?>/g, '')
    .replace(/\{[#/:@][\s\S]*?\}/g, '')
    .trim();
}

function hasAccessibleName(element) {
  const open = startTag(element);
  if (/\saria-label\s*=/.test(open) || /\saria-labelledby\s*=/.test(open)) {
    return true;
  }
  return /[A-Za-z0-9{]/.test(stripMarkup(element));
}

function collectButtonViolations(text, relPath) {
  const violations = [];
  const buttonRegex = /<button\b[\s\S]*?<\/button>/g;
  let match;
  while ((match = buttonRegex.exec(text)) !== null) {
    const element = match[0];
    if (!hasAccessibleName(element)) {
      violations.push({
        file: relPath,
        line: lineForIndex(text, match.index),
        rule: 'button-accessible-name',
        message: 'Icon-only or textless <button> must include aria-label or aria-labelledby.',
      });
    }
  }
  return violations;
}

function collectRoleButtonViolations(text, relPath) {
  const violations = [];
  const roleRegex = /<([\w:-]+)\b(?=[^>]*\srole\s*=\s*["']button["'])[^>]*>/g;
  let match;
  while ((match = roleRegex.exec(text)) !== null) {
    const tag = match[1];
    if (INTERACTIVE_SEMANTIC_TAGS.has(tag)) continue;

    const open = match[0];
    const line = lineForIndex(text, match.index);
    if (!/\stabindex\s*=/.test(open)) {
      violations.push({
        file: relPath,
        line,
        rule: 'role-button-tabindex',
        message: 'Generic role="button" element must declare tabindex.',
      });
    }
    if (!/\son:keydown\s*=|\sonkeydown\s*=/.test(open)) {
      violations.push({
        file: relPath,
        line,
        rule: 'role-button-keydown',
        message: 'Generic role="button" element must handle keyboard activation.',
      });
    }
    if (!hasAccessibleName(open)) {
      violations.push({
        file: relPath,
        line,
        rule: 'role-button-accessible-name',
        message: 'Generic role="button" element must include an accessible name.',
      });
    }
  }
  return violations;
}

function collectIgnoreViolations(text, relPath) {
  const violations = [];
  const lines = text.split('\n');
  for (let index = 0; index < lines.length; index += 1) {
    if (!lines[index].includes('svelte-ignore a11y_')) continue;
    const previous = index > 0 ? lines[index - 1] : '';
    if (!previous.includes('a11y-reviewed:')) {
      violations.push({
        file: relPath,
        line: index + 1,
        rule: 'reviewed-a11y-ignore',
        message: 'Svelte a11y ignore comments require an adjacent a11y-reviewed reason.',
      });
    }
  }
  return violations;
}

async function main() {
  const files = [];
  for (const root of SCAN_ROOTS) {
    files.push(...(await listSvelteFiles(join(ROOT, root))));
  }

  const violations = [];
  for (const file of files) {
    const text = await readFile(file, 'utf8');
    const relPath = relative(ROOT, file).replaceAll('\\', '/');
    violations.push(
      ...collectButtonViolations(text, relPath),
      ...collectRoleButtonViolations(text, relPath),
      ...collectIgnoreViolations(text, relPath)
    );
  }

  if (violations.length === 0) {
    console.log('svelte accessibility gate passed');
    return;
  }

  console.error('svelte accessibility gate failed');
  for (const violation of violations) {
    console.error(
      `${violation.file}:${violation.line} [${violation.rule}] ${violation.message}`
    );
  }
  process.exit(1);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exit(1);
});
