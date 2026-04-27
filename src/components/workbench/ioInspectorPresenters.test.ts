import test from 'node:test';
import assert from 'node:assert/strict';

import {
  classifyIoArtifactMedia,
  formatIoArtifactAvailabilityLabel,
  formatIoArtifactBytes,
  formatIoArtifactMediaLabel,
  formatProjectionFreshness,
} from './ioInspectorPresenters.ts';

test('classifyIoArtifactMedia groups common artifact media types', () => {
  assert.equal(classifyIoArtifactMedia('text/plain'), 'text');
  assert.equal(classifyIoArtifactMedia('image/png'), 'image');
  assert.equal(classifyIoArtifactMedia('audio/wav'), 'audio');
  assert.equal(classifyIoArtifactMedia('video/mp4'), 'video');
  assert.equal(classifyIoArtifactMedia('application/json'), 'json');
  assert.equal(classifyIoArtifactMedia('text/csv'), 'table');
  assert.equal(classifyIoArtifactMedia('application/parquet'), 'table');
  assert.equal(classifyIoArtifactMedia('application/octet-stream'), 'file');
  assert.equal(classifyIoArtifactMedia(null), 'unknown');
});

test('formatIoArtifactMediaLabel exposes stable UI labels', () => {
  assert.equal(formatIoArtifactMediaLabel('application/json'), 'JSON');
  assert.equal(formatIoArtifactMediaLabel('image/jpeg'), 'Image');
  assert.equal(formatIoArtifactMediaLabel(undefined), 'Unknown');
});

test('formatIoArtifactAvailabilityLabel distinguishes referenced and metadata-only artifacts', () => {
  assert.equal(formatIoArtifactAvailabilityLabel({ payload_ref: 'artifact://run/output' }), 'Payload referenced');
  assert.equal(formatIoArtifactAvailabilityLabel({ payload_ref: '' }), 'Metadata only');
  assert.equal(formatIoArtifactAvailabilityLabel({ payload_ref: null }), 'Metadata only');
});

test('formatIoArtifactBytes renders compact sizes', () => {
  assert.equal(formatIoArtifactBytes(null), 'Size unknown');
  assert.equal(formatIoArtifactBytes(999), '999 B');
  assert.equal(formatIoArtifactBytes(2_048), '2.0 KiB');
  assert.equal(formatIoArtifactBytes(2_097_152), '2.0 MiB');
});

test('formatProjectionFreshness keeps projection status visible', () => {
  assert.equal(formatProjectionFreshness(null), 'Projection unavailable');
  assert.equal(
    formatProjectionFreshness({
      projection_name: 'io_artifact',
      projection_version: 1,
      last_applied_event_seq: 42,
      status: 'rebuilding',
      rebuilt_at_ms: null,
      updated_at_ms: 100,
    }),
    'Rebuilding at seq 42',
  );
});
