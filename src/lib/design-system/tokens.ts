/**
 * Design System Tokens
 *
 * These tokens define the visual language for generated components.
 * The agent MUST use only these values when generating UI.
 */

export const designTokens = {
  colors: {
    // Backgrounds
    'bg-primary': 'bg-neutral-900',
    'bg-secondary': 'bg-neutral-800',
    'bg-tertiary': 'bg-neutral-700',
    'bg-accent': 'bg-blue-600',
    'bg-accent-hover': 'bg-blue-500',
    'bg-success': 'bg-green-600',
    'bg-success-hover': 'bg-green-500',
    'bg-warning': 'bg-yellow-600',
    'bg-warning-hover': 'bg-yellow-500',
    'bg-error': 'bg-red-600',
    'bg-error-hover': 'bg-red-500',

    // Text
    'text-primary': 'text-neutral-100',
    'text-secondary': 'text-neutral-400',
    'text-muted': 'text-neutral-500',
    'text-accent': 'text-blue-400',
    'text-success': 'text-green-400',
    'text-warning': 'text-yellow-400',
    'text-error': 'text-red-400',

    // Borders
    'border-default': 'border-neutral-700',
    'border-subtle': 'border-neutral-800',
    'border-focus': 'border-blue-500',
    'border-success': 'border-green-500',
    'border-error': 'border-red-500',
  },

  spacing: {
    xs: '0.25rem', // 4px
    sm: '0.5rem', // 8px
    md: '1rem', // 16px
    lg: '1.5rem', // 24px
    xl: '2rem', // 32px
    '2xl': '3rem', // 48px
  },

  borderRadius: {
    none: 'rounded-none',
    sm: 'rounded',
    md: 'rounded-lg',
    lg: 'rounded-xl',
    full: 'rounded-full',
  },

  typography: {
    'heading-lg': 'text-2xl font-bold',
    'heading-md': 'text-xl font-semibold',
    'heading-sm': 'text-lg font-medium',
    body: 'text-base',
    'body-sm': 'text-sm',
    caption: 'text-xs text-neutral-400',
  },

  shadows: {
    none: 'shadow-none',
    sm: 'shadow-sm',
    md: 'shadow-md',
    lg: 'shadow-lg',
  },

  transitions: {
    fast: 'transition-all duration-150',
    normal: 'transition-all duration-200',
    slow: 'transition-all duration-300',
  },
} as const;

export type DesignTokens = typeof designTokens;

// Utility to get all allowed Tailwind classes from tokens
export function getAllowedClasses(): string[] {
  const classes: string[] = [];

  // Colors
  Object.values(designTokens.colors).forEach((c) => classes.push(c));

  // Border radius
  Object.values(designTokens.borderRadius).forEach((c) => classes.push(c));

  // Typography
  Object.values(designTokens.typography).forEach((c) => {
    c.split(' ').forEach((cls) => classes.push(cls));
  });

  // Shadows
  Object.values(designTokens.shadows).forEach((c) => classes.push(c));

  // Transitions
  Object.values(designTokens.transitions).forEach((c) => {
    c.split(' ').forEach((cls) => classes.push(cls));
  });

  return [...new Set(classes)];
}
