import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildDiagnosticsRetentionCleanupRows,
  buildDiagnosticsRetentionPolicyRows,
  buildDiagnosticsRetentionSettingRows,
  buildManagedMediaDependencyRows,
  buildArtifactPolicyRows,
  findFormatOption,
  formatManagedMediaDependencyStatus,
  formatOptionItems,
  formatRangeLabel,
  formatSettingsBytes,
  formatSettingsSeconds,
  managedMediaVersionOptions,
  managedMediaVersionStatusLabel,
  optionValuesWithCurrent,
  parseNullableIntegerField,
} from './settingsPagePresenters.ts';
import type { DiagnosticsRetentionPolicy } from '../../services/diagnostics/types.ts';
import type {
  WorkflowManagedMediaDependencyStatus,
  WorkflowMediaFormatOption,
} from '../../services/workflow/types.ts';

const jpgOption: WorkflowMediaFormatOption = {
  format_id: 'jpg',
  display_name: 'JPEG',
  media_type: 'image/jpeg',
  codec_ids: [],
  quality_min_percent: 1,
  quality_max_percent: 100,
  bitrate_min_kbps: null,
  bitrate_max_kbps: null,
  crf_min: null,
  crf_max: null,
  bit_depths: ['8bit'],
  color_profile_ids: ['srgb'],
  provided_by_dependency_id: 'oiiotool',
  provided_by_version: null,
};

const oggOption: WorkflowMediaFormatOption = {
  format_id: 'ogg',
  display_name: 'Ogg',
  media_type: 'audio/ogg',
  codec_ids: ['opus', 'vorbis'],
  quality_min_percent: null,
  quality_max_percent: null,
  bitrate_min_kbps: 32,
  bitrate_max_kbps: 512,
  crf_min: null,
  crf_max: null,
  bit_depths: [],
  color_profile_ids: [],
  provided_by_dependency_id: 'ffmpeg',
  provided_by_version: null,
};

test('settings byte and duration labels distinguish unlimited values', () => {
  assert.equal(formatSettingsBytes(null), 'Unlimited');
  assert.equal(formatSettingsBytes(512), '512 B');
  assert.equal(formatSettingsBytes(2048), '2.0 KiB');
  assert.equal(formatSettingsBytes(1_048_576), '1.0 MiB');

  assert.equal(formatSettingsSeconds(null), 'No TTL');
  assert.equal(formatSettingsSeconds(86_400), '1 day');
  assert.equal(formatSettingsSeconds(172_800), '2 days');
  assert.equal(formatSettingsSeconds(3_600), '1 hour');
  assert.equal(formatSettingsSeconds(90), '90 seconds');
});

test('artifact policy rows preserve backend ids and nullable budgets', () => {
  const rows = buildArtifactPolicyRows({
    policy_id: 'artifact-policy-v1',
    policy_version: 3,
    ttl_seconds: 604_800,
    max_disk_bytes: null,
    max_memory_bytes: 4096,
    max_single_artifact_bytes: 1024,
    spill_threshold_bytes: null,
    delete_on_consume: true,
  });

  assert.equal(rows.find((row) => row.label === 'Policy')?.value, 'artifact-policy-v1');
  assert.equal(rows.find((row) => row.label === 'Version')?.value, '3');
  assert.equal(rows.find((row) => row.label === 'TTL')?.value, '7 days');
  assert.equal(rows.find((row) => row.label === 'Disk Budget')?.value, 'Unlimited');
  assert.equal(rows.find((row) => row.label === 'Memory Budget')?.value, '4.0 KiB');
  assert.equal(rows.find((row) => row.label === 'Delete On Consume')?.value, 'Enabled');
});

test('diagnostics retention policy rows expose global retention settings', () => {
  const policy = diagnosticsRetentionPolicy();

  const rows = buildDiagnosticsRetentionPolicyRows(policy);
  assert.equal(rows.find((row) => row.label === 'Policy')?.value, 'standard-local-v1');
  assert.equal(rows.find((row) => row.label === 'Version')?.value, '3');
  assert.equal(rows.find((row) => row.label === 'Days')?.value, '30');

  const settingRows = buildDiagnosticsRetentionSettingRows(policy);
  assert.equal(settingRows.find((row) => row.label === 'Final Outputs')?.value, '30 days, Retain Payload Reference');
  assert.equal(settingRows.find((row) => row.label === 'Intermediate Node I/O')?.value, '14 days, Metadata Only');
  assert.equal(settingRows.find((row) => row.label === 'Cleanup Trigger')?.value, 'Manual Or Maintenance');
});

