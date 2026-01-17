/**
 * Agent Context Builder
 *
 * Generates the design system documentation to inject into agent prompts.
 * This ensures the agent uses only approved colors, icons, and patterns.
 */

import { designTokens } from './tokens';
import { getIconUsageExamples } from './icons';

/**
 * Build the complete design system context for the agent
 */
export function buildDesignSystemContext(): string {
  return `# Design System Reference

You MUST use only the colors, spacing, and patterns defined here.
Do NOT use arbitrary values, custom colors, or emoji for icons.

## Colors

### Backgrounds
${formatColorSection(designTokens.colors, 'bg-')}

### Text Colors
${formatColorSection(designTokens.colors, 'text-')}

### Border Colors
${formatColorSection(designTokens.colors, 'border-')}

## Spacing

Use these values for padding, margin, and gap:
| Token | Value | Tailwind Example |
|-------|-------|------------------|
${Object.entries(designTokens.spacing)
  .map(([name, value]) => `| ${name} | ${value} | \`p-${getTailwindSpacing(name)}\`, \`m-${getTailwindSpacing(name)}\`, \`gap-${getTailwindSpacing(name)}\` |`)
  .join('\n')}

## Border Radius
${Object.entries(designTokens.borderRadius)
  .map(([name, value]) => `- **${name}**: \`${value}\``)
  .join('\n')}

## Typography
${Object.entries(designTokens.typography)
  .map(([name, value]) => `- **${name}**: \`${value}\``)
  .join('\n')}

## Shadows
${Object.entries(designTokens.shadows)
  .map(([name, value]) => `- **${name}**: \`${value}\``)
  .join('\n')}

## Transitions
${Object.entries(designTokens.transitions)
  .map(([name, value]) => `- **${name}**: \`${value}\``)
  .join('\n')}

${getIconUsageExamples()}

## Common Patterns

### Buttons
\`\`\`svelte
<!-- Primary button -->
<button class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-neutral-100 rounded-lg transition-all duration-150">
  Click me
</button>

<!-- Secondary button -->
<button class="px-4 py-2 bg-neutral-700 hover:bg-neutral-600 text-neutral-100 rounded-lg border border-neutral-600 transition-all duration-150">
  Cancel
</button>

<!-- Icon button -->
<button class="p-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-400 hover:text-neutral-100 rounded-lg transition-all duration-150">
  <Settings class="w-5 h-5" />
</button>
\`\`\`

### Input Fields
\`\`\`svelte
<input
  type="text"
  class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 rounded-lg text-neutral-100 placeholder-neutral-500 focus:border-blue-500 focus:outline-none transition-all duration-150"
  placeholder="Enter text..."
/>
\`\`\`

### Cards
\`\`\`svelte
<div class="p-4 bg-neutral-800 border border-neutral-700 rounded-xl">
  <h3 class="text-lg font-medium text-neutral-100">Card Title</h3>
  <p class="text-sm text-neutral-400">Card content goes here.</p>
</div>
\`\`\`

### Badges
\`\`\`svelte
<!-- Success badge -->
<span class="px-2 py-1 text-xs bg-green-600 text-neutral-100 rounded-full">Active</span>

<!-- Warning badge -->
<span class="px-2 py-1 text-xs bg-yellow-600 text-neutral-100 rounded-full">Pending</span>

<!-- Error badge -->
<span class="px-2 py-1 text-xs bg-red-600 text-neutral-100 rounded-full">Error</span>
\`\`\`

## STRICT RULES

1. **Colors**: Use ONLY the colors listed above. Never use:
   - Arbitrary values like \`bg-[#123456]\` or \`text-[rgb(...)]\`
   - Unlisted Tailwind colors like \`bg-purple-500\` or \`text-pink-400\`

2. **Icons**: Use ONLY Lucide icons from the list above. Never use:
   - Emoji (no unicode emoji characters)
   - Inline SVG (import from lucide-svelte instead)
   - Made-up icon names

3. **Styling**: Follow the established patterns for consistency.

4. **Svelte 5**: Use the runes syntax:
   - \`$props()\` for component props
   - \`$state()\` for reactive state
   - \`$derived()\` for computed values
   - \`$effect()\` for side effects`;
}

/**
 * Format a section of color tokens
 */
function formatColorSection(
  colors: typeof designTokens.colors,
  prefix: string
): string {
  return Object.entries(colors)
    .filter(([key]) => key.startsWith(prefix))
    .map(([name, value]) => `- **${name}**: \`${value}\``)
    .join('\n');
}

/**
 * Map spacing token names to Tailwind spacing values
 */
function getTailwindSpacing(name: string): string {
  const map: Record<string, string> = {
    xs: '1',
    sm: '2',
    md: '4',
    lg: '6',
    xl: '8',
    '2xl': '12',
  };
  return map[name] || '4';
}

/**
 * Get a compact version of the design system for token-limited contexts
 */
export function getCompactDesignContext(): string {
  return `# Design System (Compact)

## Colors (use these Tailwind classes):
- Backgrounds: bg-neutral-900 (primary), bg-neutral-800 (secondary), bg-neutral-700 (tertiary), bg-blue-600 (accent), bg-green-600 (success), bg-red-600 (error)
- Text: text-neutral-100 (primary), text-neutral-400 (secondary), text-neutral-500 (muted), text-blue-400/green-400/red-400 (status)
- Borders: border-neutral-700 (default), border-blue-500 (focus)

## Icons: Import from 'lucide-svelte'. Use w-4/w-5/w-6 h-4/h-5/h-6 for sizing.

## Svelte 5: Use $props(), $state(), $derived(), $effect() runes.

## Rules: No arbitrary colors, no emoji, no inline SVG.`;
}
