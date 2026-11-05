import type {
  LibraryUsageProjectionRecord,
  ProjectionStateRecord,
  PumasHfDownloadRequest,
  PumasHfDownloadStartAuditResponse,
  PumasHfModelSearchAuditRequest,
  PumasHfModelSearchAuditResponse,
  PumasHfModelSearchResult,
  PumasModelDeleteAuditResponse,
} from '../../services/diagnostics/types';
import type { PortOption } from '../../services/workflow/types';
import type { KeyValueRow, SegmentOption } from './ui/types';
import { formatProjectionFreshnessState } from './projectionFreshness.ts';

export type LibraryAssetCategory =
  | 'model'
  | 'runtime'
  | 'workflow'
  | 'node'
  | 'template'
  | 'connector'
  | 'pumas'
  | 'pantograph'
  | 'unclassified';

export function classifyLibraryAsset(assetId: string): LibraryAssetCategory {
  const normalized = assetId.toLowerCase();
  const [prefix] = normalized.split(/[:/]/, 1);
  switch (prefix) {
    case 'model':
    case 'models':
      return 'model';
    case 'runtime':
    case 'runtimes':
      return 'runtime';
    case 'workflow':
    case 'workflows':
      return 'workflow';
    case 'node':
    case 'nodes':
      return 'node';
    case 'template':
    case 'templates':
      return 'template';
    case 'connector':
    case 'connectors':
      return 'connector';
    case 'pumas':
    case 'puma':
      return 'pumas';
    case 'pantograph':
      return 'pantograph';
    default:
      return 'unclassified';
  }
}

export function formatLibraryAssetCategory(assetId: string): string {
  switch (classifyLibraryAsset(assetId)) {
    case 'model':
      return 'Model';
    case 'runtime':
      return 'Runtime';
    case 'workflow':
      return 'Workflow';
    case 'node':
      return 'Node';
    case 'template':
      return 'Template';
    case 'connector':
      return 'Connector';
    case 'pumas':
      return 'Pumas';
    case 'pantograph':
      return 'Pantograph';
    case 'unclassified':
      return 'Unclassified';
  }
}

export function isLibraryAssetLastUsedByRun(
  asset: Pick<LibraryUsageProjectionRecord, 'last_workflow_run_id'>,
  workflowRunId: string | null | undefined,
): boolean {
  return Boolean(workflowRunId && asset.last_workflow_run_id === workflowRunId);
}

