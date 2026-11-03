import type {
  DiagnosticsRetentionPolicy,
  WorkflowRetentionCleanupResult,
} from '../../services/diagnostics/types';
import type {
  WorkflowArtifactFormatCapabilities,
  WorkflowManagedMediaDependencyStatus,
  WorkflowManagedMediaDependencyVersionStatus,
  WorkflowArtifactPolicy,
  WorkflowMediaFormatOption,
} from '../../services/workflow/types';
import type { GlyphState } from './ui/statusGlyph';
import type { KeyValueRow } from './ui/types';

export interface SettingsOptionItem {
  value: string;
  label: string;
  detail: string;
  unavailable?: boolean;
}

/** Settings rows render through the shared KeyValue part. */
export type SettingsPolicyRow = KeyValueRow;

export interface ManagedMediaDependencyStatusPresentation {
  installLabel: string;
  readinessLabel: string;
  categoryLabel: string;
  packageLabel: string;
  /** Decorative glyph beside `readinessLabel`; the label carries the status. */
  statusGlyph: GlyphState;
}

export interface NullableIntegerParseResult {
  value: number | null;
  error: string | null;
}

export interface NullableIntegerParseOptions {
  min?: number;
  max?: number | null;
}

export function formatSettingsBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) {
    return 'Unlimited';
  }
  if (bytes >= 1_073_741_824) {
    return `${(bytes / 1_073_741_824).toFixed(1)} GiB`;
  }
  if (bytes >= 1_048_576) {
    return `${(bytes / 1_048_576).toFixed(1)} MiB`;
  }
  if (bytes >= 1_024) {
    return `${(bytes / 1_024).toFixed(1)} KiB`;
  }
  return `${bytes} B`;
}

export function formatSettingsSeconds(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) {
    return 'No TTL';
  }
  if (seconds % 86_400 === 0) {
    const days = seconds / 86_400;
    return `${days} ${days === 1 ? 'day' : 'days'}`;
  }
  if (seconds % 3_600 === 0) {
    const hours = seconds / 3_600;
    return `${hours} ${hours === 1 ? 'hour' : 'hours'}`;
  }
  return `${seconds} seconds`;
}

export function formatBooleanPolicy(value: boolean): string {
  return value ? 'Enabled' : 'Disabled';
}

export function formatManagedMediaDependencyStatus(
  dependency: WorkflowManagedMediaDependencyStatus,
): ManagedMediaDependencyStatusPresentation {
  return {
    installLabel: sentenceCaseSnakeLabel(dependency.install_state),
    readinessLabel: sentenceCaseSnakeLabel(dependency.readiness),
    categoryLabel: sentenceCaseSnakeLabel(dependency.category),
    packageLabel: sentenceCaseSnakeLabel(dependency.catalog.package_kind),
    statusGlyph: managedMediaDependencyGlyph(dependency),
  };
}

export function buildManagedMediaDependencyRows(
  dependency: WorkflowManagedMediaDependencyStatus,
): SettingsPolicyRow[] {
  return [
    { key: 'Source', value: `${dependency.catalog.source.owner}/${dependency.catalog.source.project}` },
    { key: 'Catalog version', value: dependency.catalog.version, mono: true },
    { key: 'Platform', value: dependency.catalog.platform_key, mono: true },
    { key: 'Package', value: sentenceCaseSnakeLabel(dependency.catalog.package_kind) },
    { key: 'Archive', value: dependency.catalog.archive_kind ? sentenceCaseSnakeLabel(dependency.catalog.archive_kind) : 'None' },
    { key: 'Selected', value: dependency.selection.selected_version ?? 'None', mono: true },
    { key: 'Active', value: dependency.selection.active_version ?? 'None', mono: true },
    { key: 'Default', value: dependency.selection.default_version ?? 'None', mono: true },
    { key: 'Installed versions', value: String(dependency.versions.length), mono: true },
  ];
}

export function managedMediaVersionOptions(
  dependency: WorkflowManagedMediaDependencyStatus,
): string[] {
  const versions = dependency.versions.map((version) => version.version);
  return optionValuesWithCurrent(versions, dependency.selection.selected_version);
}

export function managedMediaVersionStatusLabel(
  version: WorkflowManagedMediaDependencyVersionStatus,
): string {
  const labels = [version.version];
  if (version.active) {
    labels.push('active');
  }
  if (version.selected) {
    labels.push('selected');
  }
  if (version.readiness !== 'ready') {
    labels.push(sentenceCaseSnakeLabel(version.readiness).toLowerCase());
  }
  return labels.join(' | ');
}

