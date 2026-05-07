import test from 'node:test';
import assert from 'node:assert/strict';
import {
  extractPumasSelectorCursor,
  extractPumasSelectorCursorFromResult,
  invalidatePumasModelOptionsCache,
  loadPumasModelOptions,
  selectorUpdateFeedRequiresRefresh,
  type ModelLibraryUpdateFeed,
  type WorkflowInvoker,
} from './pumaModelOptionsCache.ts';
import type { PortOption, PortOptionsResult } from './types.ts';

function option(modelId: string, cursor: string): PortOption {
  return {
    value: modelId,
    label: modelId,
    metadata: {
      id: modelId,
      package_facts_summary_cursor: cursor,
    },
  };
}

function portOptions(
  options: PortOption[],
  metadata?: Record<string, unknown>,
): PortOptionsResult {
  return {
    options,
    totalCount: options.length,
    searchable: true,
    metadata,
  };
}

function updateFeed(overrides: Partial<ModelLibraryUpdateFeed> = {}): ModelLibraryUpdateFeed {
  return {
    cursor: 'model-library-updates:1',
    events: [],
    stale_cursor: false,
    snapshot_required: false,
    ...overrides,
  };
}

test.afterEach(() => {
  invalidatePumasModelOptionsCache();
});

test('extractPumasSelectorCursorFromResult reads result metadata before row metadata', () => {
  assert.equal(
    extractPumasSelectorCursorFromResult(
      portOptions(
        [option('llm/imported/ready', 'model-library-updates:1')],
        { package_facts_summary_cursor: 'model-library-updates:2' },
      ),
    ),
    'model-library-updates:2',
  );
  assert.equal(
    extractPumasSelectorCursorFromResult(
      portOptions([], { package_facts_summary_cursor: 'model-library-updates:3' }),
    ),
    'model-library-updates:3',
  );
});

test('extractPumasSelectorCursor reads the first selector cursor from option metadata', () => {
  assert.equal(
    extractPumasSelectorCursor([
      { value: 'empty', label: 'empty', metadata: {} },
      option('llm/imported/test', 'model-library-updates:12'),
    ]),
    'model-library-updates:12',
  );
});

test('selectorUpdateFeedRequiresRefresh treats events and stale cursors as cache invalidation', () => {
  assert.equal(selectorUpdateFeedRequiresRefresh(updateFeed()), false);
  assert.equal(
    selectorUpdateFeedRequiresRefresh(updateFeed({ stale_cursor: true })),
    true,
  );
  assert.equal(
    selectorUpdateFeedRequiresRefresh(updateFeed({ snapshot_required: true })),
    true,
  );
  assert.equal(
    selectorUpdateFeedRequiresRefresh(updateFeed({
      events: [{
        cursor: 'model-library-updates:2',
        model_id: 'llm/imported/test',
        change_kind: 'package_facts_modified',
        fact_family: 'package_facts',
        refresh_scope: 'summary_and_detail',
      }],
    })),
    true,
  );
});

test('loadPumasModelOptions caches a snapshot after empty update handoff', async () => {
  const calls: string[] = [];
  const invoker: WorkflowInvoker = async (command) => {
    calls.push(command);
    if (command === 'query_port_options') {
      return portOptions([option('llm/imported/ready', 'model-library-updates:1')]) as never;
    }
    if (command === 'list_model_library_updates_since') {
      return updateFeed({ cursor: 'model-library-updates:1' }) as never;
    }
    throw new Error(`unexpected command ${command}`);
  };

  const first = await loadPumasModelOptions(invoker);
  const second = await loadPumasModelOptions(invoker);

  assert.equal(first, second);
  assert.deepEqual(
    calls,
    ['query_port_options', 'list_model_library_updates_since'],
  );
});

test('loadPumasModelOptions reloads once when handoff reports model updates', async () => {
  const snapshots = [
    portOptions([option('llm/imported/stale', 'model-library-updates:1')]),
    portOptions([option('llm/imported/fresh', 'model-library-updates:2')]),
  ];
  const calls: string[] = [];
  const invoker: WorkflowInvoker = async (command) => {
    calls.push(command);
    if (command === 'query_port_options') {
      const snapshot = snapshots.shift();
      assert.ok(snapshot);
      return snapshot as never;
    }
    if (command === 'list_model_library_updates_since') {
      return updateFeed({
        cursor: 'model-library-updates:2',
        events: [{
          cursor: 'model-library-updates:2',
          model_id: 'llm/imported/stale',
          change_kind: 'model_modified',
          fact_family: 'model_record',
          refresh_scope: 'summary',
        }],
      }) as never;
    }
    throw new Error(`unexpected command ${command}`);
  };

  const options = await loadPumasModelOptions(invoker);

  assert.deepEqual(options.map((item) => item.value), ['llm/imported/fresh']);
  assert.deepEqual(
    calls,
    ['query_port_options', 'list_model_library_updates_since', 'query_port_options'],
  );
});

test('loadPumasModelOptions keeps read-only snapshots when update feed is unavailable', async () => {
  const calls: string[] = [];
  const invoker: WorkflowInvoker = async (command) => {
    calls.push(command);
    if (command === 'query_port_options') {
      return portOptions([option('llm/imported/read-only', 'model-library-updates:1')]) as never;
    }
    if (command === 'list_model_library_updates_since') {
      throw new Error('Pumas owner API unavailable');
    }
    throw new Error(`unexpected command ${command}`);
  };

  const options = await loadPumasModelOptions(invoker);

  assert.deepEqual(options.map((item) => item.value), ['llm/imported/read-only']);
  assert.deepEqual(
    calls,
    ['query_port_options', 'list_model_library_updates_since'],
  );
});

test('loadPumasModelOptions performs update handoff for empty selector snapshots', async () => {
  const calls: string[] = [];
  const invoker: WorkflowInvoker = async (command) => {
    calls.push(command);
    if (command === 'query_port_options') {
      return portOptions([], { package_facts_summary_cursor: 'model-library-updates:1' }) as never;
    }
    if (command === 'list_model_library_updates_since') {
      return updateFeed({ cursor: 'model-library-updates:1' }) as never;
    }
    throw new Error(`unexpected command ${command}`);
  };

  const options = await loadPumasModelOptions(invoker);

  assert.deepEqual(options, []);
  assert.deepEqual(
    calls,
    ['query_port_options', 'list_model_library_updates_since'],
  );
});
