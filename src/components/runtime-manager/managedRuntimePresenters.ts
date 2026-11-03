import type {
  ManagedBinaryInstallState,
  ManagedRuntimeJobState,
  ManagedRuntimeReadinessState,
} from '../../services/managedRuntime/types.ts';
import type { GlyphState } from '../workbench/ui/statusGlyph.ts';

/**
 * Glyph for a managed runtime or dependency readiness state. The readiness
 * word is always rendered next to it; the glyph only repeats it.
 */
export function managedRuntimeReadinessGlyph(state: ManagedRuntimeReadinessState): GlyphState {
  switch (state) {
    case 'ready':
      return 'succeeded';
    case 'downloading':
    case 'extracting':
    case 'validating':
      return 'running';
    case 'failed':
      return 'failed';
    case 'missing':
      return 'warn';
    case 'unknown':
    case 'unsupported':
      return 'idle';
  }
}

/** System-provided binaries count as usable even when readiness lags. */
export function managedDependencyGlyph(
  readiness: ManagedRuntimeReadinessState,
  installState: ManagedBinaryInstallState,
): GlyphState {
  if (installState === 'system_provided') {
    return 'succeeded';
  }
  return managedRuntimeReadinessGlyph(readiness);
}

/** Glyph for an active install or download job. */
export function managedRuntimeJobGlyph(state: ManagedRuntimeJobState): GlyphState {
  switch (state) {
    case 'queued':
      return 'queued';
    case 'downloading':
    case 'extracting':
    case 'validating':
      return 'running';
    case 'paused':
      return 'waiting';
    case 'ready':
      return 'succeeded';
    case 'failed':
      return 'failed';
    case 'cancelled':
      return 'cancelled';
  }
}

/** `system_provided` → `System provided`: backend snake-case words as sentence-case copy. */
export function sentenceCaseState(value: string): string {
  const words = value.replaceAll('_', ' ').trim();
  return words.length === 0 ? words : words.charAt(0).toUpperCase() + words.slice(1);
}
