import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildSelectionInputProviderQuery,
  loadLatestSelectionInputProviderOptions,
  normalizePortOptions,
  type SelectionInputProviderQuery,
} from './selectionInputProviderOptions.ts';
import type { PortDefinition, PortOptionsCommandArgs, PortOptionsResult } from '../../../services/workflow/types.ts';

function providerPort(overrides: Partial<PortDefinition> = {}): PortDefinition {
  return {
    id: 'denoising_scheduler',
    label: 'Denoising Scheduler',
    data_type: 'string',
    required: false,
    multiple: false,
    options_provider: {
      node_type: 'llm-inference',
      port_id: 'denoising_scheduler',
    },
    inference_payloads: [
      {
        task_id: 'image_generation',
        role: 'options',
      },
    ],
    ...overrides,
  };
}

function result(label: string): PortOptionsResult {
  return {
    options: [{ label, value: label.toLowerCase() }],
    totalCount: 1,
    searchable: true,
  };
}

test('buildSelectionInputProviderQuery carries stable context refs without package facts', () => {
  const query = buildSelectionInputProviderQuery(
    {
      id: 'image-node',
      data: {
        task_kind: 'image_generation',
        backend_key: 'pytorch',
        runtime_variant_id: 'pytorch.cuda',
        package_facts_summary_cursor: 'model-library-updates:7',
        pumas_model_ref: {
          model_id: 'tiny-sd',
          revision: 'main',
          selected_artifact_id: 'diffusers',
          selected_artifact_path: '/must/not/cross/frontend/context',
        },
      },
    },
    providerPort(),
  );

  assert.ok(query);
  assert.equal(query.args.nodeType, 'llm-inference');
  assert.equal(query.args.portId, 'denoising_scheduler');
  assert.deepEqual(query.args.context, {
    targetNodeId: 'image-node',
    taskKind: 'image_generation',
    selectedModelRef: 'tiny-sd@main@diffusers',
    packageFactsSummaryCursor: 'model-library-updates:7',
    backendId: 'pytorch',
    runtimeVariantId: 'pytorch.cuda',
  });
  assert.equal(JSON.stringify(query.args.context).includes('must/not/cross'), false);
});

test('normalizePortOptions keeps backend option values and presentation labels separate', () => {
  assert.deepEqual(
    normalizePortOptions([
      {
        label: 'Euler Discrete',
        value: 'euler_discrete',
        description: 'presentation only',
        metadata: { family: 'diffusers' },
      },
    ]),
    [{ label: 'Euler Discrete', value: 'euler_discrete' }],
  );
});

test('normalizePortOptions carries typed disabled state outside metadata', () => {
  assert.deepEqual(
    normalizePortOptions([
      {
        label: 'DPM++',
        value: 'dpmpp',
        disabled: true,
        unavailableState: 'requires_runtime_capability',
        unavailableReasonCode: 'scheduler_not_supported',
        unavailableReason: 'Selected runtime does not expose this scheduler',
        metadata: {
          family: 'diffusers',
        },
      },
    ]),
    [
      {
        label: 'DPM++',
        value: 'dpmpp',
        disabled: true,
        unavailableState: 'requires_runtime_capability',
        unavailableReasonCode: 'scheduler_not_supported',
        unavailableReason: 'Selected runtime does not expose this scheduler',
      },
    ],
  );
});

test('loadLatestSelectionInputProviderOptions discards stale provider responses', async () => {
  let latestKey = '';
  const pending = new Map<string, (value: PortOptionsResult) => void>();
  const loader = async (args: PortOptionsCommandArgs): Promise<PortOptionsResult> =>
    new Promise((resolve) => {
      pending.set(args.context?.runtimeVariantId ?? 'unknown', resolve);
    });

  const first = queryForRuntime('pytorch.cpu');
  const second = queryForRuntime('pytorch.cuda');
  latestKey = first.requestKey;
  const firstPromise = loadLatestSelectionInputProviderOptions(first, () => latestKey, loader);

  latestKey = second.requestKey;
  const secondPromise = loadLatestSelectionInputProviderOptions(second, () => latestKey, loader);

  pending.get('pytorch.cuda')?.(result('CUDA'));
  pending.get('pytorch.cpu')?.(result('CPU'));

  assert.deepEqual(await secondPromise, {
    status: 'applied',
    options: [{ label: 'CUDA', value: 'cuda' }],
  });
  assert.deepEqual(await firstPromise, {
    status: 'stale',
    options: [],
  });
});

function queryForRuntime(runtimeVariantId: string): SelectionInputProviderQuery {
  const query = buildSelectionInputProviderQuery(
    {
      id: 'image-node',
      data: {
        task_kind: 'image_generation',
        runtime_variant_id: runtimeVariantId,
        pumas_model_ref: 'pumas://models/tiny-sd',
      },
    },
    providerPort(),
  );
  assert.ok(query);
  return query;
}
