/**
 * Status glyph vocabulary (docs/design/workbench-redesign-spec.md).
 *
 * Every page maps backend status strings through this one module, so a new
 * backend status changes the glyph in exactly one place. The glyph is always
 * decorative; callers render the status in text next to it.
 */

export const GLYPH_STATES = [
  'running',
  'waiting',
  'queued',
  'succeeded',
  'failed',
  'cancelled',
  'idle',
  'warn',
] as const;

export type GlyphState = (typeof GLYPH_STATES)[number];

/** Log severities the LogLine part understands. */
export type LogLevel = 'info' | 'warn' | 'error';

/**
 * Literal class names so Tailwind's content scan keeps them in the build.
 * Do not build these strings dynamically.
 */
export const GLYPH_CLASS: Record<GlyphState, string> = {
  running: 'pg-g-running',
  waiting: 'pg-g-waiting',
  queued: 'pg-g-queued',
  succeeded: 'pg-g-succeeded',
  failed: 'pg-g-failed',
  cancelled: 'pg-g-cancelled',
  idle: 'pg-g-idle',
  warn: 'pg-g-warn',
};

const GLYPH_LABEL: Record<GlyphState, string> = {
  running: 'Running',
  waiting: 'Waiting',
  queued: 'Queued',
  succeeded: 'Succeeded',
  failed: 'Failed',
  cancelled: 'Cancelled',
  idle: 'No run',
  warn: 'Warning',
};

const STATUS_TO_GLYPH: Record<string, GlyphState> = {
  // Run list projection statuses (RunListProjectionStatus).
  accepted: 'queued',
  future: 'queued',
  scheduled: 'queued',
  queued: 'queued',
  delayed: 'queued',
  running: 'running',
  completed: 'succeeded',
  failed: 'failed',
  cancelled: 'cancelled',
  // Node execution statuses (NodeExecutionProjectionStatus) and display labels.
  waiting: 'waiting',
  succeeded: 'succeeded',
  done: 'succeeded',
  next: 'queued',
  not_reached: 'queued',
  skipped: 'cancelled',
};

/**
 * Map a backend run or node status to its glyph. Matching ignores case and
 * surrounding whitespace so display labels such as `Waiting` map the same way
 * as projection values. Missing or unknown statuses render as `idle`.
 */
export function glyphStateForStatus(status: string | null | undefined): GlyphState {
  if (typeof status !== 'string') {
    return 'idle';
  }
  const key = status.trim().toLowerCase().replaceAll(' ', '_');
  return STATUS_TO_GLYPH[key] ?? 'idle';
}

export function glyphStateForLogLevel(level: LogLevel): GlyphState {
  switch (level) {
    case 'error':
      return 'failed';
    case 'warn':
      return 'warn';
    case 'info':
      return 'succeeded';
  }
}

export function glyphClass(state: GlyphState): string {
  return GLYPH_CLASS[state];
}

/** Plain-language name of a glyph state, for text that accompanies the glyph. */
export function glyphStateLabel(state: GlyphState): string {
  return GLYPH_LABEL[state];
}
