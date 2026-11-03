import test from 'node:test';
import assert from 'node:assert/strict';

import type { IoArtifactProjectionRecord, ResolvedNodeIoRecord } from '../../services/diagnostics/types';
import type { WorkflowGraphDiagnostic } from '../../services/workflow/types';

import {
  assertCompleteIoArtifactImageRead,
  buildIoArtifactDescriptorMetadataRows,
  buildIoArtifactDownloadFilename,
  buildIoArtifactNodeGroups,
  buildIoArtifactPreviewReadRequest,
  buildIoArtifactRendererSummary,
  buildResolvedNodeIoDisplayRows,
  buildRetentionCleanupDetailRows,
  buildRetentionPolicyDetailRows,
  buildRetentionPolicySettingRows,
  canAcknowledgeIoArtifactConsumed,
  canReadIoArtifactBody,
  canRenderIoArtifactTextPreview,
  classifyIoArtifactMedia,
  decodeIoArtifactTextPreview,
  formatIoArtifactAvailabilityLabel,
  formatIoArtifactBytes,
  formatIoArtifactConversionStatusLabel,
  formatIoArtifactDetailValue,
  formatIoArtifactEndpointValue,
  formatIoArtifactLifecycleStateLabel,
  formatIoArtifactMediaLabel,
  formatIoArtifactPayloadKindLabel,
  formatIoArtifactPreviewExtent,
  formatIoArtifactRetentionStateLabel,
  formatIoArtifactRoleLabel,
  formatProjectionFreshness,
  formatResolvedNodeIoDirectionLabel,
  formatResolvedNodeIoProvenance,
  formatResolvedNodeIoResolutionLabel,
  ioArtifactPayloadTargetId,
  isNormalResolvedNodeIoDisplayRow,
  isWorkflowInputArtifact,
  isWorkflowOutputArtifact,
  IO_INSPECTOR_BOUNDARY_NODE_ID,
  applyIoInspectorBackendFilter,
  buildIoArtifactChipLabels,
  buildIoArtifactDetailSections,
  buildIoInspectorDimensionLabel,
  buildIoInspectorNodeTree,
  buildStaleGraphFactCards,
  countIoInspectorEntriesByDirection,
  createIoArtifactDownloadUrlOwner,
  describeIoArtifactAcknowledgeUnavailable,
  describeIoArtifactReadUnavailable,
  describeIoArtifactStreamUnavailable,
  filterIoInspectorNodeTree,
  findIoInspectorEntry,
  formatIoInspectorEntryDetail,
  formatIoInspectorNodeSubLabel,
  parseGraphCanvasViewBox,
  partitionGraphDiagnosticsByNode,
  resolveIoArtifactPreviewMode,
  resolveIoInspectorSelectedEntryKey,
  sentenceCaseLabel,
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
  assert.equal(classifyIoArtifactMedia(null, 'image'), 'image');
  assert.equal(classifyIoArtifactMedia(null, 'large_table'), 'table');
  assert.equal(classifyIoArtifactMedia(null, 'generic_binary'), 'file');
  assert.equal(classifyIoArtifactMedia(null), 'unknown');
});

test('formatIoArtifactMediaLabel exposes stable UI labels', () => {
  assert.equal(formatIoArtifactMediaLabel('application/json'), 'JSON');
  assert.equal(formatIoArtifactMediaLabel('image/jpeg'), 'Image');
  assert.equal(formatIoArtifactMediaLabel(undefined, '3d'), '3D');
  assert.equal(formatIoArtifactMediaLabel(undefined), 'Unknown');
});

test('artifact previews use bounded byte range requests and expose partial state', () => {
  assert.deepEqual(buildIoArtifactPreviewReadRequest('artifact-a'), {
    artifact_id: 'artifact-a',
    byte_range_start: 0,
    byte_range_end_exclusive: 64 * 1024,
  });
  assert.deepEqual(buildIoArtifactPreviewReadRequest('artifact-a', 0), {
    artifact_id: 'artifact-a',
    byte_range_start: 0,
    byte_range_end_exclusive: 1,
  });
  assert.equal(
    formatIoArtifactPreviewExtent({
      mediaType: 'text/plain',
      byteLength: 64 * 1024,
      complete: false,
    }),
    'text/plain · 64.0 KiB · partial preview',
  );
  assert.equal(
    formatIoArtifactPreviewExtent({
      mediaType: 'text/plain',
      byteLength: 128,
      complete: true,
    }),
    'text/plain · 128 B',
  );
});

test('artifact preview requests read complete images and bound text using card media classification', () => {
  assert.deepEqual(
    buildIoArtifactPreviewReadRequest({
      artifact_id: 'payload-image',
      media_type: 'image/png',
      payload_kind: 'image',
    }),
    { artifact_id: 'payload-image' },
  );
  assert.deepEqual(
    buildIoArtifactPreviewReadRequest({
      artifact_id: 'payload-image-by-kind',
      media_type: null,
      payload_kind: 'image',
    }),
    { artifact_id: 'payload-image-by-kind' },
  );
  assert.deepEqual(
    buildIoArtifactPreviewReadRequest({
      artifact_id: 'payload-image-by-format',
      media_type: null,
      payload_kind: null,
      format: { format_id: 'png', media_type: 'image/png' },
    }),
    { artifact_id: 'payload-image-by-format' },
  );
  assert.deepEqual(
    buildIoArtifactPreviewReadRequest({
      artifact_id: 'payload-text',
      media_type: 'text/plain',
      payload_kind: 'text',
    }),
    {
      artifact_id: 'payload-text',
      byte_range_start: 0,
      byte_range_end_exclusive: 64 * 1024,
    },
  );
  assert.deepEqual(
    buildIoArtifactPreviewReadRequest({
      artifact_id: 'payload-text-by-kind',
      media_type: null,
      payload_kind: 'text',
    }),
    {
      artifact_id: 'payload-text-by-kind',
      byte_range_start: 0,
      byte_range_end_exclusive: 64 * 1024,
    },
  );
  assert.deepEqual(
    buildIoArtifactPreviewReadRequest({
      artifact_id: 'payload-text-by-format',
      media_type: null,
      payload_kind: null,
      format: { format_id: 'txt', media_type: 'text/plain' },
    }),
    {
      artifact_id: 'payload-text-by-format',
      byte_range_start: 0,
      byte_range_end_exclusive: 64 * 1024,
    },
  );
});

