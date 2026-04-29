import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildArtifactPolicyRows,
  findFormatOption,
  formatOptionItems,
  formatRangeLabel,
  formatSettingsBytes,
  formatSettingsSeconds,
  optionValuesWithCurrent,
  parseNullableIntegerField,
} from './settingsPagePresenters.ts';
import type { WorkflowMediaFormatOption } from '../../services/workflow/types.ts';

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
