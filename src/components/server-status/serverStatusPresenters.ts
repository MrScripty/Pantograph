import type { HealthStatus } from '../../services/HealthMonitorService.ts';
import type { GlyphState } from '../workbench/ui/statusGlyph.ts';

/**
 * Glyph for the LLM server header. The status word next to it comes from the
 * same inputs (`ServerStatus.svelte` statusText), so the glyph never carries
 * meaning the text does not.
 */
export function serverConnectionGlyph(
  ready: boolean,
  healthy: boolean | undefined,
  mode: string,
): GlyphState {
  if (ready) {
    return healthy === false ? 'failed' : 'succeeded';
  }
  return mode !== 'none' ? 'running' : 'idle';
}

/** Glyph for the last health check; unknown or missing results are idle. */
export function healthStatusGlyph(status: HealthStatus | null | undefined): GlyphState {
  switch (status?.type) {
    case 'healthy':
      return 'succeeded';
    case 'degraded':
      return 'warn';
    case 'unhealthy':
      return 'failed';
    default:
      return 'idle';
  }
}