test('complete image reads are accepted, including an opaque response media type', () => {
  assert.doesNotThrow(() => assertCompleteIoArtifactImageRead(
    { media_type: 'image/png', payload_kind: 'image' },
    { media_type: 'application/octet-stream', complete: true },
  ));
});

test('incomplete image reads are rejected before decoding, even with an opaque response media type', () => {
  assert.throws(
    () => assertCompleteIoArtifactImageRead(
      { media_type: 'image/png', payload_kind: 'image' },
      { media_type: 'application/octet-stream', complete: false },
    ),
    /complete image body is required/,
  );
  assert.throws(
    () => assertCompleteIoArtifactImageRead(
      { media_type: 'application/octet-stream', payload_kind: null },
      { media_type: 'image/png', complete: false },
    ),
    /complete image body is required/,
  );
  assert.throws(
    () => assertCompleteIoArtifactImageRead(
      { media_type: 'application/octet-stream', payload_kind: 'image' },
      { media_type: 'application/octet-stream', complete: false },
    ),
    /complete image body is required/,
  );
});

test('ioArtifactPayloadTargetId prefers retained payload identity over fact identity', () => {
  assert.equal(
    ioArtifactPayloadTargetId({
      artifact_id: 'fact-node-output',
      payload_artifact_id: 'payload-node-output',
    }),
    'payload-node-output',
  );
  assert.equal(
    ioArtifactPayloadTargetId({
      artifact_id: 'fact-node-output',
      payload_artifact_id: '   ',
    }),
    'fact-node-output',
  );
  assert.equal(
    ioArtifactPayloadTargetId({
      artifact_id: 'legacy-artifact',
    }),
    'legacy-artifact',
  );
});

test('resolved node io rows prefer canonical outputs over workflow boundary aliases', () => {
  const artifacts = [
    {
      artifact_id: 'fact-output',
      artifact_fact_id: 'fact-output',
      payload_artifact_id: 'payload-output',
      artifact_role: 'node_output',
      event_id: 'event-output',
    },
    {
      artifact_id: 'fact-boundary',
      artifact_fact_id: 'fact-boundary',
      payload_artifact_id: 'payload-output',
      artifact_role: 'workflow_output',
      event_id: 'event-boundary',
    },
  ];

  const rows = buildResolvedNodeIoDisplayRows(
    [
      {
        node_id: 'text-output',
        port_id: 'text',
        direction: 'output',
        resolution: 'workflow_boundary',
        artifact_fact_id: 'fact-boundary',
        payload_artifact_id: 'payload-output',
        artifact_id: 'fact-boundary',
      },
      {
        node_id: 'text-output',
        port_id: 'text',
        direction: 'output',
        resolution: 'produced_output',
        artifact_fact_id: 'fact-output',
        payload_artifact_id: 'payload-output',
        artifact_id: 'fact-output',
      },
    ],
    artifacts,
  );

  assert.equal(rows.length, 1);
  assert.equal(rows[0].nodeIo.resolution, 'produced_output');
  assert.equal(rows[0].artifact?.artifact_id, 'fact-output');
});

test('resolved node io rows resolve derived inputs to the upstream retained payload', () => {
  const artifacts = [
    {
      artifact_id: 'fact-inference-output',
      artifact_fact_id: 'fact-inference-output',
      payload_artifact_id: 'payload-text',
      artifact_role: 'node_output',
      event_id: 'event-inference-output',
    },
  ];

  const rows = buildResolvedNodeIoDisplayRows(
    [
      {
        node_id: 'text-output',
        port_id: 'text',
        direction: 'input',
        resolution: 'derived_from_edge',
        artifact_fact_id: 'fact-inference-output',
        payload_artifact_id: 'payload-text',
        artifact_id: 'fact-inference-output',
        upstream_node_id: 'inference',
        upstream_port_id: 'text',
      },
    ],
    artifacts,
  );

  assert.equal(rows.length, 1);
  assert.equal(rows[0].nodeIo.upstream_node_id, 'inference');
  assert.equal(rows[0].artifact?.artifact_id, 'fact-inference-output');
});

test('resolved node io presenters expose provenance and hide diagnostic data ports', () => {
  assert.equal(formatResolvedNodeIoDirectionLabel('input'), 'Input');
  assert.equal(formatResolvedNodeIoResolutionLabel('derived_from_edge'), 'Derived from edge');
  assert.equal(
    formatResolvedNodeIoProvenance({
      port_id: 'text',
      direction: 'input',
      resolution: 'derived_from_edge',
      provenance_kind: 'graph_edge',
      upstream_node_id: 'inference',
      upstream_port_id: 'response',
    }),
    'From inference:response',
  );
  assert.equal(
    formatResolvedNodeIoProvenance({
      port_id: 'text',
      direction: 'output',
      resolution: 'workflow_boundary',
      provenance_kind: 'workflow_output_boundary',
      upstream_node_id: null,
      upstream_port_id: null,
    }),
    'To workflow output boundary',
  );
  assert.equal(
    formatResolvedNodeIoProvenance({
      port_id: 'prompt',
      direction: 'input',
      resolution: 'explicit_input',
      provenance_kind: 'cache_replay',
      upstream_node_id: null,
      upstream_port_id: null,
    }),
    'Cache replay on prompt',
  );
  assert.equal(
    isNormalResolvedNodeIoDisplayRow({
      nodeIo: {
        node_id: 'node-a',
        port_id: '_data',
        direction: 'output',
        resolution: 'produced_output',
      },
      artifact: {
        artifact_id: 'artifact-data',
        artifact_fact_id: 'artifact-data',
        payload_artifact_id: 'artifact-data',
        producer_port_id: '_data',
      },
    }),
    false,
  );
});