test('diagnostics retention cleanup rows expose backend cleanup result', () => {
  const rows = buildDiagnosticsRetentionCleanupRows({
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
  assert.deepEqual(buildDiagnosticsRetentionCleanupRows(null), []);
});

test('format options use backend capability labels and expose unsupported current values', () => {
  assert.deepEqual(formatOptionItems([jpgOption]), [
    {
      value: 'jpg',
      label: 'JPEG',
      detail: 'image/jpeg | oiiotool',
    },
  ]);

  assert.deepEqual(formatOptionItems([jpgOption], 'webp')[0], {
    value: 'webp',
    label: 'webp (unsupported)',
    detail: 'Not reported by current capabilities',
    unavailable: true,
  });

  assert.equal(formatOptionItems([oggOption])[0].detail, 'audio/ogg | codecs opus, vorbis | ffmpeg');
  assert.equal(findFormatOption([oggOption], 'ogg'), oggOption);
  assert.equal(findFormatOption([oggOption], 'wav'), null);
});

test('range and value helpers preserve backend-provided choices', () => {
  assert.equal(formatRangeLabel(1, 100, '%'), '1% to 100%');
  assert.equal(formatRangeLabel(null, 100), 'Backend validated');
  assert.deepEqual(optionValuesWithCurrent(['8bit', '10bit'], 'float'), ['float', '8bit', '10bit']);
  assert.deepEqual(optionValuesWithCurrent(['8bit', '10bit'], '8bit'), ['8bit', '10bit']);
});

test('nullable integer parsing accepts blank optional values and reports simple field errors', () => {
  assert.deepEqual(parseNullableIntegerField('Disk budget', ''), { value: null, error: null });
  assert.deepEqual(parseNullableIntegerField('Disk budget', ' 1024 ', { min: 0 }), {
    value: 1024,
    error: null,
  });
  assert.deepEqual(parseNullableIntegerField('Disk budget', '1.5'), {
    value: null,
    error: 'Disk budget must be a whole number',
  });
  assert.deepEqual(parseNullableIntegerField('TTL seconds', '0', { min: 1 }), {
    value: null,
    error: 'TTL seconds must be at least 1',
  });
  assert.deepEqual(parseNullableIntegerField('Quality', '101', { min: 1, max: 100 }), {
    value: null,
    error: 'Quality must be at most 100',
  });
});

test('managed media dependency presenters expose backend status and version facts', () => {
  const dependency = managedMediaDependencyStatus();

  assert.deepEqual(formatManagedMediaDependencyStatus(dependency), {
    installLabel: 'Installed',
    readinessLabel: 'Ready',
    categoryLabel: 'Tool Binary',
    packageLabel: 'Archive',
    statusClass: 'border-emerald-800 bg-emerald-950/40 text-emerald-200',
  });

  const rows = buildManagedMediaDependencyRows(dependency);
  assert.equal(rows.find((row) => row.label === 'Source')?.value, 'FFmpeg/FFmpeg');
  assert.equal(rows.find((row) => row.label === 'Catalog Version')?.value, '7.1');
  assert.equal(rows.find((row) => row.label === 'Selected')?.value, '7.1');
  assert.equal(rows.find((row) => row.label === 'Installed Versions')?.value, '1');

  assert.deepEqual(managedMediaVersionOptions(dependency), ['7.1']);
  assert.equal(managedMediaVersionStatusLabel(dependency.versions[0]), '7.1 | active | selected');
});

function managedMediaDependencyStatus(): WorkflowManagedMediaDependencyStatus {
  return {
    id: 'ffmpeg',
    display_name: 'FFmpeg',
    category: 'tool_binary',
    install_state: 'installed',
    readiness: 'ready',
    available: true,
    missing_files: [],
    catalog: {
      id: 'ffmpeg',
      display_name: 'FFmpeg',
      category: 'tool_binary',
      source: {
        owner: 'FFmpeg',
        project: 'FFmpeg',
      },
      license_redistribution:
        'LGPL-2.1-or-later/GPL-2.0-or-later depending on enabled codecs',
      platform_key: 'linux-x86_64',
      version: '7.1',
      package_kind: 'archive',
      archive_kind: 'tar_gz',
      archive_name: null,
      download_url: null,
      expected_files: ['bin/ffmpeg'],
      checksum_sha256: null,
      signature: null,
    },
    selection: {
      selected_version: '7.1',
      active_version: '7.1',
      default_version: '7.1',
    },
    versions: [
      {
        version: '7.1',
        platform_key: 'linux-x86_64',
        install_root: '/tmp/pantograph/managed/ffmpeg/7.1',
        expected_files: ['bin/ffmpeg'],
        missing_files: [],
        install_state: 'installed',
        readiness: 'ready',
        selected: true,
        active: true,
      },
    ],
  };
}

function diagnosticsRetentionPolicy(): DiagnosticsRetentionPolicy {
  return {
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
      max_total_storage_bytes: null,
      media_behavior: 'metadata_and_reference_only',
      compression_behavior: 'not_configured',
      cleanup_trigger: 'manual_or_maintenance',
    },
    applied_at_ms: 1_000,
    explanation: 'test policy',
  };
}
