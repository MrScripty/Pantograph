/**
 * Design System
 *
 * Provides design tokens, icons, and validation for generated components.
 */

export { designTokens, getAllowedClasses, type DesignTokens } from './tokens';
export {
  availableIcons,
  getIconUsageExamples,
  isValidIcon,
  type IconName,
} from './icons';
export {
  buildDesignSystemContext,
  getCompactDesignContext,
} from './agentContext';
export {
  validateComponent,
  formatValidationErrors,
  hasLikelyViolations,
  type ValidationResult,
} from './validator';
