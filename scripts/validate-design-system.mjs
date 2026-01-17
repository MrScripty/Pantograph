#!/usr/bin/env node
/**
 * Design System validation for generated Svelte components
 *
 * Usage: node validate-design-system.mjs <filepath>
 *
 * Validates that generated components follow the design system:
 * - Only use approved Tailwind colors
 * - Use Lucide icons instead of emoji
 * - No arbitrary CSS values
 *
 * Returns warnings (not errors) to guide the agent without blocking.
 * Exits with code 0 always (advisory only).
 * Outputs JSON with validation results to stdout.
 */

import { readFileSync } from 'fs';

const filepath = process.argv[2];

if (!filepath) {
  console.log(JSON.stringify({ valid: true, warnings: ['No filepath provided'] }));
  process.exit(0);
}

// Design system allowed colors (from the design tokens)
const allowedColorClasses = new Set([
  // Backgrounds
  'bg-neutral-900',
  'bg-neutral-800',
  'bg-neutral-700',
  'bg-blue-600',
  'bg-blue-500',
  'bg-green-600',
  'bg-green-500',
  'bg-yellow-600',
  'bg-yellow-500',
  'bg-red-600',
  'bg-red-500',
  // Text
  'text-neutral-100',
  'text-neutral-400',
  'text-neutral-500',
  'text-blue-400',
  'text-green-400',
  'text-yellow-400',
  'text-red-400',
  // Borders
  'border-neutral-700',
  'border-neutral-800',
  'border-blue-500',
  'border-green-500',
  'border-red-500',
  // Common additional classes used in examples
  'text-white',
  'text-neutral-300',
  'bg-neutral-600',
  'bg-transparent',
  'border-neutral-600',
  // Hover variants of allowed colors
  'hover:bg-neutral-700',
  'hover:bg-neutral-600',
  'hover:bg-blue-700',
  'hover:bg-blue-500',
  'hover:bg-green-700',
  'hover:bg-green-500',
  'hover:text-neutral-100',
  'hover:text-neutral-300',
  'hover:border-blue-500',
  // Focus variants
  'focus:border-blue-500',
  'focus-within:border-blue-500',
]);

// Pattern to find Tailwind color classes
const colorPattern = /(?:^|\s)((?:bg|text|border|ring|outline|divide|placeholder)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}(?:\/\d+)?)/g;

// Pattern to find arbitrary values
const arbitraryPattern = /(?:bg|text|border|ring)-\[[^\]]+\]/g;

// Emoji pattern (common emoji ranges)
const emojiPattern = /[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F600}-\u{1F64F}\u{1F680}-\u{1F6FF}]/gu;

// Available Lucide icons
const availableIcons = new Set([
  'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown',
  'ChevronLeft', 'ChevronRight', 'ChevronUp', 'ChevronDown', 'ChevronsUpDown',
  'Menu', 'MoreHorizontal', 'MoreVertical', 'ExternalLink',
  'Plus', 'Minus', 'X', 'Check', 'Search', 'Filter', 'SortAsc', 'SortDesc',
  'Refresh', 'RotateCcw', 'RotateCw',
  'Edit', 'Pencil', 'Trash', 'Trash2', 'Copy', 'Clipboard', 'Save',
  'Download', 'Upload',
  'AlertCircle', 'AlertTriangle', 'Info', 'HelpCircle', 'CheckCircle', 'XCircle',
  'Loader', 'Loader2',
  'Eye', 'EyeOff', 'Lock', 'Unlock',
  'User', 'Users', 'UserPlus', 'UserMinus', 'LogIn', 'LogOut', 'Settings', 'Cog',
  'Mail', 'Send', 'MessageSquare', 'MessageCircle', 'Bell', 'BellOff',
  'Play', 'Pause', 'Stop', 'SkipForward', 'SkipBack', 'FastForward', 'Rewind',
  'Volume', 'Volume1', 'Volume2', 'VolumeX', 'Mic', 'MicOff',
  'File', 'FileText', 'Folder', 'FolderOpen', 'FolderPlus', 'Image', 'Film', 'Music',
  'Grid', 'List', 'Columns', 'Rows', 'Maximize', 'Minimize', 'Expand', 'Shrink',
  'Calendar', 'Clock', 'Timer', 'Hourglass',
  'Home', 'Bookmark', 'Heart', 'Star', 'ThumbsUp', 'ThumbsDown',
  'Flag', 'Tag', 'Hash', 'Link', 'Unlink', 'Paperclip', 'Pin', 'PinOff',
  'Sun', 'Moon', 'Monitor',
  'ToggleLeft', 'ToggleRight', 'Circle', 'CircleDot', 'Square', 'CheckSquare',
  'Zap', 'Terminal', 'Code', 'Braces', 'Database', 'Server',
  'Wifi', 'WifiOff', 'Bluetooth', 'Battery', 'Power', 'Palette',
]);

