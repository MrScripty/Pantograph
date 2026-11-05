import test from 'node:test';
import assert from 'node:assert/strict';
import { healthStatusGlyph, serverConnectionGlyph } from './serverStatusPresenters.ts';

test('server connection glyph follows readiness, health and mode', () => {
  assert.equal(serverConnectionGlyph(true, true, 'external'), 'succeeded');
  assert.equal(serverConnectionGlyph(true, undefined, 'sidecar_inference'), 'succeeded');
  assert.equal(serverConnectionGlyph(true, false, 'external'), 'failed');
  assert.equal(serverConnectionGlyph(false, undefined, 'sidecar_inference'), 'running');
  assert.equal(serverConnectionGlyph(false, false, 'none'), 'idle');
});

test('health status glyph maps each health status type', () => {
  assert.equal(healthStatusGlyph({ type: 'healthy' }), 'succeeded');
  assert.equal(healthStatusGlyph({ type: 'degraded', reason: 'slow' }), 'warn');
  assert.equal(healthStatusGlyph({ type: 'unhealthy', reason: 'down' }), 'failed');
  assert.equal(healthStatusGlyph({ type: 'unknown' }), 'idle');
  assert.equal(healthStatusGlyph(null), 'idle');
  assert.equal(healthStatusGlyph(undefined), 'idle');
});
