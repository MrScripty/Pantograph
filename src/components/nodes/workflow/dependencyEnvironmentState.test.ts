import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildDependencyEnvironmentActionPayload,
  dependencyCodeLabel,
  dependencyBadgeFor,
  formatDependencyActivityLine,
  getPatchFrom,
  hasOverrideFields,
  isPatchTarget,
  matchesDependencyActivityEvent,
  mergeOverridePatches,
  parseOverridePatches,
  renderDependencyActivityEvent,
  upsertExtraIndexUrls,
  upsertStringOverrideField,
} from './dependencyEnvironmentState.ts';

test('parseOverridePatches accepts valid JSON patch arrays and drops invalid entries', () => {
  const patches = parseOverridePatches(
    JSON.stringify([
      {
        contract_version: 1,
        binding_id: 'binding-a',
        scope: 'requirement',
        requirement_name: 'torch',
        fields: {
          index_url: 'https://packages.example/simple',
          extra_index_urls: [' https://extra.example/simple ', ''],
        },
        source: 'user',
      },
      {
        binding_id: '',
        scope: 'binding',
        fields: {},
      },
    ]),
  );

  assert.equal(patches.length, 1);
  assert.equal(patches[0].binding_id, 'binding-a');
  assert.equal(patches[0].requirement_name, 'torch');
  assert.deepEqual(patches[0].fields.extra_index_urls, ['https://extra.example/simple']);
});

test('mergeOverridePatches lets local overlays replace connected patches by target', () => {
  const merged = mergeOverridePatches(
    [
      {
        contract_version: 1,
        binding_id: 'binding-a',
        scope: 'requirement',
        requirement_name: 'torch',
        fields: { index_url: 'https://upstream.example/simple' },
      },
    ],
    [
      {
        contract_version: 1,
        binding_id: 'binding-a',
        scope: 'requirement',
        requirement_name: 'torch',
        fields: { index_url: 'https://local.example/simple' },
      },
    ],
  );

  assert.equal(merged.length, 1);
  assert.equal(merged[0].fields.index_url, 'https://local.example/simple');
});

test('patch lookup and field checks classify override targets', () => {
  const patch = {
    contract_version: 1,
    binding_id: 'binding-a',
    scope: 'requirement' as const,
    requirement_name: 'Torch',
    fields: { wheel_source_path: '/wheels' },
  };

  assert.equal(isPatchTarget(patch, 'binding-a', 'requirement', 'torch'), true);
  assert.equal(isPatchTarget(patch, 'binding-a', 'binding'), false);
  assert.equal(hasOverrideFields(patch.fields), true);
  assert.equal(getPatchFrom([patch], 'binding-a', 'requirement', 'torch'), patch);
});

test('dependencyCodeLabel maps known backend codes to readable labels', () => {
  assert.equal(dependencyCodeLabel('dependency_install_failed'), 'dependency check failed');
  assert.equal(dependencyCodeLabel('unknown_profile'), 'unknown profile');
  assert.equal(dependencyCodeLabel('custom_backend_code'), 'custom backend code');
});

test('dependencyBadgeFor derives labels from status and validation state', () => {
  assert.equal(
    dependencyBadgeFor(null, {
      state: 'ready',
      requirements: {
        model_id: 'model-a',
        platform_key: 'linux',
        dependency_contract_version: 1,
        validation_state: 'resolved',
        validation_errors: [],
        bindings: [],
        selected_binding_ids: [],
      },
      bindings: [],
    }).label,
    'deps ready',
  );

  assert.equal(
    dependencyBadgeFor(
      {
        model_id: 'model-a',
        platform_key: 'linux',
        dependency_contract_version: 1,
        validation_state: 'unknown_profile',
        validation_errors: [],
        bindings: [],
        selected_binding_ids: [],
      },
      null,
    ).label,
    'requirements unresolved',
  );
});

test('dependency activity helpers filter and render matching backend events', () => {
  const event = {
    timestamp: '2026-04-22T00:00:00Z',
    node_type: 'dependency-environment',
    model_path: '/models/model.gguf',
    phase: 'install',
    message: 'Installing torch',
    binding_id: 'binding-a',
    requirement_name: 'torch',
  };

  assert.equal(matchesDependencyActivityEvent(event, '/models/model.gguf'), true);
  assert.equal(matchesDependencyActivityEvent(event, '/models/other.gguf'), false);
  assert.equal(renderDependencyActivityEvent(event), 'install | binding-a | torch: Installing torch');
  assert.equal(formatDependencyActivityLine(' done ', '12:00:00'), '[12:00:00] done');
  assert.equal(formatDependencyActivityLine(' ', '12:00:00'), null);
});

test('upsertStringOverrideField adds updates and removes empty patches', () => {
  const withValue = upsertStringOverrideField(
    [],
    'binding-a',
    'binding',
    undefined,
    'python_executable',
    ' /usr/bin/python3 ',
    '2026-04-22T00:00:00.000Z',
  );

  assert.equal(withValue.length, 1);
  assert.equal(withValue[0].fields.python_executable, '/usr/bin/python3');
  assert.equal(withValue[0].source, 'user');

  const cleared = upsertStringOverrideField(
    withValue,
    'binding-a',
    'binding',
    undefined,
    'python_executable',
    '',
    '2026-04-22T00:00:01.000Z',
  );

  assert.deepEqual(cleared, []);
});

test('upsertExtraIndexUrls dedupes comma-separated URLs', () => {
  const patches = upsertExtraIndexUrls(
    [],
    'binding-a',
    'torch',
    'https://a.example/simple, https://a.example/simple, https://b.example/simple',
    '2026-04-22T00:00:00.000Z',
  );

  assert.deepEqual(patches[0].fields.extra_index_urls, [
    'https://a.example/simple',
    'https://b.example/simple',
  ]);
});

test('buildDependencyEnvironmentActionPayload projects upstream model and override state', () => {
  const payload = buildDependencyEnvironmentActionPayload({
    action: 'run',
    mode: 'manual',
    upstreamModelPath: ' /models/model.gguf ',
    upstreamModelId: 'model-a',
    upstreamModelType: 'embedding',
    upstreamTaskType: 'embed',
    upstreamBackendKey: 'llama_cpp',
    upstreamPlatformContext: { os: 'linux' },
    selectedBindingIds: ['binding-a'],
    upstreamRequirements: null,
    dependencyRequirements: {
      model_id: 'model-a',
      platform_key: 'linux-x86_64',
      backend_key: 'llama_cpp',
      dependency_contract_version: 1,
      validation_state: 'resolved',
      validation_errors: [],
      bindings: [],
      selected_binding_ids: [],
    },
    effectiveManualOverrides: [
      {
        contract_version: 1,
        binding_id: 'binding-a',
        scope: 'binding',
        fields: { python_executable: '/usr/bin/python3' },
      },
    ],
  });

  assert.equal(payload?.modelPath, '/models/model.gguf');
  assert.equal(payload?.modelId, 'model-a');
  assert.deepEqual(payload?.selectedBindingIds, ['binding-a']);
  assert.equal(payload?.dependencyOverridePatches?.length, 1);
});