export function managedMediaDependencyGlyph(
  dependency: WorkflowManagedMediaDependencyStatus,
): GlyphState {
  if (dependency.readiness === 'ready' && dependency.install_state === 'installed') {
    return 'succeeded';
  }
  if (dependency.install_state === 'unsupported' || dependency.readiness === 'unsupported') {
    return 'idle';
  }
  return 'warn';
}

export function formatCapabilityCount(options: WorkflowMediaFormatOption[] | undefined): string {
  const count = options?.length ?? 0;
  return `${count} ${count === 1 ? 'format' : 'formats'}`;
}

/** One line for the format defaults heading, for example "Image 2 formats, audio 1 format, ...". */
export function formatCapabilitySummary(
  capabilities: WorkflowArtifactFormatCapabilities | null,
): string {
  return [
    `Image ${formatCapabilityCount(capabilities?.image_formats)}`,
    `audio ${formatCapabilityCount(capabilities?.audio_formats)}`,
    `video ${formatCapabilityCount(capabilities?.video_formats)}`,
    `3D ${formatCapabilityCount(capabilities?.three_d_formats)}`,
  ].join(', ');
}

export function buildCapabilityRows(
  capabilities: WorkflowArtifactFormatCapabilities | null,
): SettingsPolicyRow[] {
  return [
    { key: 'Image', value: formatCapabilityCount(capabilities?.image_formats), mono: true },
    { key: 'Audio', value: formatCapabilityCount(capabilities?.audio_formats), mono: true },
    { key: 'Video', value: formatCapabilityCount(capabilities?.video_formats), mono: true },
    { key: '3D', value: formatCapabilityCount(capabilities?.three_d_formats), mono: true },
  ];
}

export function buildArtifactPolicyRows(policy: WorkflowArtifactPolicy | null): SettingsPolicyRow[] {
  if (!policy) {
    return [
      { key: 'Policy', value: 'Unavailable' },
      { key: 'Version', value: 'Unavailable' },
      { key: 'TTL', value: 'Unavailable' },
      { key: 'Disk budget', value: 'Unavailable' },
      { key: 'Memory budget', value: 'Unavailable' },
      { key: 'Single artifact limit', value: 'Unavailable' },
      { key: 'Spill threshold', value: 'Unavailable' },
      { key: 'Delete on consume', value: 'Unavailable' },
    ];
  }

  return [
    { key: 'Policy', value: policy.policy_id, mono: true },
    { key: 'Version', value: String(policy.policy_version), mono: true },
    { key: 'TTL', value: formatSettingsSeconds(policy.ttl_seconds) },
    { key: 'Disk budget', value: formatSettingsBytes(policy.max_disk_bytes) },
    { key: 'Memory budget', value: formatSettingsBytes(policy.max_memory_bytes) },
    { key: 'Single artifact limit', value: formatSettingsBytes(policy.max_single_artifact_bytes) },
    { key: 'Spill threshold', value: formatSettingsBytes(policy.spill_threshold_bytes) },
    { key: 'Delete on consume', value: formatBooleanPolicy(policy.delete_on_consume) },
  ];
}

export function buildDiagnosticsRetentionPolicyRows(
  policy: DiagnosticsRetentionPolicy | null,
): SettingsPolicyRow[] {
  if (!policy) {
    return [
      { key: 'Policy', value: 'Unavailable' },
      { key: 'Version', value: 'Unavailable' },
      { key: 'Class', value: 'Unavailable' },
      { key: 'Days', value: 'Unavailable' },
    ];
  }

  return [
    { key: 'Policy', value: policy.policy_id, mono: true },
    { key: 'Version', value: String(policy.policy_version), mono: true },
    { key: 'Class', value: policy.retention_class, mono: true },
    { key: 'Days', value: String(policy.retention_days) },
  ];
}

export function buildDiagnosticsRetentionSettingRows(
  policy: DiagnosticsRetentionPolicy | null,
): SettingsPolicyRow[] {
  if (!policy) {
    return [];
  }

  const settings = policy.settings;
  return [
    {
      key: 'Final outputs',
      value: formatRetentionPolicyMode(settings.final_outputs),
    },
    {
      key: 'Workflow inputs',
      value: formatRetentionPolicyMode(settings.workflow_inputs),
    },
    {
      key: 'Intermediate node I/O',
      value: formatRetentionPolicyMode(settings.intermediate_node_io),
    },
    {
      key: 'Failed run data',
      value: formatRetentionPolicyMode(settings.failed_run_data),
    },
    {
      key: 'Cleanup trigger',
      value: sentenceCaseSnakeLabel(settings.cleanup_trigger),
    },
  ];
}

