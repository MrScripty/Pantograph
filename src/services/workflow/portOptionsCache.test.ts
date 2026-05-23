import test from 'node:test';
import assert from 'node:assert/strict';
import {
  invalidatePortOptionsCache,
  loadPortOptions,
  portOptionsCacheKey,
  type PortOptionsInvoker,
} from './portOptionsCache.ts';
import type { PortOptionsCommandArgs, PortOptionsResult } from './types.ts';

function result(label: string): PortOptionsResult {
  return {
    options: [{ value: label, label }],
    totalCount: 1,
    searchable: true,
  };
}

test.afterEach(() => {
  invalidatePortOptionsCache();
});

test('portOptionsCacheKey includes provider context refs in stable order', () => {
  const left: PortOptionsCommandArgs = {
    nodeType: 'llm-inference',
    portId: 'denoising_scheduler',
    context: {
      selectedModelRef: 'pumas://models/diffusion/tiny',
      packageFactsSummaryCursor: 'model-library-updates:2',
      requestedRuntimeId: 'pytorch',
    },
  };
  const right: PortOptionsCommandArgs = {
    nodeType: 'llm-inference',
    portId: 'denoising_scheduler',
    context: {
      requestedRuntimeId: 'pytorch',
      packageFactsSummaryCursor: 'model-library-updates:2',
      selectedModelRef: 'pumas://models/diffusion/tiny',
    },
  };

  assert.equal(portOptionsCacheKey(left), portOptionsCacheKey(right));
  assert.notEqual(
    portOptionsCacheKey(left),
    portOptionsCacheKey({
      ...left,
      context: {
        ...left.context,
        packageFactsSummaryCursor: 'model-library-updates:3',
      },
    }),
  );
});

test('loadPortOptions caches rows by node port and provider context', async () => {
  const calls: PortOptionsCommandArgs[] = [];
  const invoker: PortOptionsInvoker = async (_command, args) => {
    calls.push(args as PortOptionsCommandArgs);
    return result(`call-${calls.length}`) as never;
  };

  const base: PortOptionsCommandArgs = {
    nodeType: 'llm-inference',
    portId: 'denoising_scheduler',
    context: {
      targetNodeId: 'node-a',
      selectedModelRef: 'pumas://models/diffusion/tiny',
      packageFactsSummaryCursor: 'model-library-updates:1',
      requestedRuntimeId: 'pytorch',
      requestedDeviceId: 'cuda:0',
    },
  };

  const first = await loadPortOptions(base, {}, invoker);
  const second = await loadPortOptions({ ...base }, {}, invoker);
  const third = await loadPortOptions(
    {
      ...base,
      context: {
        ...base.context,
        requestedDeviceId: 'cpu',
      },
    },
    {},
    invoker,
  );

  assert.equal(first, second);
  assert.notEqual(first, third);
  assert.equal(calls.length, 2);
  assert.deepEqual(calls[0], base);
});

test('loadPortOptions shares inflight requests and supports force refresh', async () => {
  let resolveRequest: ((value: PortOptionsResult) => void) | null = null;
  let calls = 0;
  const invoker: PortOptionsInvoker = async () => {
    calls += 1;
    return new Promise<PortOptionsResult>((resolve) => {
      resolveRequest = resolve;
    }) as never;
  };
  const args: PortOptionsCommandArgs = {
    nodeType: 'llm-inference',
    portId: 'denoising_scheduler',
    context: {
      selectedModelRef: 'pumas://models/diffusion/tiny',
      packageFactsSummaryCursor: 'model-library-updates:1',
    },
  };

  const firstPromise = loadPortOptions(args, {}, invoker);
  const secondPromise = loadPortOptions(args, {}, invoker);
  assert.equal(calls, 1);

  assert.ok(resolveRequest, 'inflight request should expose a resolver');
  (resolveRequest as (value: PortOptionsResult) => void)(result('resolved'));
  const first = await firstPromise;
  const second = await secondPromise;
  assert.equal(first, second);

  const refreshed = await loadPortOptions(args, { forceRefresh: true }, async () => {
    calls += 1;
    return result('refreshed') as never;
  });
  assert.equal(calls, 2);
  assert.equal(refreshed.options[0]?.label, 'refreshed');
});
