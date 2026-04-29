import type {
  WorkflowArtifactPolicy,
  WorkflowMediaFormatOption,
} from '../../services/workflow/types';

export interface SettingsOptionItem {
  value: string;
  label: string;
  detail: string;
  unavailable?: boolean;
}

export interface SettingsPolicyRow {
  label: string;
  value: string;
  mono?: boolean;
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

export function buildArtifactPolicyRows(policy: WorkflowArtifactPolicy | null): SettingsPolicyRow[] {
  if (!policy) {
    return [
      { label: 'Policy', value: 'Unavailable' },
      { label: 'Version', value: 'Unavailable' },
      { label: 'TTL', value: 'Unavailable' },
      { label: 'Disk Budget', value: 'Unavailable' },
      { label: 'Memory Budget', value: 'Unavailable' },
      { label: 'Single Artifact Limit', value: 'Unavailable' },
      { label: 'Spill Threshold', value: 'Unavailable' },
      { label: 'Delete On Consume', value: 'Unavailable' },
    ];
  }

  return [
    { label: 'Policy', value: policy.policy_id, mono: true },
    { label: 'Version', value: String(policy.policy_version), mono: true },
    { label: 'TTL', value: formatSettingsSeconds(policy.ttl_seconds) },
    { label: 'Disk Budget', value: formatSettingsBytes(policy.max_disk_bytes) },
    { label: 'Memory Budget', value: formatSettingsBytes(policy.max_memory_bytes) },
    { label: 'Single Artifact Limit', value: formatSettingsBytes(policy.max_single_artifact_bytes) },
    { label: 'Spill Threshold', value: formatSettingsBytes(policy.spill_threshold_bytes) },
    { label: 'Delete On Consume', value: formatBooleanPolicy(policy.delete_on_consume) },
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