test('artifact text previews decode readable artifact families inline', () => {
  assert.equal(canRenderIoArtifactTextPreview('text/plain'), true);
  assert.equal(canRenderIoArtifactTextPreview('application/json'), true);
  assert.equal(canRenderIoArtifactTextPreview('text/csv'), true);
  assert.equal(canRenderIoArtifactTextPreview('image/png'), false);
  assert.equal(canRenderIoArtifactTextPreview(null, 'structured'), true);

  assert.deepEqual(decodeIoArtifactTextPreview([72, 101, 108, 108, 111]), {
    text: 'Hello',
    truncated: false,
  });
  assert.deepEqual(decodeIoArtifactTextPreview(new TextEncoder().encode('abcdef'), 3), {
    text: 'abc',
    truncated: true,
  });
});

test('buildIoArtifactRendererSummary maps media families to renderer states', () => {
  assert.deepEqual(
    buildIoArtifactRendererSummary({
      media_type: 'image/png',
      payload_ref: 'artifact://image',
      retention_state: 'retained',
    }),
    {
      family: 'image',
      title: 'Image preview',
      detail: 'Payload retained',
    },
  );
  assert.deepEqual(
    buildIoArtifactRendererSummary({
      media_type: null,
      payload_ref: null,
      retention_state: 'metadata_only',
      payload_kind: 'structured',
    }),
    {
      family: 'json',
      title: 'JSON',
      detail: 'Metadata retained only',
    },
  );
  assert.deepEqual(buildIoArtifactRendererSummary({ media_type: undefined, payload_ref: '' }), {
    family: 'unknown',
    title: 'Unknown media',
    detail: 'Retention unknown',
  });
});


test('formatIoArtifactAvailabilityLabel distinguishes referenced and metadata-only artifacts', () => {
  assert.equal(
    formatIoArtifactAvailabilityLabel({
      payload_ref: null,
      read_handle: 'artifact-read://run/output',
      retention_state: 'retained',
    }),
    'Payload referenced',
  );
  assert.equal(
    formatIoArtifactAvailabilityLabel({
      payload_ref: null,
      stream_handle: 'artifact-stream://run/output',
      retention_state: 'retained',
    }),
    'Payload referenced',
  );
  assert.equal(formatIoArtifactAvailabilityLabel({ payload_ref: '' }), 'Metadata only');
  assert.equal(
    formatIoArtifactAvailabilityLabel({
      payload_ref: 'artifact://run/output',
      retention_state: 'expired',
    }),
    'Metadata only',
  );
});

test('artifact body controls are limited to retained readable artifacts', () => {
  assert.equal(
    canReadIoArtifactBody({
      retention_state: 'retained',
      lifecycle_state: 'retained',
      read_handle: 'artifact-read://run/output',
      access_modes: ['read', 'download'],
    }),
    true,
  );
  assert.equal(
    canReadIoArtifactBody({
      retention_state: 'metadata_only',
      lifecycle_state: 'retained',
      read_handle: 'artifact-read://run/output',
      access_modes: ['read'],
    }),
    false,
  );
  assert.equal(
    canReadIoArtifactBody({
      retention_state: 'retained',
      lifecycle_state: 'writing',
      read_handle: 'artifact-read://run/output',
      access_modes: ['read'],
    }),
    false,
  );
  assert.equal(canAcknowledgeIoArtifactConsumed({ artifact_id: 'artifact-a', retention_state: 'retained' }), true);
  assert.equal(
    canAcknowledgeIoArtifactConsumed({ artifact_id: 'artifact-a', retention_state: 'metadata_only' }),
    false,
  );
});

test('buildIoArtifactDownloadFilename keeps filenames local and media-derived', () => {
  assert.equal(
    buildIoArtifactDownloadFilename({
      artifact_id: '../artifact:one',
      media_type: 'image/jpeg',
    }),
    'artifact-one.jpg',
  );
  assert.equal(
    buildIoArtifactDownloadFilename({
      artifact_id: 'table-result',
      media_type: 'text/csv',
    }),
    'table-result.csv',
  );
  assert.equal(
    buildIoArtifactDownloadFilename({
      artifact_id: 'json-result',
      payload_kind: 'structured',
    }),
    'json-result.json',
  );
});

test('artifact descriptor labels and rows tolerate absent optional projection fields', () => {
  assert.equal(formatIoArtifactPayloadKindLabel('generic_binary'), 'Generic binary');
  assert.equal(formatIoArtifactPayloadKindLabel(undefined), 'Payload kind unknown');
  assert.equal(formatIoArtifactLifecycleStateLabel('streaming'), 'Streaming');
  assert.equal(formatIoArtifactLifecycleStateLabel(null), 'Lifecycle unknown');

  assert.deepEqual(
    buildIoArtifactDescriptorMetadataRows({
      payload_kind: undefined,
      lifecycle_state: null,
      access_modes: undefined,
      read_handle: null,
      stream_handle: null,
      runtime_id: null,
      runtime_version: null,
      selected_backend_key: null,
      model_id: null,
      model_version: null,
      format: null,
    }),
    [
      { label: 'Payload Kind', value: 'Payload kind unknown', mono: false },
      { label: 'Lifecycle', value: 'Lifecycle unknown', mono: false },
      { label: 'Access', value: 'Unavailable', mono: false },
    ],
  );
});

