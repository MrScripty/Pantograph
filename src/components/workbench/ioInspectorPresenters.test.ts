import test from 'node:test';
import assert from 'node:assert/strict';

import {
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
