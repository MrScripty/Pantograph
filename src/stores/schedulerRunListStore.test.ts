import test from 'node:test';
import assert from 'node:assert/strict';

import {
  DEFAULT_SCHEDULER_RUN_FILTERS,
  DEFAULT_SCHEDULER_VIEW_PREFERENCES,
  SCHEDULER_SECONDARY_FILTER_KEYS,
  normalizeSchedulerRunFilters,
  withSchedulerRunFilters,
  withSchedulerSecondaryFiltersCleared,
  withSchedulerViewPreferences,
} from './schedulerRunListStore.ts';
import type { SchedulerRunFilters } from './schedulerRunListStore.ts';

test('normalizeSchedulerRunFilters fills defaults for missing values', () => {
  assert.deepEqual(normalizeSchedulerRunFilters(null), DEFAULT_SCHEDULER_RUN_FILTERS);
});

test('normalizeSchedulerRunFilters rejects unknown enum values', () => {
  assert.deepEqual(
    normalizeSchedulerRunFilters({
      search: 'caption',
      statusSegment: 'waiting',
      status: 'bogus',
      schedulerPolicy: '',
      retentionPolicy: 'ephemeral',
      sort: 'unknown',
    } as unknown as Partial<SchedulerRunFilters>),
    {
      search: 'caption',
      statusSegment: 'all',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'ephemeral',
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
    },
  );
});

test('withSchedulerRunFilters applies partial updates without losing existing fields', () => {
  const next = withSchedulerRunFilters(DEFAULT_SCHEDULER_RUN_FILTERS, {
    search: 'workflow-a',
    statusSegment: 'queued',
    status: 'scheduled',
    client: 'client-a',
    selectedRuntime: 'runtime-a',
    selectedRuntimeVariant: 'runtime_a.cuda',
    selectedBackend: 'llama_cpp',
    selectedDeviceClass: 'cuda',
  });

  assert.deepEqual(next, {
    search: 'workflow-a',
    statusSegment: 'queued',
    status: 'scheduled',
    schedulerPolicy: 'all',
    retentionPolicy: 'all',
    client: 'client-a',
    clientSession: 'all',
    bucket: 'all',
    selectedRuntime: 'runtime-a',
    selectedRuntimeVariant: 'runtime_a.cuda',
    selectedBackend: 'llama_cpp',
    selectedDeviceClass: 'cuda',
    selectedDevice: 'all',
    selectedNetworkNode: 'all',
    acceptedDate: 'all',
    sort: 'last_updated_desc',
  });
});

test('the Filters disclosure holds the fourteen secondary filters, not search or the segment', () => {
  assert.equal(SCHEDULER_SECONDARY_FILTER_KEYS.length, 14);
  assert.equal(new Set(SCHEDULER_SECONDARY_FILTER_KEYS).size, 14);
  assert.ok(!SCHEDULER_SECONDARY_FILTER_KEYS.includes('search' as never));
  assert.ok(!SCHEDULER_SECONDARY_FILTER_KEYS.includes('statusSegment' as never));
});

test('withSchedulerSecondaryFiltersCleared resets disclosure filters and keeps search and segment', () => {
  const applied = withSchedulerRunFilters(DEFAULT_SCHEDULER_RUN_FILTERS, {
    search: 'caption',
    statusSegment: 'failed',
    status: 'failed',
    sort: 'duration_desc',
    selectedDevice: 'cuda:0',
    acceptedDate: '2026-11-03',
  });

  assert.deepEqual(withSchedulerSecondaryFiltersCleared(applied), {
    ...DEFAULT_SCHEDULER_RUN_FILTERS,
    search: 'caption',
    statusSegment: 'failed',
  });
});

test('withSchedulerViewPreferences keeps the lanes preference boolean', () => {
  assert.deepEqual(DEFAULT_SCHEDULER_VIEW_PREFERENCES, { showLanes: true });
  assert.deepEqual(withSchedulerViewPreferences(DEFAULT_SCHEDULER_VIEW_PREFERENCES, { showLanes: false }), {
    showLanes: false,
  });
  assert.deepEqual(
    withSchedulerViewPreferences({ showLanes: false }, { showLanes: 'yes' } as unknown as { showLanes: boolean }),
    { showLanes: false },
  );
});