test('artifact descriptor rows expose handle and format metadata without payload bodies', () => {
  const rows = buildIoArtifactDescriptorMetadataRows({
    payload_kind: 'image',
    lifecycle_state: 'retained',
    access_modes: ['read', 'download'],
    read_handle: 'artifact-read://artifact-run-1-image',
    stream_handle: null,
    runtime_id: 'runtime-transformers',
    runtime_version: '0.1.0',
    selected_backend_key: 'vllm',
    model_id: 'pumas://models/image-alpha',
    model_version: 'rev-a',
    format: {
      format_id: 'jpg',
      media_type: 'image/jpeg',
      codec_id: null,
      quality_percent: 75,
      bitrate_kbps: null,
      crf: null,
      bit_depth: '8bit',
      color_profile_id: 'srgb',
      converter_id: 'oiiotool',
      converter_version: '2.5.0',
      library_version: 'openimageio-2.5.0',
      conversion_id: 'conversion-image-1',
      conversion_status: 'converted',
      conversion_command_id: 'image_oiio_ocio_jpg_srgb',
      conversion_dependencies: [
        {
          dependency_id: 'oiiotool',
          active_version: '2.5.0',
          lease_id: 'lease-oiio-1',
          lease_holder: 'workflow_run:run-a/node:image-output/port:image/conversion:conversion-image-1',
        },
      ],
    },
  });

  assert.equal(rows.find((row) => row.label === 'Payload Kind')?.value, 'Image');
  assert.equal(rows.find((row) => row.label === 'Lifecycle')?.value, 'Retained');
  assert.equal(rows.find((row) => row.label === 'Access')?.value, 'Read, Download');
  assert.equal(rows.find((row) => row.label === 'Read Handle')?.value, 'artifact-read://artifact-run-1-image');
  assert.equal(rows.find((row) => row.label === 'Runtime')?.value, 'runtime-transformers');
  assert.equal(rows.find((row) => row.label === 'Backend')?.value, 'vllm');
  assert.equal(rows.find((row) => row.label === 'Model')?.value, 'pumas://models/image-alpha');
  assert.equal(rows.find((row) => row.label === 'Format')?.value, 'jpg');
  assert.equal(rows.find((row) => row.label === 'Format Media')?.value, 'image/jpeg');
  assert.equal(rows.find((row) => row.label === 'Quality')?.value, '75%');
  assert.equal(rows.find((row) => row.label === 'Bit Depth')?.value, '8bit');
  assert.equal(rows.find((row) => row.label === 'Library Version')?.value, 'openimageio-2.5.0');
  assert.equal(rows.find((row) => row.label === 'Conversion')?.value, 'Converted');
  assert.equal(rows.find((row) => row.label === 'Conversion Id')?.value, 'conversion-image-1');
  assert.equal(rows.find((row) => row.label === 'Conversion Command')?.value, 'image_oiio_ocio_jpg_srgb');
  assert.equal(rows.find((row) => row.label === 'Dependency 1')?.value, 'oiiotool@2.5.0 · lease-oiio-1');
  assert.equal(
    rows.find((row) => row.label === 'Lease Holder 1')?.value,
    'workflow_run:run-a/node:image-output/port:image/conversion:conversion-image-1',
  );
});

test('formatIoArtifactConversionStatusLabel exposes backend conversion states', () => {
  assert.equal(formatIoArtifactConversionStatusLabel('converted'), 'Converted');
  assert.equal(formatIoArtifactConversionStatusLabel('passed_through'), 'Passed through');
  assert.equal(formatIoArtifactConversionStatusLabel('failed'), 'Failed');
  assert.equal(formatIoArtifactConversionStatusLabel(undefined), 'Conversion unknown');
});

