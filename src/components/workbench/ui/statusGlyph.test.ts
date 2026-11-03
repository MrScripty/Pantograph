import test from 'node:test';
import assert from 'node:assert/strict';

import {
  GLYPH_CLASS,
  GLYPH_STATES,
  glyphClass,
  glyphStateForLogLevel,
  glyphStateForStatus,
  glyphStateLabel,
} from './statusGlyph.ts';

test('run list statuses map to the spec glyph vocabulary', () => {
  for (const status of ['accepted', 'future', 'scheduled', 'queued', 'delayed']) {
    assert.equal(glyphStateForStatus(status), 'queued', status);
  }
  assert.equal(glyphStateForStatus('running'), 'running');
  assert.equal(glyphStateForStatus('completed'), 'succeeded');
  assert.equal(glyphStateForStatus('failed'), 'failed');
  assert.equal(glyphStateForStatus('cancelled'), 'cancelled');
});

test('node statuses and display labels map the same way', () => {
  assert.equal(glyphStateForStatus('waiting'), 'waiting');
  assert.equal(glyphStateForStatus('Waiting'), 'waiting');
  assert.equal(glyphStateForStatus(' Running '), 'running');
  assert.equal(glyphStateForStatus('Not reached'), 'queued');
  assert.equal(glyphStateForStatus('skipped'), 'cancelled');
});

test('missing or unknown statuses render as idle instead of guessing', () => {
  assert.equal(glyphStateForStatus(null), 'idle');
  assert.equal(glyphStateForStatus(undefined), 'idle');
  assert.equal(glyphStateForStatus(''), 'idle');
  assert.equal(glyphStateForStatus('exploded'), 'idle');
});

test('log levels map to succeeded, warn and failed glyphs', () => {
  assert.equal(glyphStateForLogLevel('info'), 'succeeded');
  assert.equal(glyphStateForLogLevel('warn'), 'warn');
  assert.equal(glyphStateForLogLevel('error'), 'failed');
});

test('every glyph state has a literal class and a text label', () => {
  for (const state of GLYPH_STATES) {
    assert.equal(glyphClass(state), GLYPH_CLASS[state]);
    assert.match(glyphClass(state), /^pg-g-[a-z]+$/);
    assert.ok(glyphStateLabel(state).length > 0);
  }
});