try {
  const content = readFileSync(filepath, 'utf-8');
  const warnings = [];

  // Check for arbitrary values
  const arbitraryMatches = content.match(arbitraryPattern);
  if (arbitraryMatches) {
    warnings.push({
      type: 'arbitrary_color',
      message: `Arbitrary color values found: ${[...new Set(arbitraryMatches)].join(', ')}. Use design system colors instead.`,
      matches: [...new Set(arbitraryMatches)],
    });
  }

  // Check for non-design-system colors
  const colorMatches = [...content.matchAll(colorPattern)];
  const disallowedColors = colorMatches
    .map(m => m[1])
    .filter(color => {
      // Allow if it's in our allowed set
      if (allowedColorClasses.has(color)) return false;
      // Allow hover/focus variants of allowed colors
      const baseColor = color.replace(/^(hover:|focus:|active:|group-hover:)/, '');
      if (allowedColorClasses.has(baseColor)) return false;
      return true;
    });

  if (disallowedColors.length > 0) {
    const unique = [...new Set(disallowedColors)];
    warnings.push({
      type: 'non_design_system_color',
      message: `Non-design-system colors found: ${unique.join(', ')}. Consider using design system colors.`,
      matches: unique,
    });
  }

  // Check for emoji
  const emojiMatches = content.match(emojiPattern);
  if (emojiMatches) {
    warnings.push({
      type: 'emoji',
      message: `Emoji found: ${[...new Set(emojiMatches)].join(' ')}. Use Lucide icons instead.`,
      matches: [...new Set(emojiMatches)],
    });
  }

  // Check Lucide icon imports
  const iconImportMatch = content.match(/import\s*\{([^}]+)\}\s*from\s*['"]lucide-svelte['"]/);
  if (iconImportMatch) {
    const importedIcons = iconImportMatch[1]
      .split(',')
      .map(s => s.trim())
      .filter(s => s.length > 0);

    const unknownIcons = importedIcons.filter(icon => !availableIcons.has(icon));
    if (unknownIcons.length > 0) {
      warnings.push({
        type: 'unknown_icon',
        message: `Unknown Lucide icons: ${unknownIcons.join(', ')}. Verify these exist in lucide-svelte.`,
        matches: unknownIcons,
      });
    }
  }

  // Check for inline SVG (should use Lucide)
  if (content.includes('<svg') && !content.includes('lucide-svelte')) {
    warnings.push({
      type: 'inline_svg',
      message: 'Inline SVG found without Lucide import. Consider using Lucide icons for consistency.',
    });
  }

  // Output result
  if (warnings.length > 0) {
    console.log(JSON.stringify({
      valid: true, // Warnings don't block - they're advisory
      warnings: warnings.map(w => w.message),
      details: warnings,
    }));
  } else {
    console.log(JSON.stringify({ valid: true, warnings: [] }));
  }

  process.exit(0);

} catch (error) {
  // If validation fails, don't block - just warn
  console.log(JSON.stringify({
    valid: true,
    warnings: [`Design system validation error: ${error.message}`],
  }));
  process.exit(0);
}
