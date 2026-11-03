import test from 'node:test';
import assert from 'node:assert/strict';
import {
  managedDependencyGlyph,
  managedRuntimeJobGlyph,
  managedRuntimeReadinessGlyph,
  sentenceCaseState,
} from './managedRuntimePresenters.ts';

test('readiness glyph covers every readiness state', () => {
  assert.equal(managedRuntimeReadinessGlyph('ready'), 'succeeded');
  assert.equal(managedRuntimeReadinessGlyph('downloading'), 'running');
  assert.equal(managedRuntimeReadinessGlyph('extracting'), 'running');
  assert.equal(managedRuntimeReadinessGlyph('validating'), 'running');
  assert.equal(managedRuntimeReadinessGlyph('failed'), 'failed');
  assert.equal(managedRuntimeReadinessGlyph('missing'), 'warn');
  assert.equal(managedRuntimeReadinessGlyph('unknown'), 'idle');
  assert.equal(managedRuntimeReadinessGlyph('unsupported'), 'idle');
});

test('dependency glyph treats system-provided binaries as usable', () => {
  assert.equal(managedDependencyGlyph('missing', 'system_provided'), 'succeeded');
  assert.equal(managedDependencyGlyph('missing', 'missing'), 'warn');
  assert.equal(managedDependencyGlyph('ready', 'installed'), 'succeeded');
  assert.equal(managedDependencyGlyph('unsupported', 'unsupported'), 'idle');
});

test('job glyph covers every job state', () => {
  assert.equal(managedRuntimeJobGlyph('queued'), 'queued');
  assert.equal(managedRuntimeJobGlyph('downloading'), 'running');
  assert.equal(managedRuntimeJobGlyph('extracting'), 'running');
  assert.equal(managedRuntimeJobGlyph('validating'), 'running');
  assert.equal(managedRuntimeJobGlyph('paused'), 'waiting');
  assert.equal(managedRuntimeJobGlyph('ready'), 'succeeded');
  assert.equal(managedRuntimeJobGlyph('failed'), 'failed');
  assert.equal(managedRuntimeJobGlyph('cancelled'), 'cancelled');
});

test('state words render as sentence case', () => {
  assert.equal(sentenceCaseState('system_provided'), 'System provided');
  assert.equal(sentenceCaseState('ready'), 'Ready');
  assert.equal(sentenceCaseState(''), '');
});
