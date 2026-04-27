import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildIoArtifactNodeGroups,
  buildIoArtifactRendererSummary,
  classifyIoArtifactMedia,
  formatIoArtifactAvailabilityLabel,
  formatIoArtifactBytes,
  formatIoArtifactMediaLabel,
  formatIoArtifactRoleLabel,
  formatProjectionFreshness,
  isWorkflowInputArtifact,
  isWorkflowOutputArtifact,
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

test('buildIoArtifactRendererSummary maps media families to renderer states', () => {
  assert.deepEqual(buildIoArtifactRendererSummary({ media_type: 'image/png', payload_ref: 'artifact://image' }), {
    family: 'image',
    title: 'Image preview',
    detail: 'Payload reference retained',
  });
  assert.deepEqual(buildIoArtifactRendererSummary({ media_type: 'application/json', payload_ref: null }), {
    family: 'json',
    title: 'JSON',
    detail: 'Metadata retained only',
  });
  assert.deepEqual(buildIoArtifactRendererSummary({ media_type: undefined, payload_ref: '' }), {
    family: 'unknown',
    title: 'Unknown media',
    detail: 'Metadata retained only',
  });
});


test('formatIoArtifactAvailabilityLabel distinguishes referenced and metadata-only artifacts', () => {
  assert.equal(formatIoArtifactAvailabilityLabel({ payload_ref: 'artifact://run/output' }), 'Payload referenced');
  assert.equal(formatIoArtifactAvailabilityLabel({ payload_ref: '' }), 'Metadata only');
  assert.equal(formatIoArtifactAvailabilityLabel({ payload_ref: null }), 'Metadata only');
});

test('workflow artifact role helpers identify workflow boundaries', () => {
  assert.equal(isWorkflowInputArtifact({ artifact_role: 'workflow_input' }), true);
  assert.equal(isWorkflowInputArtifact({ artifact_role: 'node_input' }), false);
  assert.equal(isWorkflowOutputArtifact({ artifact_role: 'workflow_output' }), true);
  assert.equal(isWorkflowOutputArtifact({ artifact_role: 'node_output' }), false);
  assert.equal(formatIoArtifactRoleLabel('workflow_input'), 'Workflow input');
  assert.equal(formatIoArtifactRoleLabel('workflow_output'), 'Workflow output');
  assert.equal(formatIoArtifactRoleLabel('custom_role'), 'custom_role');
  assert.equal(formatIoArtifactRoleLabel(''), 'Unclassified');
});

test('buildIoArtifactNodeGroups groups node artifacts by latest event', () => {
  assert.deepEqual(
    buildIoArtifactNodeGroups([
      {
        node_id: 'node-a',
        node_type: 'text',
        artifact_role: 'node_input',
        event_seq: 2,
      },
      {
        node_id: 'node-b',
        node_type: 'image',
        artifact_role: 'node_output',
        event_seq: 4,
      },
      {
        node_id: 'node-a',
        node_type: null,
        artifact_role: 'node_output',
        event_seq: 3,
      },
      {
        node_id: null,
        node_type: null,
        artifact_role: 'workflow_output',
        event_seq: 5,
      },
    ]),
    [
      {
        node_id: 'node-b',
        node_type: 'image',
        input_count: 0,
        output_count: 1,
        artifact_count: 1,
        latest_event_seq: 4,
      },
      {
        node_id: 'node-a',
        node_type: 'text',
        input_count: 1,
        output_count: 1,
        artifact_count: 2,
        latest_event_seq: 3,
      },
    ],
  );
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
