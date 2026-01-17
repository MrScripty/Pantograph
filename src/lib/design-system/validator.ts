/**
 * Design System Validator
 *
 * Validates generated component source code for compliance with
 * the design system rules. Returns errors that can be sent back
 * to the agent for correction.
 */

import { designTokens } from './tokens';
import { availableIcons, type IconName } from './icons';

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// All allowed Tailwind color classes from our design system
const allowedColorClasses = new Set(Object.values(designTokens.colors));

// Additional commonly allowed Tailwind classes (non-color utilities)
const allowedUtilityPatterns = [
  // Spacing
  /^[pm][xytblr]?-\d+$/,
  /^gap-\d+$/,
  /^space-[xy]-\d+$/,

  // Sizing
  /^[wh]-(full|screen|auto|\d+|px)$/,
  /^(min|max)-[wh]-/,

  // Flexbox
  /^flex(-\w+)?$/,
  /^items-/,
  /^justify-/,
  /^self-/,
  /^grow/,
  /^shrink/,
  /^basis-/,

  // Grid
  /^grid(-\w+)?$/,
  /^col-/,
  /^row-/,

  // Position
  /^(absolute|relative|fixed|sticky)$/,
  /^(top|right|bottom|left|inset)-/,
  /^z-/,

  // Display
  /^(block|inline|inline-block|hidden|invisible|visible)$/,

  // Overflow
  /^overflow-/,

  // Text
  /^text-(left|center|right|justify)$/,
  /^text-(xs|sm|base|lg|xl|2xl|3xl|4xl)$/,
  /^font-(thin|light|normal|medium|semibold|bold|extrabold|black)$/,
  /^leading-/,
  /^tracking-/,
  /^truncate$/,
  /^whitespace-/,
  /^break-/,

  // Appearance
  /^cursor-/,
  /^pointer-events-/,
  /^select-/,
  /^opacity-/,

  // Effects
  /^shadow/,
  /^blur/,

  // Borders
  /^border(-[trbl])?(-\d+)?$/,
  /^rounded/,
  /^divide-/,
  /^ring/,
  /^outline/,

  // Transitions
  /^transition/,
  /^duration-/,
  /^ease-/,
  /^delay-/,
  /^animate-/,

  // Transforms
  /^transform/,
  /^scale-/,
  /^rotate-/,
  /^translate-/,
  /^skew-/,
  /^origin-/,

  // Interactivity
  /^hover:/,
  /^focus:/,
  /^active:/,
  /^disabled:/,
  /^group-/,

  // Placeholder
  /^placeholder-/,
];

// Patterns for disallowed arbitrary values
const arbitraryValuePattern = /\[.+\]/;

// Regex to find color classes that aren't in our allowed set
const tailwindColorPattern =
  /(?:^|\s)((?:bg|text|border|ring|outline|divide|placeholder)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}(?:\/\d+)?)/g;

// Emoji detection pattern (common emoji ranges)
const emojiPattern =
  /[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F600}-\u{1F64F}\u{1F680}-\u{1F6FF}]/gu;

/**
 * Validate a component source file against the design system
 */
export function validateComponent(source: string): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Check for arbitrary color values
  const arbitraryMatches = source.match(
    /(?:bg|text|border|ring)-\[[^\]]+\]/g
  );
  if (arbitraryMatches) {
    errors.push(
      `Arbitrary color values not allowed. Found: ${arbitraryMatches.join(', ')}. Use design system colors instead.`
    );
  }

  // Check for non-design-system Tailwind colors
  const colorMatches = [...source.matchAll(tailwindColorPattern)];
  const disallowedColors = colorMatches
    .map((m) => m[1])
    .filter((color) => !allowedColorClasses.has(color));

  if (disallowedColors.length > 0) {
    // Deduplicate
    const unique = [...new Set(disallowedColors)];
    errors.push(
      `Use design system colors instead of: ${unique.join(', ')}. See design system reference for allowed colors.`
    );
  }

  // Check for emoji
  const emojiMatches = source.match(emojiPattern);
  if (emojiMatches) {
    const unique = [...new Set(emojiMatches)];
    errors.push(
      `Emoji not allowed. Found: ${unique.join(' ')}. Use Lucide icons from 'lucide-svelte' instead.`
    );
  }

  // Validate icon imports
  const iconImportMatch = source.match(
    /import\s*\{([^}]+)\}\s*from\s*['"]lucide-svelte['"]/
  );
  if (iconImportMatch) {
    const importedIcons = iconImportMatch[1]
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

    const invalidIcons = importedIcons.filter(
      (icon) => !availableIcons.includes(icon as IconName)
    );

    if (invalidIcons.length > 0) {
      warnings.push(
        `Unknown Lucide icons (verify they exist): ${invalidIcons.join(', ')}`
      );
    }
  }

  // Check for inline SVG (should use Lucide instead)
  if (source.includes('<svg') && !source.includes('lucide-svelte')) {
    warnings.push(
      'Inline SVG detected. Consider using Lucide icons from lucide-svelte for consistency.'
    );
  }

  // Check for old Svelte syntax instead of runes
  if (source.includes('export let ') && !source.includes('$props')) {
    warnings.push(
      'Using legacy "export let" syntax. Use Svelte 5 runes: const { prop } = $props();'
    );
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Format validation errors for returning to the agent
 */
export function formatValidationErrors(result: ValidationResult): string {
  if (result.valid && result.warnings.length === 0) {
    return '';
  }

  const parts: string[] = [];

  if (result.errors.length > 0) {
    parts.push('## Errors (must fix):');
    result.errors.forEach((e, i) => parts.push(`${i + 1}. ${e}`));
  }

  if (result.warnings.length > 0) {
    parts.push('## Warnings:');
    result.warnings.forEach((w, i) => parts.push(`${i + 1}. ${w}`));
  }

  return parts.join('\n');
}

/**
 * Quick check if a string looks like it has design system violations
 * (faster than full validation, for quick filtering)
 */
export function hasLikelyViolations(source: string): boolean {
  // Quick checks for common violations
  if (arbitraryValuePattern.test(source)) return true;
  if (emojiPattern.test(source)) return true;
  if (tailwindColorPattern.test(source)) {
    // Need deeper check to see if they're allowed
    const matches = [...source.matchAll(tailwindColorPattern)];
    return matches.some((m) => !allowedColorClasses.has(m[1]));
  }
  return false;
}