test('formatIoArtifactRetentionStateLabel exposes typed retention state labels', () => {
  assert.equal(formatIoArtifactRetentionStateLabel('retained'), 'Payload retained');
  assert.equal(formatIoArtifactRetentionStateLabel('external'), 'External reference');
  assert.equal(formatIoArtifactRetentionStateLabel('truncated'), 'Payload truncated');
  assert.equal(formatIoArtifactRetentionStateLabel('too_large'), 'Too large to retain');
  assert.equal(formatIoArtifactRetentionStateLabel('expired'), 'Payload expired');
  assert.equal(formatIoArtifactRetentionStateLabel('deleted'), 'Payload deleted');
  assert.equal(formatIoArtifactRetentionStateLabel(undefined), 'Retention unknown');
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
        producer_node_id: null,
        consumer_node_id: 'node-a',
        artifact_role: 'node_input',
        event_seq: 2,
      },
      {
        node_id: 'node-b',
        node_type: 'image',
        producer_node_id: 'node-b',
        consumer_node_id: null,
        artifact_role: 'node_output',
        event_seq: 4,
      },
      {
        node_id: 'node-a',
        node_type: null,
        producer_node_id: 'node-a',
        consumer_node_id: null,
        artifact_role: 'node_output',
        event_seq: 3,
      },
      {
        node_id: null,
        node_type: null,
        producer_node_id: null,
        consumer_node_id: null,
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

test('buildIoArtifactNodeGroups uses endpoint fields before event node ids', () => {
  assert.deepEqual(
    buildIoArtifactNodeGroups([
      {
        node_id: 'edge-observer',
        node_type: 'bridge',
        producer_node_id: 'producer-node',
        consumer_node_id: 'consumer-node',
        artifact_role: 'node_output',
        event_seq: 7,
      },
    ]),
    [
      {
        node_id: 'consumer-node',
        node_type: null,
        input_count: 1,
        output_count: 0,
        artifact_count: 1,
        latest_event_seq: 7,
      },
      {
        node_id: 'producer-node',
        node_type: null,
        input_count: 0,
        output_count: 1,
        artifact_count: 1,
        latest_event_seq: 7,
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

test('formatIoArtifactDetailValue keeps missing projection details explicit', () => {
  assert.equal(formatIoArtifactDetailValue('runtime-a'), 'runtime-a');
  assert.equal(formatIoArtifactDetailValue(''), 'Unavailable');
  assert.equal(formatIoArtifactDetailValue(null), 'Unavailable');
  assert.equal(formatIoArtifactEndpointValue('node-a', 'out'), 'node-a:out');
  assert.equal(formatIoArtifactEndpointValue('node-a', null), 'node-a');
  assert.equal(formatIoArtifactEndpointValue(null, 'out'), 'Unavailable');
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

test('retention policy detail rows expose backend policy state', () => {
  const policy = {
    policy_id: 'standard-local-v1',
    policy_version: 3,
    retention_class: 'standard',
    retention_days: 30,
    settings: {
      final_outputs: { retention_days: 30, payload_mode: 'retain_payload_reference' },
      workflow_inputs: { retention_days: 30, payload_mode: 'retain_payload_reference' },
      intermediate_node_io: { retention_days: 14, payload_mode: 'metadata_only' },
      failed_run_data: { retention_days: 7, payload_mode: 'metadata_only' },
      max_artifact_bytes: null,
      max_total_storage_bytes: 1_073_741_824,
      media_behavior: 'metadata_and_reference_only',
      compression_behavior: 'not_configured',
      cleanup_trigger: 'manual_or_maintenance',
    },
    applied_at_ms: 86_400_000,
    explanation: 'short local history',
  } as const;
  const rows = buildRetentionPolicyDetailRows(policy);

  assert.equal(rows.find((row) => row.label === 'Policy')?.value, 'standard-local-v1');
  assert.equal(rows.find((row) => row.label === 'Version')?.value, '3');
  assert.equal(rows.find((row) => row.label === 'Class')?.value, 'standard');
  assert.equal(rows.find((row) => row.label === 'Days')?.value, '30');
  assert.match(rows.find((row) => row.label === 'Applied')?.value ?? '', /1970/);
  assert.deepEqual(buildRetentionPolicyDetailRows(null), []);
});

test('retention policy setting rows expose first-pass global setting groups', () => {
  const rows = buildRetentionPolicySettingRows({
    policy_id: 'standard-local-v1',
    policy_version: 3,
    retention_class: 'standard',
    retention_days: 30,
    settings: {
      final_outputs: { retention_days: 30, payload_mode: 'retain_payload_reference' },
      workflow_inputs: { retention_days: 30, payload_mode: 'retain_payload_reference' },
      intermediate_node_io: { retention_days: 14, payload_mode: 'metadata_only' },
      failed_run_data: { retention_days: 7, payload_mode: 'metadata_only' },
      max_artifact_bytes: null,
      max_total_storage_bytes: 1_073_741_824,
      media_behavior: 'metadata_and_reference_only',
      compression_behavior: 'not_configured',
      cleanup_trigger: 'manual_or_maintenance',
    },
    applied_at_ms: 86_400_000,
    explanation: 'short local history',
  });

  assert.equal(rows.find((row) => row.label === 'Final Outputs')?.value, '30 days, Retain Payload Reference');
  assert.equal(rows.find((row) => row.label === 'Intermediate Node I/O')?.value, '14 days, Metadata Only');
  assert.equal(rows.find((row) => row.label === 'Maximum Artifact Size')?.value, 'Size unknown');
  assert.equal(rows.find((row) => row.label === 'Maximum Total Storage')?.value, '1.0 GiB');
  assert.equal(rows.find((row) => row.label === 'Media Behavior')?.value, 'Metadata And Reference Only');
  assert.equal(rows.find((row) => row.label === 'Cleanup Trigger')?.value, 'Manual Or Maintenance');
  assert.deepEqual(buildRetentionPolicySettingRows(null), []);
});

test('retention cleanup detail rows expose backend cleanup status', () => {
  const rows = buildRetentionCleanupDetailRows({
    policy_id: 'standard-local-v1',
    policy_version: 4,
    retention_class: 'standard',
    cutoff_occurred_before_ms: 172_800_000,
    expired_artifact_count: 12,
    last_event_seq: 44,
  });

  assert.equal(rows.find((row) => row.label === 'Policy')?.value, 'standard-local-v1');
  assert.equal(rows.find((row) => row.label === 'Version')?.value, '4');
  assert.match(rows.find((row) => row.label === 'Cutoff')?.value ?? '', /1970/);
  assert.equal(rows.find((row) => row.label === 'Expired')?.value, '12');
  assert.equal(rows.find((row) => row.label === 'Last Event Seq')?.value, '44');
  assert.deepEqual(buildRetentionCleanupDetailRows(null), []);
});

// --- Node and port tree (workbench redesign M3) -----------------------------


function treeArtifact(overrides: Partial<IoArtifactProjectionRecord> & { artifact_id: string }): IoArtifactProjectionRecord {
  return {
    event_seq: 1,
    event_id: `event-${overrides.artifact_id}`,
    occurred_at_ms: 1_000,
    recorded_at_ms: 1_000,
    workflow_run_id: 'run-1',
    workflow_id: 'wf',
    artifact_role: 'node_output',
    retention_state: 'retained',
    ...overrides,
  };
}

function resolved(overrides: Partial<ResolvedNodeIoRecord> & Pick<ResolvedNodeIoRecord, 'node_id' | 'port_id' | 'direction'>): ResolvedNodeIoRecord {
  return { resolution: overrides.direction === 'input' ? 'derived_from_edge' : 'produced_output', ...overrides };
}

const promptText = treeArtifact({
  artifact_id: 'art-prompt',
  producer_node_id: 'prompt',
  producer_port_id: 'text',
  consumer_node_id: 'sample',
  consumer_port_id: 'prompt',
  media_type: 'text/plain',
  payload_kind: 'text',
  size_bytes: 412,
});
const decodedImage = treeArtifact({
  artifact_id: 'art-image',
  producer_node_id: 'decode',
  producer_port_id: 'image',
  media_type: 'image/png',
  payload_kind: 'image',
  size_bytes: 4 * 1_048_576,
  artifact_role: 'workflow_output',
});
const latents = treeArtifact({
  artifact_id: 'art-latents',
  producer_node_id: 'sample',
  producer_port_id: 'latents',
  media_type: 'application/octet-stream',
  payload_kind: 'generic_binary',
  retention_state: 'metadata_only',
  size_bytes: 33 * 1_048_576,
});

function sampleTree() {
  return buildIoInspectorNodeTree({
    graphNodes: [
      { id: 'prompt', node_type: 'text-input' },
      { id: 'sample', node_type: 'pytorch-inference' },
      { id: 'decode', node_type: 'image-output' },
      { id: 'unload', node_type: 'unload-model' },
    ],
    nodeStatuses: [
      { node_id: 'prompt', status: 'completed' },
      { node_id: 'sample', status: 'running' },
      { node_id: 'unload', status: 'queued' },
    ],
    resolvedNodeIo: [
      resolved({ node_id: 'prompt', port_id: 'text', direction: 'output', artifact_id: 'art-prompt' }),
      resolved({
        node_id: 'sample',
        port_id: 'prompt',
        direction: 'input',
        artifact_id: 'art-prompt',
        upstream_node_id: 'prompt',
        upstream_port_id: 'text',
      }),
      resolved({ node_id: 'sample', port_id: 'latents', direction: 'output', artifact_id: 'art-latents' }),
      resolved({ node_id: 'sample', port_id: '_data', direction: 'output', artifact_id: 'art-latents' }),
    ],
    // decodedImage has no resolved row: it must come from its producer metadata.
    artifacts: [promptText, latents, decodedImage],
  });
}

test('node tree lists snapshot nodes in graph order with inputs before outputs', () => {
  const tree = sampleTree();
  assert.deepEqual(tree.map((node) => node.nodeId), ['prompt', 'sample', 'decode', 'unload']);
  const sample = tree[1];
  assert.equal(sample.nodeType, 'pytorch-inference');
  assert.equal(sample.status, 'running');
  assert.deepEqual(
    sample.entries.map((entry) => [entry.direction, entry.portId, entry.artifact.artifact_id]),
    [
      ['input', 'prompt', 'art-prompt'],
      ['output', 'latents', 'art-latents'],
    ],
    'the _data diagnostic port is not listed',
  );
  const decode = tree[2];
  assert.deepEqual(decode.entries.map((entry) => [entry.direction, entry.portId, entry.nodeIo]), [['output', 'image', null]]);
  assert.deepEqual(tree[3].entries, []);
});

test('node tree puts artifacts with no node under a trailing workflow boundary group', () => {
  const tree = buildIoInspectorNodeTree({
    graphNodes: [{ id: 'a', node_type: 'x' }],
    nodeStatuses: [],
    resolvedNodeIo: [],
    artifacts: [treeArtifact({ artifact_id: 'loose', artifact_role: 'workflow_input' })],
  });
  assert.deepEqual(tree.map((node) => [node.nodeId, node.boundary]), [['a', false], [IO_INSPECTOR_BOUNDARY_NODE_ID, true]]);
  assert.equal(tree[1].entries[0].direction, 'input');
  assert.equal(formatIoInspectorNodeSubLabel(tree[1]), '1 retained port');
});

test('node tree does not duplicate an artifact the resolved rows already place', () => {
  const tree = sampleTree();
  const promptEntries = tree.flatMap((node) => node.entries).filter((entry) => entry.artifact.artifact_id === 'art-prompt');
  assert.deepEqual(promptEntries.map((entry) => `${entry.nodeId}:${entry.portId}`), ['prompt:text', 'sample:prompt']);
  assert.equal(new Set(tree.flatMap((node) => node.entries.map((entry) => entry.key))).size, 4);
});

test('direction and search filters narrow entries and count per direction', () => {
  const tree = sampleTree();
  assert.deepEqual(countIoInspectorEntriesByDirection(tree), { all: 4, input: 1, output: 3 });

  const inputs = filterIoInspectorNodeTree(tree, { direction: 'input', search: '' });
  assert.equal(inputs.length, 4, 'nodes stay listed without search text');
  assert.deepEqual(countIoInspectorEntriesByDirection(inputs), { all: 1, input: 1, output: 0 });

  const byPort = filterIoInspectorNodeTree(tree, { direction: 'all', search: 'LATENT' });
  assert.deepEqual(byPort.map((node) => [node.nodeId, node.entries.map((entry) => entry.portId)]), [['sample', ['latents']]]);

  const byNodeType = filterIoInspectorNodeTree(tree, { direction: 'all', search: 'image-output' });
  assert.deepEqual(byNodeType.map((node) => node.nodeId), ['decode']);

  const byArtifactId = filterIoInspectorNodeTree(tree, { direction: 'output', search: 'art-prompt' });
  assert.deepEqual(byArtifactId.map((node) => node.nodeId), ['prompt']);
});

test('selection keeps the chosen entry and otherwise prefers an image output', () => {
  const tree = sampleTree();
  const imageKey = resolveIoInspectorSelectedEntryKey(null, tree);
  assert.equal(findIoInspectorEntry(tree, imageKey)?.artifact.artifact_id, 'art-image');

  const latentsKey = tree[1].entries[1].key;
  assert.equal(resolveIoInspectorSelectedEntryKey(latentsKey, tree), latentsKey);
  assert.equal(resolveIoInspectorSelectedEntryKey('gone', tree), imageKey, 'a vanished entry falls back');

  const noImage = buildIoInspectorNodeTree({ graphNodes: [], nodeStatuses: [], resolvedNodeIo: [], artifacts: [promptText] });
  assert.equal(findIoInspectorEntry(noImage, resolveIoInspectorSelectedEntryKey(null, noImage))?.direction, 'output');
  assert.equal(resolveIoInspectorSelectedEntryKey('x', []), null);
  assert.equal(findIoInspectorEntry(tree, null), null);
});

test('node and entry labels read as sentences', () => {
  const tree = sampleTree();
  assert.equal(formatIoInspectorNodeSubLabel(tree[0]), 'Succeeded, 1 retained port');
  assert.equal(formatIoInspectorNodeSubLabel(tree[1], 2), 'Running, 2 retained ports, 2 stale graph facts');
  assert.equal(formatIoInspectorNodeSubLabel(tree[2]), 'No status recorded, 1 retained port');
  assert.equal(formatIoInspectorNodeSubLabel(tree[3]), 'Queued, not reached yet');
  assert.equal(formatIoInspectorEntryDetail(decodedImage), 'Image, payload retained');
  assert.equal(formatIoInspectorEntryDetail(latents), 'Generic binary, metadata retained only');
  assert.equal(formatIoInspectorEntryDetail({ ...promptText, lifecycle_state: 'streaming' }), 'Text, streaming');
  assert.deepEqual(buildIoArtifactChipLabels(decodedImage), ['Workflow output', 'Image', 'Payload retained', 'image/png, 4.0 MiB']);
});

test('dimension line counts nodes, artifacts and kept payload bytes', () => {
  assert.equal(
    buildIoInspectorDimensionLabel({ runId: 'run-1', nodeCount: 4, artifacts: [promptText, latents, decodedImage] }),
    'run-1, 4 nodes, 3 retained artifacts, 4.0 MiB of payloads kept',
  );
  assert.equal(
    buildIoInspectorDimensionLabel({ runId: 'run-1', nodeCount: 1, artifacts: [] }),
    'run-1, 1 node, 0 retained artifacts, 0 B of payloads kept',
  );
  assert.equal(
    buildIoInspectorDimensionLabel({ runId: 'run-1', nodeCount: 1, artifacts: [], freshness: 'Current at seq 12' }),
    'run-1, 1 node, 0 retained artifacts, 0 B of payloads kept, projection current at seq 12',
  );
});

test('backend filter keeps matching artifacts and the resolved rows that point at them', () => {
  const cuda = treeArtifact({ artifact_id: 'a', artifact_fact_id: 'fact-a', selected_backend_key: 'cuda' });
  const cpu = treeArtifact({ artifact_id: 'b', payload_artifact_id: 'payload-b', selected_backend_key: 'cpu' });
  const rows = [
    resolved({ node_id: 'n', port_id: 'p', direction: 'output', artifact_fact_id: 'fact-a' }),
    resolved({ node_id: 'n', port_id: 'q', direction: 'output', payload_artifact_id: 'payload-b' }),
  ];
  const unfiltered = applyIoInspectorBackendFilter([cuda, cpu], rows, '  ');
  assert.equal(unfiltered.artifacts.length, 2);
  assert.equal(unfiltered.resolvedNodeIo.length, 2);
  const filtered = applyIoInspectorBackendFilter([cuda, cpu], rows, ' cuda ');
  assert.deepEqual(filtered.artifacts.map((artifact) => artifact.artifact_id), ['a']);
  assert.deepEqual(filtered.resolvedNodeIo.map((row) => row.port_id), ['p']);
});

test('detail sections split origin, making and storage facts', () => {
  const sections = buildIoArtifactDetailSections(
    {
      artifact: {
        ...decodedImage,
        runtime_id: 'diffusers',
        selected_backend_key: 'cuda',
        read_handle: 'artifact-read://x',
        lifecycle_state: 'retained',
        access_modes: ['read', 'download'],
        content_hash: 'sha256:abc',
        format: { format_id: 'png', media_type: 'image/png', bit_depth: '8-bit' },
      },
      nodeIo: null,
      direction: 'output',
      portId: 'image',
    },
    (ms) => `t${ms}`,
  );
  assert.deepEqual(sections.origin.map((row) => [row.key, row.value]), [
    ['Producer', 'decode:image'],
    ['Consumer', 'Unavailable'],
    ['Direction', 'Output'],
    ['Observed', 't1000'],
    ['Provenance', 'From artifact metadata; no resolved node I/O row'],
  ]);
  assert.deepEqual(sections.making.map((row) => row.key), ['Runtime', 'Backend', 'Format', 'Format media', 'Bit depth']);
  assert.deepEqual(sections.storage.map((row) => row.key), [
    'Payload kind',
    'Lifecycle',
    'Access',
    'Read handle',
    'Retention',
    'Availability',
    'Size',
    'Payload ref',
    'Content hash',
  ]);
  const withRow = buildIoArtifactDetailSections(
    { artifact: promptText, nodeIo: resolved({ node_id: 'sample', port_id: 'prompt', direction: 'input', upstream_node_id: 'prompt', upstream_port_id: 'text' }), direction: 'input', portId: 'prompt' },
    String,
  );
  assert.equal(withRow.origin.find((row) => row.key === 'Provenance')?.value, 'From prompt:text');
  assert.equal(withRow.making[0].key, 'Backend', 'backend is always shown');
  assert.equal(sentenceCaseLabel('Conversion Command'), 'Conversion command');
  assert.equal(sentenceCaseLabel('CRF'), 'CRF');
});

test('preview mode follows the media family and the loaded body', () => {
  assert.equal(resolveIoArtifactPreviewMode('image', null), 'none');
  assert.equal(resolveIoArtifactPreviewMode('image', { text: null }), 'image');
  assert.equal(resolveIoArtifactPreviewMode('audio', { text: null }), 'audio');
  assert.equal(resolveIoArtifactPreviewMode('json', { text: '{}' }), 'text');
  assert.equal(resolveIoArtifactPreviewMode('text', { text: '' }), 'text');
  assert.equal(resolveIoArtifactPreviewMode('video', { text: null }), 'binary');
  assert.equal(resolveIoArtifactPreviewMode('file', {}), 'binary');
});

test('disabled artifact actions explain why', () => {
  assert.equal(describeIoArtifactReadUnavailable({ retention_state: 'retained', read_handle: 'h' }), null);
  assert.equal(
    describeIoArtifactReadUnavailable({ retention_state: 'metadata_only' }),
    'Metadata retained only, so there is no body to read.',
  );
  assert.equal(
    describeIoArtifactReadUnavailable({ retention_state: 'retained', read_handle: 'h', lifecycle_state: 'streaming' }),
    'The body is streaming; it can be read once it is retained.',
  );
  assert.match(describeIoArtifactReadUnavailable({ retention_state: 'retained' }) ?? '', /did not grant/);
  assert.equal(describeIoArtifactStreamUnavailable({ stream_handle: 's' }), null);
  assert.equal(describeIoArtifactStreamUnavailable({}), 'This body was not written as a stream.');
  assert.equal(describeIoArtifactAcknowledgeUnavailable({ artifact_id: 'a', retention_state: 'retained' }), null);
  assert.match(describeIoArtifactAcknowledgeUnavailable({ artifact_id: 'a', retention_state: 'expired' }) ?? '', /retained/);
});

test('stale graph facts become muted cards, grouped by node', () => {
  const diagnostics: WorkflowGraphDiagnostic[] = [
    { code: 'unknown_node_type', severity: 'error', scope: 'node', node_id: 'sample', message: 'Node type is not registered.', blocking_submission: true },
    { code: 'unknown_node_type', severity: 'error', scope: 'node', node_id: 'sample', message: 'Node type is not registered.', blocking_submission: true },
    { code: 'duplicate_edge_id', severity: 'warning', scope: 'graph', message: 'Two edges share an id.', blocking_submission: false },
  ];
  const cards = buildStaleGraphFactCards(diagnostics);
  assert.equal(cards.length, 2, 'duplicates collapse');
  assert.deepEqual(cards[0], {
    key: cards[0].key,
    title: 'Unknown node type',
    code: 'Error, unknown_node_type',
    explanation: 'Node type is not registered.',
    suggestion: 'Try: open the workflow in Graph and fix this node before running it again.',
  });
  assert.equal(cards[1].suggestion, null);
  const parts = partitionGraphDiagnosticsByNode(diagnostics);
  assert.equal(parts.graph.length, 1);
  assert.equal(parts.byNodeId.get('sample')?.length, 2);
});

test('canvas view box parses strings and objects and falls back when malformed', () => {
  assert.deepEqual(parseGraphCanvasViewBox('-96 -10 400 300'), { x: -96, y: -10, width: 400, height: 300 });
  assert.deepEqual(parseGraphCanvasViewBox({ x: 1, y: 2, width: 3, height: 4 }), { x: 1, y: 2, width: 3, height: 4 });
  assert.deepEqual(parseGraphCanvasViewBox('nope'), { x: 0, y: 0, width: 640, height: 360 });
  assert.deepEqual(parseGraphCanvasViewBox('0 0 0 10'), { x: 0, y: 0, width: 640, height: 360 });
  assert.deepEqual(parseGraphCanvasViewBox(undefined), { x: 0, y: 0, width: 640, height: 360 });
});

test('download URL owner revokes after the delay or on dispose, exactly once', () => {
  const revoked: string[] = [];
  const timers = new Map<number, () => void>();
  let nextHandle = 1;
  const owner = createIoArtifactDownloadUrlOwner({
    delayMs: 30_000,
    revoke: (url) => revoked.push(url),
    schedule: (callback) => {
      const handle = nextHandle++;
      timers.set(handle, callback);
      return handle;
    },
    cancel: (handle) => timers.delete(handle),
  });

  owner.release('blob:a');
  owner.release('blob:b');
  assert.equal(owner.pendingCount(), 2);
  timers.get(1)?.();
  timers.delete(1);
  assert.deepEqual(revoked, ['blob:a']);
  assert.equal(owner.pendingCount(), 1);

  owner.dispose();
  assert.deepEqual(revoked, ['blob:a', 'blob:b']);
  assert.equal(timers.size, 0, 'pending timers are cancelled');
  assert.equal(owner.pendingCount(), 0);

  owner.release('blob:late');
  assert.deepEqual(revoked, ['blob:a', 'blob:b', 'blob:late'], 'a URL released after dispose is revoked at once');
  assert.equal(timers.size, 0);
});
