import test from 'node:test';
import assert from 'node:assert/strict';

import {
  DEFAULT_SCHEDULER_RUN_COLUMN_STATE,
  DEFAULT_SCHEDULER_RUN_FILTERS,
  normalizeSchedulerRunColumnState,
  normalizeSchedulerRunFilters,
  withSchedulerRunColumnState,
  withSchedulerRunFilters,
} from './schedulerRunListStore.ts';
import type {
  SchedulerRunColumnState,
  SchedulerRunFilters,
} from './schedulerRunListStore.ts';

test('normalizeSchedulerRunFilters fills defaults for missing values', () => {
  assert.deepEqual(normalizeSchedulerRunFilters(null), DEFAULT_SCHEDULER_RUN_FILTERS);
});

test('normalizeSchedulerRunFilters rejects unknown enum values', () => {
  assert.deepEqual(
    normalizeSchedulerRunFilters({
      search: 'caption',
      status: 'bogus',
      schedulerPolicy: '',
      retentionPolicy: 'ephemeral',
      sort: 'unknown',
    } as unknown as Partial<SchedulerRunFilters>),
    {
      search: 'caption',
      status: 'all',
      schedulerPolicy: 'all',
      retentionPolicy: 'ephemeral',
      client: 'all',
      clientSession: 'all',
      bucket: 'all',
      selectedRuntime: 'all',
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
    status: 'scheduled',
    client: 'client-a',
    selectedRuntime: 'runtime-a',
  });

  assert.deepEqual(next, {
    search: 'workflow-a',
    status: 'scheduled',
    schedulerPolicy: 'all',
    retentionPolicy: 'all',
    client: 'client-a',
    clientSession: 'all',
    bucket: 'all',
    selectedRuntime: 'runtime-a',
    selectedDevice: 'all',
    selectedNetworkNode: 'all',
    acceptedDate: 'all',
    sort: 'last_updated_desc',
  });
});

test('normalizeSchedulerRunColumnState defaults and rejects unknown columns', () => {
  assert.deepEqual(
    normalizeSchedulerRunColumnState({
      visibleColumns: ['status', 'bogus', 'workflow', 'status'],
    } as unknown as Partial<SchedulerRunColumnState>),
    {
      visibleColumns: ['status', 'workflow'],
    },
  );

  assert.deepEqual(
    normalizeSchedulerRunColumnState({ visibleColumns: [] }),
    DEFAULT_SCHEDULER_RUN_COLUMN_STATE,
  );
});

test('withSchedulerRunColumnState applies visibility updates', () => {
  assert.deepEqual(
    withSchedulerRunColumnState(DEFAULT_SCHEDULER_RUN_COLUMN_STATE, {
      visibleColumns: ['status', 'duration'],
    }),
    {
      visibleColumns: ['status', 'duration'],
    },
  );
});
