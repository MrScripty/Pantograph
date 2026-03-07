import type { PortDataType } from './types/workflow.js';

/**
 * Keep package-side connection validation aligned with the Rust workflow types.
 */
export function isPortTypeCompatible(
  source: PortDataType,
  target: PortDataType
): boolean {
  if (source === 'any' || target === 'any') return true;
  if (source === target) return true;

  if (
    (source === 'prompt' && target === 'string') ||
    (source === 'string' && target === 'prompt')
  ) {
    return true;
  }

  if (
    (source === 'audio_stream' && target === 'stream') ||
    (source === 'stream' && target === 'audio_stream')
  ) {
    return true;
  }

  if (target === 'string') {
    return source === 'json' || source === 'number' || source === 'boolean';
  }

  return false;
}
