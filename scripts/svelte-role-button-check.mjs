import { parse } from 'svelte/compiler';

const INTERACTIVE_SEMANTIC_TAGS = new Set(['button', 'a', 'input', 'select', 'textarea']);

// A rendered text expression is name evidence, as in the existing button check.
// Attribute expressions (including event handlers) are never name evidence.
function hasRenderedName(node) {
  if (!node || typeof node !== 'object') return false;
  if (node.type === 'Text') return /[A-Za-z0-9]/.test(node.data);
  if (node.type === 'ExpressionTag') return true;
  if (node.type === 'Comment') return false;
  if (node.type === 'RegularElement' && ['script', 'style'].includes(node.name)) return false;
  return ['nodes', 'fragment', 'consequent', 'alternate', 'body', 'fallback'].some((key) => {
    const value = node[key];
    return Array.isArray(value) ? value.some(hasRenderedName) : hasRenderedName(value);
  });
}

/** Inspect parsed attributes so JavaScript arrows and quoted > cannot end a tag. */
export function collectRoleButtonViolations(text, relPath) {
  const ast = parse(text, { modern: true, filename: relPath });
  const violations = [];

  function visit(node) {
    if (!node || typeof node !== 'object') return;
    if (node.type === 'RegularElement' && !INTERACTIVE_SEMANTIC_TAGS.has(node.name)) {
      const attributes = node.attributes;
      const role = attributes.find((attribute) => attribute.type === 'Attribute' && attribute.name === 'role');
      const isButton = Array.isArray(role?.value)
        && role.value.length === 1
        && role.value[0].type === 'Text'
        && role.value[0].data === 'button';
      if (isButton) {
        const hasAttribute = (name) => attributes.some((attribute) => attribute.type === 'Attribute' && attribute.name === name);
        const hasKeydown = hasAttribute('onkeydown') || attributes.some((attribute) => attribute.type === 'OnDirective' && attribute.name === 'keydown');
        const line = text.slice(0, node.start).split('\n').length;
        const add = (rule, message) => violations.push({ file: relPath, line, rule, message });
        if (!hasAttribute('tabindex')) {
          add('role-button-tabindex', 'Generic role="button" element must declare tabindex.');
        }
        if (!hasKeydown) {
          add('role-button-keydown', 'Generic role="button" element must handle keyboard activation.');
        }
        if (!hasAttribute('aria-label') && !hasAttribute('aria-labelledby') && !hasRenderedName(node.fragment)) {
          add('role-button-accessible-name', 'Generic role="button" element must include an accessible name.');
        }
      }
    }
    for (const value of Object.values(node)) {
      if (Array.isArray(value)) value.forEach(visit);
      else if (value && typeof value === 'object') visit(value);
    }
  }

  visit(ast.fragment);
  return violations;
}
