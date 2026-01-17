/**
 * ESLint rule to detect string interpolation inside SVG elements.
 *
 * This catches a common mistake where SVG path data is inserted as text content
 * instead of being used in the correct attribute (e.g., <path d={...}>).
 *
 * BAD:  <svg>{pathData}</svg>          - renders as invisible text
 * GOOD: <path d={pathData}></path>     - correct attribute usage
 * GOOD: <text>{label}</text>           - text elements allow text content
 * GOOD: {@html svgContent}             - explicit raw HTML
 */

// SVG elements that are designed to contain text content
const SVG_TEXT_CONTENT_ELEMENTS = new Set([
  'text',
  'textpath',
  'tspan',
  'desc',
  'title',
]);

// All SVG elements (from eslint-plugin-svelte's element-types.js)
const SVG_ELEMENTS = new Set([
  'altglyph',
  'altglyphdef',
  'altglyphitem',
  'animate',
  'animatecolor',
  'animatemotion',
  'animatetransform',
  'circle',
  'clippath',
  'color-profile',
  'cursor',
  'defs',
  'desc',
  'discard',
  'ellipse',
  'feblend',
  'fecolormatrix',
  'fecomponenttransfer',
  'fecomposite',
  'feconvolvematrix',
  'fediffuselighting',
  'fedisplacementmap',
  'fedistantlight',
  'fedropshadow',
  'feflood',
  'fefunca',
  'fefuncb',
  'fefuncg',
  'fefuncr',
  'fegaussianblur',
  'feimage',
  'femerge',
  'femergenode',
  'femorphology',
  'feoffset',
  'fepointlight',
  'fespecularlighting',
  'fespotlight',
  'fetile',
  'feturbulence',
  'filter',
  'font',
  'font-face',
  'font-face-format',
  'font-face-name',
  'font-face-src',
  'font-face-uri',
  'foreignobject',
  'g',
  'glyph',
  'glyphref',
  'hatch',
  'hatchpath',
  'hkern',
  'image',
  'line',
  'lineargradient',
  'marker',
  'mask',
  'mesh',
  'meshgradient',
  'meshpatch',
  'meshrow',
  'metadata',
  'missing-glyph',
  'mpath',
  'path',
  'pattern',
  'polygon',
  'polyline',
  'radialgradient',
  'rect',
  'set',
  'solidcolor',
  'stop',
  'svg',
  'switch',
  'symbol',
  'text',
  'textpath',
  'tref',
  'tspan',
  'unknown',
  'use',
  'view',
  'vkern',
]);

/**
 * Get the element name from a SvelteElement node.
 * Handles both simple names and member expressions.
 */
function getElementName(node) {
  if (!node.name) return null;

  if (node.name.type === 'Identifier' || node.name.type === 'SvelteName') {
    return node.name.name;
  }

  // For svelte:element or other special cases
  if (node.name.type === 'SvelteMemberExpressionName') {
    return null; // Can't statically determine
  }

  return null;
}

/**
 * Check if an element name is an SVG element.
 */
function isSvgElement(name) {
  return SVG_ELEMENTS.has(name.toLowerCase());
}

/**
 * Check if an element allows text content (like <text>, <tspan>, etc.)
 */
function allowsTextContent(name) {
  return SVG_TEXT_CONTENT_ELEMENTS.has(name.toLowerCase());
}

/**
 * Check if a node is inside an attribute (not as element content).
 * Returns true if the node is part of an attribute value.
 */
function isInsideAttribute(node) {
  let parent = node.parent;
  while (parent) {
    // If we hit an attribute node before hitting a SvelteElement's children, it's in an attribute
    if (
      parent.type === 'SvelteAttribute' ||
      parent.type === 'SvelteDirective' ||
      parent.type === 'SvelteStyleDirective' ||
      parent.type === 'SvelteShorthandAttribute' ||
      parent.type === 'SvelteSpreadAttribute' ||
      parent.type === 'SvelteStartTag'
    ) {
      return true;
    }
    // If we hit a SvelteElement, we're in its children (not an attribute)
    if (parent.type === 'SvelteElement') {
      return false;
    }
    parent = parent.parent;
  }
  return false;
}

/**
 * Find the immediate parent SvelteElement that contains this node as a child.
 * Only returns an SVG element if the node is in the element's children (not attributes).
 */
function findParentSvgElement(node) {
  let parent = node.parent;
  while (parent) {
    if (parent.type === 'SvelteElement') {
      const name = getElementName(parent);
      if (name && isSvgElement(name)) {
        return { element: parent, name };
      }
    }
    parent = parent.parent;
  }
  return null;
}

export default {
  meta: {
    type: 'problem',
    docs: {
      description: 'Disallow string interpolation as direct children of non-text SVG elements',
      category: 'Possible Errors',
      recommended: true,
    },
    messages: {
      invalidSvgTextContent:
        'String interpolation inside <{{element}}> will render as invisible text. ' +
        'Did you mean to use an attribute like <path d={...}> or {@html ...} for raw SVG markup?',
    },
    schema: [],
  },

  create(context) {
    return {
      // Match mustache tags (interpolation) with kind="text" (not kind="raw" which is {@html})
      'SvelteMustacheTag'(node) {
        // Skip {@html ...} tags - those are intentional raw HTML
        if (node.kind === 'raw') {
          return;
        }

        // Skip if this is inside an attribute value (e.g., d={pathData}, width={size})
        if (isInsideAttribute(node)) {
          return;
        }

        // Find the nearest parent SVG element
        const parentInfo = findParentSvgElement(node);
        if (!parentInfo) {
          return; // Not inside an SVG element
        }

        const { name: elementName } = parentInfo;

        // Allow text content in text-related SVG elements
        if (allowsTextContent(elementName)) {
          return;
        }

        // Report the error
        context.report({
          node,
          messageId: 'invalidSvgTextContent',
          data: { element: elementName },
        });
      },
    };
  },
};
