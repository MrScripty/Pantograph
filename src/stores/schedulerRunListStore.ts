/**
 * Scheduler page filter state and view preferences.
 *
 * The run rows themselves are not stored here: the Scheduler reads the one
 * run-list projection owned by the shell (`runContextRunList` in
 * `src/components/workbench/shell/runContextStore.ts`, issue WR-005) and
 * applies these filters to it in `schedulerPagePresenters.ts`.
 */
import { writable, type Readable } from 'svelte/store';
import type { RunListProjectionStatus } from '../services/diagnostics/types';

export type SchedulerStatusFilter = 'all' | RunListProjectionStatus;

/** The coarse status segment above the run list (All, Active, Queued, Done, Failed). */
export type SchedulerStatusSegment = 'all' | 'active' | 'queued' | 'done' | 'failed';

export const SCHEDULER_STATUS_SEGMENTS: SchedulerStatusSegment[] = ['all', 'active', 'queued', 'done', 'failed'];

export type SchedulerSortKey =
  | 'last_updated_desc'
  | 'queued_desc'
  | 'duration_desc'
  | 'workflow_asc'
  | 'status_asc';

export interface SchedulerRunFilters {
  search: string;
  statusSegment: SchedulerStatusSegment;
  status: SchedulerStatusFilter;
  schedulerPolicy: string;
  retentionPolicy: string;
  client: string;
  clientSession: string;
  bucket: string;
  selectedRuntime: string;
  selectedRuntimeVariant: string;
  selectedBackend: string;
  selectedDeviceClass: string;
  selectedDevice: string;
  selectedNetworkNode: string;
  acceptedDate: string;
  sort: SchedulerSortKey;
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

/**
 * The filters that live in the Filters disclosure (every field except the
 * search text and the status segment). Applied ones are shown as chips.
 */
export type SchedulerSecondaryFilterKey = Exclude<keyof SchedulerRunFilters, 'search' | 'statusSegment'>;

export const SCHEDULER_SECONDARY_FILTER_KEYS: SchedulerSecondaryFilterKey[] = [
  'status',
  'sort',
  'schedulerPolicy',
  'retentionPolicy',
  'client',
  'clientSession',
  'bucket',
  'selectedRuntime',
  'selectedRuntimeVariant',
  'selectedBackend',
  'selectedDeviceClass',
  'selectedDevice',
  'selectedNetworkNode',
  'acceptedDate',
];

export const DEFAULT_SCHEDULER_RUN_FILTERS: SchedulerRunFilters = {
  search: '',
  statusSegment: 'all',
  status: 'all',
  schedulerPolicy: 'all',
  retentionPolicy: 'all',
  client: 'all',
  clientSession: 'all',
  bucket: 'all',
  selectedRuntime: 'all',
  selectedRuntimeVariant: 'all',
  selectedBackend: 'all',
  selectedDeviceClass: 'all',
  selectedDevice: 'all',
  selectedNetworkNode: 'all',
  acceptedDate: 'all',
  sort: 'last_updated_desc',
};

export interface SchedulerViewPreferences {
  /**
   * Show the device memory strip under the dimension line. The strip renders
   * a sentence instead of lanes until the local network status projection
   * reports per-device memory (issue WR-006).
   */
  showLanes: boolean;
}

export const DEFAULT_SCHEDULER_VIEW_PREFERENCES: SchedulerViewPreferences = {
  showLanes: true,
};

function isSchedulerStatusSegment(value: unknown): value is SchedulerStatusSegment {
  return (
    typeof value === 'string' &&
    SCHEDULER_STATUS_SEGMENTS.includes(value as SchedulerStatusSegment)
  );
}

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

function normalizeOption(value: unknown): string {
  return typeof value === 'string' && value.trim().length > 0 ? value : 'all';
}

export function normalizeSchedulerRunFilters(
  filters: Partial<SchedulerRunFilters> | null | undefined,
): SchedulerRunFilters {
  return {
    search: typeof filters?.search === 'string' ? filters.search : '',
    statusSegment: isSchedulerStatusSegment(filters?.statusSegment) ? filters.statusSegment : 'all',
    status: isSchedulerStatusFilter(filters?.status) ? filters.status : 'all',
    schedulerPolicy: normalizeOption(filters?.schedulerPolicy),
    retentionPolicy: normalizeOption(filters?.retentionPolicy),
    client: normalizeOption(filters?.client),
    clientSession: normalizeOption(filters?.clientSession),
    bucket: normalizeOption(filters?.bucket),
    selectedRuntime: normalizeOption(filters?.selectedRuntime),
    selectedRuntimeVariant: normalizeOption(filters?.selectedRuntimeVariant),
    selectedBackend: normalizeOption(filters?.selectedBackend),
    selectedDeviceClass: normalizeOption(filters?.selectedDeviceClass),
    selectedDevice: normalizeOption(filters?.selectedDevice),
    selectedNetworkNode: normalizeOption(filters?.selectedNetworkNode),
    acceptedDate: normalizeOption(filters?.acceptedDate),
    sort: isSchedulerSortKey(filters?.sort) ? filters.sort : 'last_updated_desc',
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

/** Reset every Filters-disclosure field; search and the status segment stay. */
export function withSchedulerSecondaryFiltersCleared(state: SchedulerRunFilters): SchedulerRunFilters {
  const defaults = Object.fromEntries(
    SCHEDULER_SECONDARY_FILTER_KEYS.map((key) => [key, DEFAULT_SCHEDULER_RUN_FILTERS[key]]),
  ) as Partial<SchedulerRunFilters>;
  return withSchedulerRunFilters(state, defaults);
}

export function withSchedulerViewPreferences(
  state: SchedulerViewPreferences,
  patch: Partial<SchedulerViewPreferences>,
): SchedulerViewPreferences {
  return {
    showLanes: typeof patch.showLanes === 'boolean' ? patch.showLanes : state.showLanes,
  };
}

const schedulerRunFiltersStore = writable<SchedulerRunFilters>({
  ...DEFAULT_SCHEDULER_RUN_FILTERS,
});

const schedulerViewPreferencesStore = writable<SchedulerViewPreferences>({
  ...DEFAULT_SCHEDULER_VIEW_PREFERENCES,
});

export const schedulerRunFilters: Readable<SchedulerRunFilters> = {
  subscribe: schedulerRunFiltersStore.subscribe,
};

export const schedulerViewPreferences: Readable<SchedulerViewPreferences> = {
  subscribe: schedulerViewPreferencesStore.subscribe,
};

export function setSchedulerRunFilters(patch: Partial<SchedulerRunFilters>): void {
  schedulerRunFiltersStore.update((state) => withSchedulerRunFilters(state, patch));
}

export function clearSchedulerSecondaryFilters(): void {
  schedulerRunFiltersStore.update(withSchedulerSecondaryFiltersCleared);
}

export function resetSchedulerRunFilters(): void {
  schedulerRunFiltersStore.set({ ...DEFAULT_SCHEDULER_RUN_FILTERS });
}

export function setSchedulerViewPreferences(patch: Partial<SchedulerViewPreferences>): void {
  schedulerViewPreferencesStore.update((state) => withSchedulerViewPreferences(state, patch));
}