export function buildDiagnosticsRetentionCleanupRows(
  cleanup: WorkflowRetentionCleanupResult | null,
): SettingsPolicyRow[] {
  if (!cleanup) {
    return [];
  }

  return [
    { key: 'Policy', value: cleanup.policy_id, mono: true },
    { key: 'Version', value: String(cleanup.policy_version), mono: true },
    { key: 'Class', value: cleanup.retention_class, mono: true },
    { key: 'Cutoff', value: formatSettingsTimestamp(cleanup.cutoff_occurred_before_ms) },
    { key: 'Expired', value: String(cleanup.expired_artifact_count) },
    { key: 'Last event seq', value: String(cleanup.last_event_seq ?? 'Unavailable'), mono: true },
  ];
}

export function formatOptionItems(
  options: WorkflowMediaFormatOption[],
  currentValue?: string | null,
): SettingsOptionItem[] {
  const items: SettingsOptionItem[] = options.map((option) => ({
    value: option.format_id,
    label: option.display_name,
    detail: formatMediaFormatOptionDetail(option),
  }));

  if (
    currentValue &&
    currentValue.trim().length > 0 &&
    !items.some((item) => item.value === currentValue)
  ) {
    items.unshift({
      value: currentValue,
      label: `${currentValue} (unsupported)`,
      detail: 'Not reported by current capabilities',
      unavailable: true,
    });
  }

  return items;
}

export function formatMediaFormatOptionDetail(option: WorkflowMediaFormatOption): string {
  const parts = [option.media_type];
  if (option.codec_ids.length > 0) {
    parts.push(`codecs ${option.codec_ids.join(', ')}`);
  }
  if (option.provided_by_dependency_id) {
    parts.push(option.provided_by_dependency_id);
  }
  return parts.join(' | ');
}

export function findFormatOption(
  options: WorkflowMediaFormatOption[],
  formatId: string | null | undefined,
): WorkflowMediaFormatOption | null {
  return options.find((option) => option.format_id === formatId) ?? null;
}

export function formatRangeLabel(
  min: number | null | undefined,
  max: number | null | undefined,
  suffix = '',
): string {
  if (min === null || min === undefined || max === null || max === undefined) {
    return 'Backend validated';
  }
  return `${min}${suffix} to ${max}${suffix}`;
}

export function optionValuesWithCurrent(values: string[], currentValue?: string | null): string[] {
  if (
    currentValue &&
    currentValue.trim().length > 0 &&
    !values.includes(currentValue)
  ) {
    return [currentValue, ...values];
  }
  return values;
}

export function parseNullableIntegerField(
  label: string,
  rawValue: string,
  options: NullableIntegerParseOptions = {},
): NullableIntegerParseResult {
  const normalized = rawValue.trim();
  if (normalized.length === 0) {
    return { value: null, error: null };
  }

  if (!/^\d+$/.test(normalized)) {
    return { value: null, error: `${label} must be a whole number` };
  }

  const value = Number.parseInt(normalized, 10);
  if (!Number.isSafeInteger(value)) {
    return { value: null, error: `${label} is too large` };
  }
  if (options.min !== undefined && value < options.min) {
    return { value: null, error: `${label} must be at least ${options.min}` };
  }
  if (options.max !== null && options.max !== undefined && value > options.max) {
    return { value: null, error: `${label} must be at most ${options.max}` };
  }

  return { value, error: null };
}

function sentenceCaseSnakeLabel(value: string): string {
  const words = value.split('_').filter((part) => part.length > 0);
  if (words.length === 0) {
    return '';
  }
  const [first, ...rest] = words;
  return [`${first[0].toUpperCase()}${first.slice(1)}`, ...rest].join(' ');
}

function formatRetentionPolicyMode(
  policy: DiagnosticsRetentionPolicy['settings']['final_outputs'],
): string {
  return `${policy.retention_days} days, ${sentenceCaseSnakeLabel(policy.payload_mode)}`;
}

function formatSettingsTimestamp(value: number): string {
  return new Date(value).toLocaleString();
}
