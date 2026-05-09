import type {
  DiagnosticsRetentionPolicy,
  WorkflowRetentionCleanupResult,
} from '../../services/diagnostics/types';
import type {
  WorkflowManagedMediaDependencyStatus,
  WorkflowManagedMediaDependencyVersionStatus,
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

export interface ManagedMediaDependencyStatusPresentation {
  installLabel: string;
  readinessLabel: string;
  categoryLabel: string;
  packageLabel: string;
  statusClass: string;
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
    installLabel: titleCaseSnakeLabel(dependency.install_state),
    readinessLabel: titleCaseSnakeLabel(dependency.readiness),
    categoryLabel: titleCaseSnakeLabel(dependency.category),
    packageLabel: titleCaseSnakeLabel(dependency.catalog.package_kind),
    statusClass: managedMediaDependencyStatusClass(dependency),
  };
}

export function buildManagedMediaDependencyRows(
  dependency: WorkflowManagedMediaDependencyStatus,
): SettingsPolicyRow[] {
  return [
    { label: 'Source', value: `${dependency.catalog.source.owner}/${dependency.catalog.source.project}` },
    { label: 'Catalog Version', value: dependency.catalog.version, mono: true },
    { label: 'Platform', value: dependency.catalog.platform_key, mono: true },
    { label: 'Package', value: titleCaseSnakeLabel(dependency.catalog.package_kind) },
    { label: 'Archive', value: dependency.catalog.archive_kind ? titleCaseSnakeLabel(dependency.catalog.archive_kind) : 'None' },
    { label: 'Selected', value: dependency.selection.selected_version ?? 'None', mono: true },
    { label: 'Active', value: dependency.selection.active_version ?? 'None', mono: true },
    { label: 'Default', value: dependency.selection.default_version ?? 'None', mono: true },
    { label: 'Installed Versions', value: String(dependency.versions.length), mono: true },
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
    labels.push(titleCaseSnakeLabel(version.readiness).toLowerCase());
  }
  return labels.join(' | ');
}

export function managedMediaDependencyStatusClass(
  dependency: WorkflowManagedMediaDependencyStatus,
): string {
  if (dependency.readiness === 'ready' && dependency.install_state === 'installed') {
    return 'border-emerald-800 bg-emerald-950/40 text-emerald-200';
  }
  if (dependency.install_state === 'unsupported' || dependency.readiness === 'unsupported') {
    return 'border-neutral-700 bg-neutral-900 text-neutral-400';
  }
  return 'border-amber-800 bg-amber-950/40 text-amber-200';
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

export function buildDiagnosticsRetentionPolicyRows(
  policy: DiagnosticsRetentionPolicy | null,
): SettingsPolicyRow[] {
  if (!policy) {
    return [
      { label: 'Policy', value: 'Unavailable' },
      { label: 'Version', value: 'Unavailable' },
      { label: 'Class', value: 'Unavailable' },
      { label: 'Days', value: 'Unavailable' },
    ];
  }

  return [
    { label: 'Policy', value: policy.policy_id, mono: true },
    { label: 'Version', value: String(policy.policy_version), mono: true },
    { label: 'Class', value: policy.retention_class, mono: true },
    { label: 'Days', value: String(policy.retention_days) },
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
      label: 'Final Outputs',
      value: formatRetentionPolicyMode(settings.final_outputs),
    },
    {
      label: 'Workflow Inputs',
      value: formatRetentionPolicyMode(settings.workflow_inputs),
    },
    {
      label: 'Intermediate Node I/O',
      value: formatRetentionPolicyMode(settings.intermediate_node_io),
    },
    {
      label: 'Failed Run Data',
      value: formatRetentionPolicyMode(settings.failed_run_data),
    },
    {
      label: 'Cleanup Trigger',
      value: titleCaseSnakeLabel(settings.cleanup_trigger),
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
    { label: 'Policy', value: cleanup.policy_id, mono: true },
    { label: 'Version', value: String(cleanup.policy_version), mono: true },
    { label: 'Class', value: cleanup.retention_class, mono: true },
    { label: 'Cutoff', value: formatSettingsTimestamp(cleanup.cutoff_occurred_before_ms) },
    { label: 'Expired', value: String(cleanup.expired_artifact_count) },
    { label: 'Last Event Seq', value: String(cleanup.last_event_seq ?? 'Unavailable'), mono: true },
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

function titleCaseSnakeLabel(value: string): string {
  return value
    .split('_')
    .filter((part) => part.length > 0)
    .map((part) => `${part[0].toUpperCase()}${part.slice(1)}`)
    .join(' ');
}

function formatRetentionPolicyMode(
  policy: DiagnosticsRetentionPolicy['settings']['final_outputs'],
): string {
  return `${policy.retention_days} days, ${titleCaseSnakeLabel(policy.payload_mode)}`;
}

function formatSettingsTimestamp(value: number): string {
  return new Date(value).toLocaleString();
}
