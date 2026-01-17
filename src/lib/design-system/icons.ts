/**
 * Icon System - Lucide icons available for generated components
 *
 * This module defines which icons are available and provides
 * usage examples for the agent context.
 */

// Curated list of commonly needed UI icons
export const availableIcons = [
  // Navigation & Actions
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'ArrowDown',
  'ChevronLeft',
  'ChevronRight',
  'ChevronUp',
  'ChevronDown',
  'ChevronsUpDown',
  'Menu',
  'MoreHorizontal',
  'MoreVertical',
  'ExternalLink',

  // Common Actions
  'Plus',
  'Minus',
  'X',
  'Check',
  'Search',
  'Filter',
  'SortAsc',
  'SortDesc',
  'Refresh',
  'RotateCcw',
  'RotateCw',

  // CRUD Operations
  'Edit',
  'Pencil',
  'Trash',
  'Trash2',
  'Copy',
  'Clipboard',
  'Save',
  'Download',
  'Upload',

  // State & Feedback
  'AlertCircle',
  'AlertTriangle',
  'Info',
  'HelpCircle',
  'CheckCircle',
  'XCircle',
  'Loader',
  'Loader2',

  // Visibility
  'Eye',
  'EyeOff',
  'Lock',
  'Unlock',

  // User & Account
  'User',
  'Users',
  'UserPlus',
  'UserMinus',
  'LogIn',
  'LogOut',
  'Settings',
  'Cog',

  // Communication
  'Mail',
  'Send',
  'MessageSquare',
  'MessageCircle',
  'Bell',
  'BellOff',

  // Media Controls
  'Play',
  'Pause',
  'Stop',
  'SkipForward',
  'SkipBack',
  'FastForward',
  'Rewind',
  'Volume',
  'Volume1',
  'Volume2',
  'VolumeX',
  'Mic',
  'MicOff',

  // Files & Folders
  'File',
  'FileText',
  'Folder',
  'FolderOpen',
  'FolderPlus',
  'Image',
  'Film',
  'Music',

  // Layout & View
  'Grid',
  'List',
  'Columns',
  'Rows',
  'Maximize',
  'Minimize',
  'Expand',
  'Shrink',

  // Time & Date
  'Calendar',
  'Clock',
  'Timer',
  'Hourglass',

  // Misc UI
  'Home',
  'Bookmark',
  'Heart',
  'Star',
  'ThumbsUp',
  'ThumbsDown',
  'Flag',
  'Tag',
  'Hash',
  'Link',
  'Unlink',
  'Paperclip',
  'Pin',
  'PinOff',

  // Theme
  'Sun',
  'Moon',
  'Monitor',

  // Toggle states
  'ToggleLeft',
  'ToggleRight',
  'Circle',
  'CircleDot',
  'Square',
  'CheckSquare',

  // Misc
  'Zap',
  'Terminal',
  'Code',
  'Braces',
  'Database',
  'Server',
  'Wifi',
  'WifiOff',
  'Bluetooth',
  'Battery',
  'Power',
  'Palette',
] as const;

export type IconName = (typeof availableIcons)[number];

/**
 * Generate icon usage documentation for the agent context
 */
export function getIconUsageExamples(): string {
  return `## Icons (Lucide)

Import icons from 'lucide-svelte' and size them with Tailwind classes.

### Usage Examples:
\`\`\`svelte
<script lang="ts">
  import { Search, ChevronRight, AlertCircle, Loader2 } from 'lucide-svelte';
</script>

<!-- Basic icon -->
<Search class="w-5 h-5" />

<!-- Colored icon -->
<AlertCircle class="w-6 h-6 text-red-400" />

<!-- Icon in button -->
<button class="flex items-center gap-2">
  <Search class="w-4 h-4" />
  Search
</button>

<!-- Spinning loader -->
<Loader2 class="w-5 h-5 animate-spin" />
\`\`\`

### Icon Sizing:
- \`w-3 h-3\` - Extra small (12px)
- \`w-4 h-4\` - Small (16px)
- \`w-5 h-5\` - Medium (20px)
- \`w-6 h-6\` - Large (24px)
- \`w-8 h-8\` - Extra large (32px)

### Available Icons:
${availableIcons.join(', ')}`;
}

/**
 * Check if an icon name is valid
 */
export function isValidIcon(name: string): name is IconName {
  return availableIcons.includes(name as IconName);
}