export function formatLibraryBytes(bytes: number): string {
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

export function formatLibraryProjectionFreshness(state: ProjectionStateRecord | null): string {
  return formatProjectionFreshnessState(state);
}

// ---------------------------------------------------------------------------
// Library list: Pumas models joined with the library usage projection.
// ---------------------------------------------------------------------------

/**
 * Asset id prefixes the host writes into `library.asset_accessed` events
 * (`session_execution_api.rs::pumas_model_asset_id`,
 * `puma_lib_commands.rs::record_*_audit`). The join below depends on them.
 */
export const PUMAS_MODEL_ASSET_PREFIX = 'pumas://models/';
export const HF_MODEL_ASSET_PREFIX = 'hf://models/';

/** "Unused for 30 days" boundary. */
export const LIBRARY_UNUSED_AFTER_MS = 30 * 24 * 60 * 60 * 1000;

export type LibraryModelKind = 'text' | 'image' | 'audio';
export type LibraryKindFilter = 'all' | LibraryModelKind;

export interface LibraryModelRow {
  /** Pumas model id (selector row `model_ref.model_id`). */
  modelId: string;
  /** Usage-projection asset id for this model: `pumas://models/<modelId>`. */
  assetId: string;
  name: string;
  modelType: string | null;
  kind: LibraryModelKind | null;
  repoId: string | null;
  backend: string | null;
  path: string | null;
  storageKind: string | null;
  validationState: string | null;
  pipelineTag: string | null;
  taskType: string | null;
  /** Set when the selector reports the row as not executable. */
  unavailableReason: string | null;
  /** Usage record for `assetId`, when one exists. */
  usage: LibraryUsageProjectionRecord | null;
  /** Usage record for `hf://models/<repoId>` (download audits), when one exists. */
  downloadUsage: LibraryUsageProjectionRecord | null;
}

export function pumasModelAssetId(modelId: string): string {
  return `${PUMAS_MODEL_ASSET_PREFIX}${modelId}`;
}

function nonEmptyString(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function metadataString(option: PortOption, key: string): string | null {
  return nonEmptyString(option.metadata?.[key]);
}

/** The Pumas model id of a selector option, or null when the option carries none. */
export function libraryModelIdFromOption(option: PortOption): string | null {
  const fromMetadata = metadataString(option, 'id');
  if (fromMetadata) return fromMetadata;
  const value = option.value;
  if (value && typeof value === 'object' && 'model_id' in value) {
    return nonEmptyString((value as { model_id?: unknown }).model_id);
  }
  return nonEmptyString(value);
}

const KIND_BY_MODEL_TYPE: Record<string, LibraryModelKind> = {
  llm: 'text',
  reranker: 'text',
  embedding: 'text',
  diffusion: 'image',
  vision: 'image',
  audio: 'audio',
};

const KIND_BY_TASK: Record<string, LibraryModelKind> = {
  'text-generation': 'text',
  'text2text-generation': 'text',
  'feature-extraction': 'text',
  'sentence-similarity': 'text',
  'fill-mask': 'text',
  summarization: 'text',
  translation: 'text',
  'text-classification': 'text',
  'token-classification': 'text',
  'question-answering': 'text',
  'text-ranking': 'text',
  image_generation: 'image',
  'text-to-image': 'image',
  'image-to-image': 'image',
  'unconditional-image-generation': 'image',
  'text-to-speech': 'audio',
  'text-to-audio': 'audio',
  'automatic-speech-recognition': 'audio',
  'audio-classification': 'audio',
  'audio-to-audio': 'audio',
};

/**
 * Kind for the Text / Image / Audio segment, from the facts the selector row
 * provides: the Pumas model type first, then the task type, then the pipeline
 * tag. Anything else (for example `vlm` or `unknown`) has no kind and only
 * appears under All.
 */
export function classifyLibraryModelKind(facts: {
  modelType: string | null;
  taskType: string | null;
  pipelineTag: string | null;
}): LibraryModelKind | null {
  const byType = facts.modelType ? KIND_BY_MODEL_TYPE[facts.modelType.toLowerCase()] : undefined;
  if (byType) return byType;
  for (const task of [facts.taskType, facts.pipelineTag]) {
    const byTask = task ? KIND_BY_TASK[task.toLowerCase()] : undefined;
    if (byTask) return byTask;
  }
  return null;
}

/**
 * Joins the Pumas selector rows with the usage projection by asset id. Rows
 * without a model id are dropped; duplicate model ids keep the first row.
 */
export function buildLibraryModelRows(
  options: readonly PortOption[],
  usage: readonly LibraryUsageProjectionRecord[],
): LibraryModelRow[] {
  const usageByAsset = new Map<string, LibraryUsageProjectionRecord>();
  for (const record of usage) {
    if (!usageByAsset.has(record.asset_id)) usageByAsset.set(record.asset_id, record);
  }
  const seen = new Set<string>();
  const rows: LibraryModelRow[] = [];
  for (const option of options) {
    const modelId = libraryModelIdFromOption(option);
    if (!modelId || seen.has(modelId)) continue;
    seen.add(modelId);
    const assetId = pumasModelAssetId(modelId);
    const modelType = metadataString(option, 'model_type');
    const taskType = metadataString(option, 'task_type_primary');
    const pipelineTag = metadataString(option, 'pipeline_tag');
    const repoId = metadataString(option, 'repo_id');
    const hints = option.metadata?.runtime_engine_hints;
    const firstHint = Array.isArray(hints) ? nonEmptyString(hints[0]) : null;
    rows.push({
      modelId,
      assetId,
      name: nonEmptyString(option.label) ?? metadataString(option, 'cleaned_name') ?? modelId,
      modelType: modelType && modelType.toLowerCase() !== 'unknown' ? modelType : null,
      kind: classifyLibraryModelKind({ modelType, taskType, pipelineTag }),
      repoId,
      backend: metadataString(option, 'recommended_backend') ?? firstHint,
      path: metadataString(option, 'display_entry_path') ?? metadataString(option, 'display_indexed_path'),
      storageKind: metadataString(option, 'storage_kind'),
      validationState: metadataString(option, 'validation_state'),
      pipelineTag,
      taskType,
      unavailableReason: option.disabled
        ? (nonEmptyString(option.unavailableReason) ?? 'The Pumas library reports this model as not ready')
        : null,
      usage: usageByAsset.get(assetId) ?? null,
      downloadUsage: repoId ? (usageByAsset.get(`${HF_MODEL_ASSET_PREFIX}${repoId}`) ?? null) : null,
    });
  }
  return rows;
}

/** Case-insensitive match on name, model id, repository and model type. */
export function filterLibraryModelRows(rows: readonly LibraryModelRow[], query: string): LibraryModelRow[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...rows];
  return rows.filter((row) =>
    [row.name, row.modelId, row.repoId, row.modelType].some(
      (field) => field !== null && field.toLowerCase().includes(needle),
    ),
  );
}

const KIND_LABELS: Record<LibraryKindFilter, string> = {
  all: 'All',
  text: 'Text',
  image: 'Image',
  audio: 'Audio',
};

/** Segment options with counts, or null when no row has a known kind. */
export function libraryKindOptions(rows: readonly LibraryModelRow[]): SegmentOption<LibraryKindFilter>[] | null {
  if (!rows.some((row) => row.kind !== null)) return null;
  return (['all', 'text', 'image', 'audio'] as const).map((value) => ({
    value,
    label: KIND_LABELS[value],
    count: value === 'all' ? rows.length : rows.filter((row) => row.kind === value).length,
  }));
}

export function filterLibraryModelRowsByKind(
  rows: readonly LibraryModelRow[],
  kind: LibraryKindFilter,
): LibraryModelRow[] {
  return kind === 'all' ? [...rows] : rows.filter((row) => row.kind === kind);
}

export type LibraryModelGroupKey = 'used_this_month' | 'unused_30_days' | 'no_usage' | 'models';

export interface LibraryModelGroup {
  key: LibraryModelGroupKey;
  label: string;
  rows: LibraryModelRow[];
}

function usedWithinMonth(row: LibraryModelRow, nowMs: number): boolean {
  return row.usage !== null && nowMs - row.usage.last_accessed_at_ms < LIBRARY_UNUSED_AFTER_MS;
}

/**
 * Groups by the only time fact the projections provide (last access). Device
 * residency is not exposed, so there is no "Loaded on a device" group. When no
 * row has usage at all, everything sits in one "Models" group. Rows keep their
 * input order inside a group; empty groups are omitted.
 */
export function groupLibraryModelRows(rows: readonly LibraryModelRow[], nowMs: number): LibraryModelGroup[] {
  if (!rows.some((row) => row.usage !== null)) {
    return rows.length > 0 ? [{ key: 'models', label: 'Models', rows: [...rows] }] : [];
  }
  const groups: LibraryModelGroup[] = [
    { key: 'used_this_month', label: 'Used this month', rows: rows.filter((row) => usedWithinMonth(row, nowMs)) },
    {
      key: 'unused_30_days',
      label: 'Unused for 30 days',
      rows: rows.filter((row) => row.usage !== null && !usedWithinMonth(row, nowMs)),
    },
    { key: 'no_usage', label: 'No usage recorded', rows: rows.filter((row) => row.usage === null) },
  ];
  return groups.filter((group) => group.rows.length > 0);
}

function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

/**
 * Page summary line. Sizes and device residency are not in the projections, so
 * they are omitted. `listUnavailable` marks a model list that could not be read,
 * so an empty list is not reported as "0 models".
 */
export function formatLibraryDimensionLine(
  rows: readonly LibraryModelRow[],
  nowMs: number,
  listUnavailable = false,
): string {
  if (listUnavailable && rows.length === 0) return 'Pumas model list unavailable';
  const parts = [plural(rows.length, 'model')];
  if (rows.some((row) => row.usage !== null)) {
    const used = rows.filter((row) => usedWithinMonth(row, nowMs)).length;
    const unused = rows.filter((row) => row.usage !== null && !usedWithinMonth(row, nowMs)).length;
    const none = rows.filter((row) => row.usage === null).length;
    parts.push(`${used} used this month`, `${unused} unused for 30 days`);
    if (none > 0) parts.push(`${none} with no usage recorded`);
  }
  return parts.join(', ');
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

function pad2(value: number): string {
  return String(value).padStart(2, '0');
}

/** "20:01" for today (local time), "Oct 29" this year, "Oct 29, 2025" otherwise. */
export function formatLibraryTimeShort(ms: number, nowMs: number): string {
  const at = new Date(ms);
  const now = new Date(nowMs);
  if (at.toDateString() === now.toDateString()) return `${pad2(at.getHours())}:${pad2(at.getMinutes())}`;
  const day = `${MONTHS[at.getMonth()]} ${at.getDate()}`;
  return at.getFullYear() === now.getFullYear() ? day : `${day}, ${at.getFullYear()}`;
}

function formatWorkflowRef(usage: LibraryUsageProjectionRecord): string | null {
  if (!usage.last_workflow_id) return null;
  return usage.last_workflow_semantic_version
    ? `${usage.last_workflow_id} v${usage.last_workflow_semantic_version}`
    : usage.last_workflow_id;
}

/** List row sub-line: backend, then who used it last, or "No usage recorded". */
export function formatLibraryModelSubline(row: LibraryModelRow): string {
  let usagePart = 'No usage recorded';
  if (row.usage) {
    usagePart = row.usage.last_workflow_id
      ? `last used by ${row.usage.last_workflow_id}`
      : `${plural(row.usage.total_access_count, 'access', 'accesses')} recorded`;
  }
  if (!row.backend) return usagePart.charAt(0).toUpperCase() + usagePart.slice(1);
  return `${row.backend}, ${usagePart.charAt(0).toLowerCase()}${usagePart.slice(1)}`;
}

export function libraryModelChips(row: LibraryModelRow, activeRunId: string | null | undefined): string[] {
  const chips: string[] = [];
  if (row.usage && isLibraryAssetLastUsedByRun(row.usage, activeRunId)) chips.push('Last used by active run');
  if (row.unavailableReason) chips.push('Not ready');
  if (row.backend) chips.push(row.backend);
  if (row.usage) chips.push(plural(row.usage.run_access_count, 'run'));
  return chips;
}

export function libraryModelLocationRows(row: LibraryModelRow): KeyValueRow[] {
  const rows: KeyValueRow[] = [];
  if (row.path) rows.push({ key: 'Path', value: row.path, mono: true });
  if (row.storageKind) rows.push({ key: 'Storage', value: row.storageKind });
  if (row.validationState) rows.push({ key: 'Validation', value: row.validationState });
  rows.push({ key: 'Readiness', value: row.unavailableReason ?? 'Ready to use in a graph' });
  return rows;
}

/** "How it is used" rows; empty when the projection has no record for the model. */
export function libraryModelUsageRows(row: LibraryModelRow, nowMs: number): KeyValueRow[] {
  const usage = row.usage;
  if (!usage) return [];
  const workflow = formatWorkflowRef(usage);
  const rows: KeyValueRow[] = [
    { key: 'Runs', value: plural(usage.run_access_count, 'run') },
    { key: 'Accesses', value: String(usage.total_access_count) },
    {
      key: 'Last used',
      value: `${formatLibraryTimeShort(usage.last_accessed_at_ms, nowMs)}${workflow ? `, by ${workflow}` : ''}`,
    },
    { key: 'Last operation', value: usage.last_operation },
    { key: 'Read from network', value: formatLibraryBytes(usage.total_network_bytes) },
  ];
  if (usage.last_cache_status) rows.push({ key: 'Cache', value: usage.last_cache_status });
  return rows;
}

export function libraryModelSourceRows(row: LibraryModelRow, nowMs: number): KeyValueRow[] {
  const rows: KeyValueRow[] = [];
  if (row.repoId) rows.push({ key: 'Repository', value: row.repoId, mono: true });
  if (row.modelType) rows.push({ key: 'Model type', value: row.modelType });
  if (row.taskType) rows.push({ key: 'Task', value: row.taskType });
  if (row.pipelineTag && row.pipelineTag !== row.taskType) rows.push({ key: 'Pipeline', value: row.pipelineTag });
  if (row.downloadUsage && row.downloadUsage.last_operation === 'download') {
    rows.push({
      key: 'Download recorded',
      value: formatLibraryTimeShort(row.downloadUsage.last_accessed_at_ms, nowMs),
    });
  }
  return rows;
}

export interface LibraryWorkflowUseRow {
  key: string;
  workflowId: string | null;
  version: string | null;
  runId: string | null;
  runCount: number;
  lastUsed: string;
  isActiveRun: boolean;
}

/**
 * "Workflows that use it": the usage projection keeps one record per asset with
 * its most recent workflow and run, so the table has at most one row.
 */
export function libraryWorkflowUseRows(
  row: LibraryModelRow,
  activeRunId: string | null | undefined,
  nowMs: number,
): LibraryWorkflowUseRow[] {
  const usage = row.usage;
  if (!usage || (!usage.last_workflow_id && !usage.last_workflow_run_id)) return [];
  return [
    {
      key: `${usage.asset_id}:${usage.last_workflow_run_id ?? usage.last_workflow_id}`,
      workflowId: usage.last_workflow_id ?? null,
      version: usage.last_workflow_semantic_version ?? null,
      runId: usage.last_workflow_run_id ?? null,
      runCount: usage.run_access_count,
      lastUsed: formatLibraryTimeShort(usage.last_accessed_at_ms, nowMs),
      isActiveRun: isLibraryAssetLastUsedByRun(usage, activeRunId),
    },
  ];
}

/** Why `Delete from disk` is blocked, or null when it is allowed. */
export function libraryDeleteBlockReason(
  row: LibraryModelRow,
  activeRunId: string | null | undefined,
  actionBusy: boolean,
): string | null {
  if (row.usage && isLibraryAssetLastUsedByRun(row.usage, activeRunId)) {
    return 'Cannot delete while the active run is the last one that used this model';
  }
  if (actionBusy) return 'Wait for the current library action to finish';
  return null;
}

export function formatPumasDeleteStatus(response: PumasModelDeleteAuditResponse): string {
  if (!response.success) return response.error ?? 'Delete did not complete';
  return response.auditEventSeq ? `Delete recorded at event ${response.auditEventSeq}` : 'Delete finished';
}

// ---------------------------------------------------------------------------
// Hugging Face search mode.
// ---------------------------------------------------------------------------

export const HF_SEARCH_DEFAULT_LIMIT = 25;
export const HF_SEARCH_MAX_LIMIT = 100;
/** Results hydrated with file sizes per search (bounded host work). */
export const HF_SEARCH_MAX_HYDRATE = 10;

/** Clamps the limit field to the host's accepted range (1 to 100); a blank field uses the default. */
export function normalizeHfSearchLimit(value: unknown): number {
  if (value === null || value === undefined || (typeof value === 'string' && value.trim() === '')) {
    return HF_SEARCH_DEFAULT_LIMIT;
  }
  const numeric = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(numeric)) return HF_SEARCH_DEFAULT_LIMIT;
  return Math.min(HF_SEARCH_MAX_LIMIT, Math.max(1, Math.trunc(numeric)));
}

/** Trims the inputs: the host rejects padded queries and empty kinds. */
export function buildHfSearchRequest(input: {
  query: string;
  kind: string;
  limit: unknown;
}): PumasHfModelSearchAuditRequest {
  const limit = normalizeHfSearchLimit(input.limit);
  const kind = input.kind.trim();
  return {
    query: input.query.trim(),
    kind: kind.length > 0 ? kind : null,
    limit,
    hydrateLimit: Math.min(limit, HF_SEARCH_MAX_HYDRATE),
  };
}

export interface HfDownloadOptions {
  officialName: string;
  family: string;
  modelType: string;
}

/** Download request with today's defaults: family "model", official name = repository id. */
export function buildHfDownloadRequest(repoId: string, options: HfDownloadOptions): PumasHfDownloadRequest {
  const repo = repoId.trim();
  const modelType = options.modelType.trim();
  return {
    repo_id: repo,
    family: options.family.trim() || 'model',
    official_name: options.officialName.trim() || repo,
    model_type: modelType.length > 0 ? modelType : null,
  };
}

export interface LibraryHfResultRow {
  repoId: string;
  sub: string;
  size: string | null;
}

function compactCount(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}k`;
  return String(count);
}

/**
 * One search result row from the Pumas `HuggingFaceModel` projection (camelCase
 * fields). Results without a repository id are dropped because they cannot be
 * downloaded.
 */
export function projectHfSearchResult(result: PumasHfModelSearchResult): LibraryHfResultRow | null {
  const repoId = nonEmptyString(result.repoId);
  if (!repoId) return null;
  const parts: string[] = [];
  const kind = nonEmptyString(result.kind);
  if (kind && kind !== 'unknown') parts.push(kind);
  if (Array.isArray(result.formats)) {
    const formats = result.formats.map(nonEmptyString).filter((format): format is string => format !== null);
    if (formats.length > 0) parts.push(formats.join(', '));
  }
  if (typeof result.downloads === 'number' && Number.isFinite(result.downloads)) {
    parts.push(`${compactCount(result.downloads)} downloads`);
  }
  const size = result.totalSizeBytes;
  return {
    repoId,
    sub: parts.join(', '),
    size: typeof size === 'number' && Number.isFinite(size) && size > 0 ? formatLibraryBytes(size) : null,
  };
}

export function projectHfSearchResults(results: readonly PumasHfModelSearchResult[]): LibraryHfResultRow[] {
  const seen = new Set<string>();
  const rows: LibraryHfResultRow[] = [];
  for (const result of results) {
    const row = projectHfSearchResult(result);
    if (!row || seen.has(row.repoId)) continue;
    seen.add(row.repoId);
    rows.push(row);
  }
  return rows;
}

export function formatHfResultCountLine(count: number, limit: number): string {
  if (count === 0) return 'No Hugging Face models match. Try a shorter or different query.';
  if (count >= limit) return `Showing the first ${count}. Narrow the search or raise the limit to see others.`;
  return plural(count, 'result');
}

export function formatHfSearchStatus(response: PumasHfModelSearchAuditResponse): string {
  return response.auditEventSeq ? `Search recorded at event ${response.auditEventSeq}` : 'Search finished';
}

export function formatHfDownloadStatus(repoId: string, response: PumasHfDownloadStartAuditResponse): string {
  return response.auditEventSeq
    ? `Download ${response.downloadId} of ${repoId} recorded at event ${response.auditEventSeq}`
    : `Download ${response.downloadId} of ${repoId} started`;
}

// ---------------------------------------------------------------------------
// Request authority for the page's async work.
// ---------------------------------------------------------------------------

export interface LibraryRequestGate {
  /** Starts an invocation; earlier tokens from this gate stop being current. */
  begin(): number;
  /** True while `token` is the newest invocation and the gate is not disposed. */
  isCurrent(token: number): boolean;
  /** Owner teardown: no token is current afterwards and `begin` returns stale tokens. */
  dispose(): void;
}

/**
 * Generation token scoped to one owner (one page instance and one kind of
 * request). A completion applies its result only while `isCurrent` holds, so a
 * superseded or post-unmount completion never writes page state.
 */
export function createLibraryRequestGate(): LibraryRequestGate {
  let generation = 0;
  let disposed = false;
  return {
    begin: () => ++generation,
    isCurrent: (token) => !disposed && token === generation,
    dispose: () => {
      disposed = true;
    },
  };
}
