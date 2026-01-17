#!/usr/bin/env node
/**
 * JSX-in-template detection for Svelte components
 *
 * Usage: node validate-jsx-in-template.mjs <filepath>
 *
 * Detects React/JSX patterns in Svelte template expressions that would cause
 * cryptic "Unexpected token" errors from the Svelte compiler.
 *
 * Uses esbuild to parse expressions as JSX - if esbuild finds JSX elements,
 * we report a helpful error with fix instructions.
 *
 * Exits with code 0 if valid, code 1 if JSX patterns found.
 * Outputs JSON with validation results to stdout.
 */

import * as esbuild from 'esbuild';
import { readFileSync } from 'fs';

const filepath = process.argv[2];

if (!filepath) {
  console.log(JSON.stringify({ valid: false, error: 'No filepath provided' }));
  process.exit(1);
}

/**
 * Extract template content (everything outside <script> and <style> blocks)
 */
function extractTemplateContent(source) {
  // Remove script blocks
  let template = source.replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '');
  // Remove style blocks
  template = template.replace(/<style[^>]*>[\s\S]*?<\/style>/gi, '');
  return template;
}

/**
 * Extract all {expression} blocks from template using balanced brace matching.
 * Skips Svelte control blocks ({#if}, {/if}, {:else}, {@html}, etc.)
 */
function extractTemplateExpressions(template) {
  const expressions = [];
  let i = 0;

  while (i < template.length) {
    if (template[i] === '{') {
      // Check if this is a Svelte block (starts with #, /, :, @)
      const nextChar = template[i + 1];
      if (['#', '/', ':', '@'].includes(nextChar)) {
        // Skip to closing brace for Svelte blocks
        let depth = 1;
        i++;
        while (i < template.length && depth > 0) {
          if (template[i] === '{') depth++;
          else if (template[i] === '}') depth--;
          i++;
        }
        continue;
      }

      // Find matching closing brace with balanced counting
      let depth = 1;
      const start = i + 1;
      let j = start;

      while (j < template.length && depth > 0) {
        if (template[j] === '{') depth++;
        else if (template[j] === '}') depth--;
        j++;
      }

      if (depth === 0) {
        const expr = template.slice(start, j - 1).trim();
        if (expr) {
          expressions.push({
            expression: expr,
            start: i,
            end: j,
          });
        }
      }
      i = j;
    } else {
      i++;
    }
  }

  return expressions;
}

/**
 * Check if an expression contains JSX elements using esbuild
 */
async function checkForJSX(expression) {
  try {
    const result = await esbuild.transform(expression, {
      loader: 'jsx',
      jsx: 'transform',
      jsxFactory: '__SVELTE_JSX_DETECTED__',
      jsxFragment: '__SVELTE_JSX_FRAGMENT__',
    });
    return result.code.includes('__SVELTE_JSX_DETECTED__') ||
           result.code.includes('__SVELTE_JSX_FRAGMENT__');
  } catch (e) {
    // If esbuild can't parse it, it's not valid JSX
    return false;
  }
}

/**
 * Attempt to generate a fix suggestion for the JSX expression
 */
function generateFixSuggestion(expression) {
  // Detect pattern type and suggest fix

  // Pattern 1: condition && <element>
  const andMatch = expression.match(/^(.+?)\s*&&\s*(<.+)$/s);
  if (andMatch) {
    const [, condition, element] = andMatch;
    return `{#if ${condition.trim()}}${element.trim()}{/if}`;
  }

  // Pattern 2: condition ? <elementA> : <elementB> or : null
  const ternaryMatch = expression.match(/^(.+?)\s*\?\s*(<.+?)\s*:\s*(.+)$/s);
  if (ternaryMatch) {
    const [, condition, ifTrue, ifFalse] = ternaryMatch;
    const trimmedFalse = ifFalse.trim();
    if (trimmedFalse === 'null' || trimmedFalse === 'undefined' || trimmedFalse === '""' || trimmedFalse === "''") {
      return `{#if ${condition.trim()}}${ifTrue.trim()}{/if}`;
    }
    return `{#if ${condition.trim()}}${ifTrue.trim()}{:else}${trimmedFalse}{/if}`;
  }

  // Pattern 3: array.map(item => <element>)
  const mapMatch = expression.match(/^(.+?)\.map\s*\(\s*\(?([^)=]+)\)?\s*=>\s*(<.+)\)$/s);
  if (mapMatch) {
    const [, array, itemVar, element] = mapMatch;
    return `{#each ${array.trim()} as ${itemVar.trim()}}${element.trim()}{/each}`;
  }

  // Couldn't determine pattern - return generic suggestion
  return null;
}

/**
 * Build the error message with fix instructions
 */
function buildErrorMessage(expression, fixSuggestion) {
  let message = `JSX SYNTAX ERROR: Found JSX element in template expression.

Your code: {${expression}}

Svelte does not support JSX/React-style conditional rendering. Use Svelte's {#if} block syntax instead.

HOW TO FIX:

1. Conditional rendering (&&):
   ❌ {condition && <element>content</element>}
   ✅ {#if condition}<element>content</element>{/if}

2. Ternary expressions:
   ❌ {condition ? <A/> : <B/>}
   ✅ {#if condition}<A/>{:else}<B/>{/if}

3. Mapping arrays:
   ❌ {items.map(item => <Item {item}/>)}
   ✅ {#each items as item}<Item {item}/>{/each}`;

  if (fixSuggestion) {
    message += `

SUGGESTED FIX for your code:
${fixSuggestion}`;
  }

  return message;
}

// Main execution
try {
  const source = readFileSync(filepath, 'utf-8');
  const template = extractTemplateContent(source);
  const expressions = extractTemplateExpressions(template);

  // Check each expression for JSX
  for (const { expression } of expressions) {
    const hasJSX = await checkForJSX(expression);

    if (hasJSX) {
      const fixSuggestion = generateFixSuggestion(expression);
      const errorMessage = buildErrorMessage(expression, fixSuggestion);

      console.log(JSON.stringify({
        valid: false,
        error: errorMessage,
        expression: expression,
        suggestion: fixSuggestion,
      }));
      process.exit(1);
    }
  }

  // All expressions passed
  console.log(JSON.stringify({ valid: true }));
  process.exit(0);

} catch (error) {
  console.log(JSON.stringify({
    valid: false,
    error: `JSX validation error: ${error.message}`,
  }));
  process.exit(1);
}
