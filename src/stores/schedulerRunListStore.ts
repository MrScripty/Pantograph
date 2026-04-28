import { writable, type Readable } from 'svelte/store';
import type { RunListProjectionStatus } from '../services/diagnostics/types';

export type SchedulerStatusFilter = 'all' | RunListProjectionStatus;

export type SchedulerSortKey =
  | 'last_updated_desc'
  | 'queued_desc'
  | 'duration_desc'
  | 'workflow_asc'
  | 'status_asc';

export interface SchedulerRunFilters {
  search: string;
  status: SchedulerStatusFilter;
  schedulerPolicy: string;
  retentionPolicy: string;
  client: string;
  clientSession: string;
  bucket: string;
  acceptedDate: string;
  sort: SchedulerSortKey;
}

export const SCHEDULER_RUN_COLUMN_IDS = [
  'status',
  'workflow',
  'run_id',
  'accepted',
  'queued',
  'started',
  'completed',
  'duration',
  'queue_position',
  'priority',
  'estimate',
  'scheduler_reason',
  'policy',
  'retention',
  'updated',
] as const;

export type SchedulerRunColumnId = (typeof SCHEDULER_RUN_COLUMN_IDS)[number];

export interface SchedulerRunColumnState {
  visibleColumns: SchedulerRunColumnId[];
}

export const SCHEDULER_STATUS_FILTERS: SchedulerStatusFilter[] = [
  'all',
  'accepted',
  'future',
  'scheduled',
  'queued',
  'delayed',
  'running',
  'completed',
  'failed',
  'cancelled',
];

export const SCHEDULER_SORT_OPTIONS: { label: string; value: SchedulerSortKey }[] = [
  { label: 'Updated', value: 'last_updated_desc' },
  { label: 'Queued', value: 'queued_desc' },
  { label: 'Duration', value: 'duration_desc' },
  { label: 'Workflow', value: 'workflow_asc' },
  { label: 'Status', value: 'status_asc' },
];

export const DEFAULT_SCHEDULER_RUN_FILTERS: SchedulerRunFilters = {
  search: '',
  status: 'all',
  schedulerPolicy: 'all',
  retentionPolicy: 'all',
  client: 'all',
  clientSession: 'all',
  bucket: 'all',
  acceptedDate: 'all',
  sort: 'last_updated_desc',
};

export const DEFAULT_SCHEDULER_RUN_COLUMN_STATE: SchedulerRunColumnState = {
  visibleColumns: [...SCHEDULER_RUN_COLUMN_IDS],
};

function isSchedulerStatusFilter(value: unknown): value is SchedulerStatusFilter {
  return (
    typeof value === 'string' &&
    SCHEDULER_STATUS_FILTERS.includes(value as SchedulerStatusFilter)
  );
}

function isSchedulerSortKey(value: unknown): value is SchedulerSortKey {
  return (
    typeof value === 'string' &&
    SCHEDULER_SORT_OPTIONS.some((option) => option.value === value)
  );
}

function isSchedulerRunColumnId(value: unknown): value is SchedulerRunColumnId {
  return (
    typeof value === 'string' &&
    SCHEDULER_RUN_COLUMN_IDS.includes(value as SchedulerRunColumnId)
  );
}

function normalizeOption(value: unknown): string {
  return typeof value === 'string' && value.trim().length > 0 ? value : 'all';
}

export function normalizeSchedulerRunFilters(
  filters: Partial<SchedulerRunFilters> | null | undefined,
): SchedulerRunFilters {
  return {
    search: typeof filters?.search === 'string' ? filters.search : '',
    status: isSchedulerStatusFilter(filters?.status) ? filters.status : 'all',
    schedulerPolicy: normalizeOption(filters?.schedulerPolicy),
    retentionPolicy: normalizeOption(filters?.retentionPolicy),
    client: normalizeOption(filters?.client),
    clientSession: normalizeOption(filters?.clientSession),
    bucket: normalizeOption(filters?.bucket),
    acceptedDate: normalizeOption(filters?.acceptedDate),
    sort: isSchedulerSortKey(filters?.sort) ? filters.sort : 'last_updated_desc',
  };
}

export function normalizeSchedulerRunColumnState(
  state: Partial<SchedulerRunColumnState> | null | undefined,
): SchedulerRunColumnState {
  const columns = state?.visibleColumns?.filter(isSchedulerRunColumnId) ?? [];
  return {
    visibleColumns: columns.length > 0 ? [...new Set(columns)] : [...SCHEDULER_RUN_COLUMN_IDS],
  };
}

export function withSchedulerRunFilters(
  state: SchedulerRunFilters,
  patch: Partial<SchedulerRunFilters>,
): SchedulerRunFilters {
  return normalizeSchedulerRunFilters({
    ...state,
    ...patch,
  });
}

export function withSchedulerRunColumnState(
  state: SchedulerRunColumnState,
  patch: Partial<SchedulerRunColumnState>,
): SchedulerRunColumnState {
  return normalizeSchedulerRunColumnState({
    ...state,
    ...patch,
  });
}

const schedulerRunFiltersStore = writable<SchedulerRunFilters>({
  ...DEFAULT_SCHEDULER_RUN_FILTERS,
});

const schedulerRunColumnStateStore = writable<SchedulerRunColumnState>({
  visibleColumns: [...DEFAULT_SCHEDULER_RUN_COLUMN_STATE.visibleColumns],
});

export const schedulerRunFilters: Readable<SchedulerRunFilters> = {
  subscribe: schedulerRunFiltersStore.subscribe,
};

export const schedulerRunColumnState: Readable<SchedulerRunColumnState> = {
  subscribe: schedulerRunColumnStateStore.subscribe,
};

export function setSchedulerRunFilters(patch: Partial<SchedulerRunFilters>): void {
  schedulerRunFiltersStore.update((state) => withSchedulerRunFilters(state, patch));
}

export function setSchedulerRunColumnState(patch: Partial<SchedulerRunColumnState>): void {
  schedulerRunColumnStateStore.update((state) => withSchedulerRunColumnState(state, patch));
}

export function resetSchedulerRunFilters(): void {
  schedulerRunFiltersStore.set({ ...DEFAULT_SCHEDULER_RUN_FILTERS });
}

export function resetSchedulerRunColumnState(): void {
  schedulerRunColumnStateStore.set({
    visibleColumns: [...DEFAULT_SCHEDULER_RUN_COLUMN_STATE.visibleColumns],
  });
}
