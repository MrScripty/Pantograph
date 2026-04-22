import test from 'node:test';
import assert from 'node:assert/strict';

import {
  dependencyCodeLabel,
  getPatchFrom,
  hasOverrideFields,
  isPatchTarget,
  mergeOverridePatches,
  parseOverridePatches,
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
