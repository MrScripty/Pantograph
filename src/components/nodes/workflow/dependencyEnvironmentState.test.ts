import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildDependencyEnvironmentActionPayload,
  adoptDependencyEnvironmentUpstreamRequirements,
  appendDependencyActivityLogLine,
  applyDependencyEnvironmentActionNodeData,
  buildDependencyEnvironmentNodeData,
  clearDependencyBindingOverrides,
  clearDependencyRequirementOverrides,
  countDependencyBindingPatches,
  countDependencyRequirementPatches,
  createDependencyEnvironmentNodeDataState,
  dependencyCodeLabel,
  dependencyBadgeFor,
  filterDependencyEnvironmentBindings,
  formatDependencyEnvironmentActionError,
  formatDependencyEnvironmentListenerError,
  formatDependencyActivityLine,
  formatDependencyActivityTimestamp,
  formatDependencyOverrideUpdatedAt,
  getPatchFrom,
  hasDependencyBindingOverrideFields,
  hasDependencyRequirementOverrideFields,
  hasOverrideFields,
  isDependencyEnvironmentBindingSelected,
  isPatchTarget,
  matchesDependencyActivityEvent,
  mergeOverridePatches,
  parseOverridePatches,
  readDependencyExtraIndexUrls,
  readDependencyOverrideInputValue,
  readDependencyStringOverrideField,
  renderDependencyActivityEvent,
  resolveDependencyEnvironmentUpstreamState,
  runDependencyEnvironmentActionRequest,
  setupDependencyEnvironmentActivityListener,
  toggleDependencyEnvironmentAllBindings,
  toggleDependencyEnvironmentBindingSelection,
  upsertExtraIndexUrls,
  upsertStringOverrideField,
} from './dependencyEnvironmentState.ts';

test('dependency environment node state helpers initialize persist and apply backend data', () => {
  const requirements = {
    model_id: 'model-a',
    platform_key: 'linux-x86_64',
    backend_key: 'llama_cpp',
    dependency_contract_version: 1,
    validation_state: 'resolved' as const,
    validation_errors: [],
    bindings: [],
    selected_binding_ids: ['binding-a'],
  };

  const initialState = createDependencyEnvironmentNodeDataState({
    mode: 'manual',
    selected_binding_ids: ['binding-a'],
    dependency_requirements: requirements,
    activity_log: ['[12:00:00] resolve started'],
  });

  assert.equal(initialState.mode, 'manual');
  assert.deepEqual(initialState.selectedBindingIds, ['binding-a']);
  assert.equal(initialState.dependencyRequirements, requirements);

  assert.deepEqual(buildDependencyEnvironmentNodeData(initialState), {
    mode: 'manual',
    selected_binding_ids: ['binding-a'],
    dependency_requirements: requirements,
    dependency_status: null,
    environment_ref: null,
    manual_overrides: [],
    dependency_override_patches: [],
    activity_log: ['[12:00:00] resolve started'],
  });

  const nextState = applyDependencyEnvironmentActionNodeData(initialState, {
    mode: 'auto',
    selected_binding_ids: ['binding-b'],
    environment_ref: {
      contract_version: 1,
      env_id: 'env-a',
      state: 'ready',
    },
  });

  assert.equal(nextState.mode, 'auto');
  assert.deepEqual(nextState.selectedBindingIds, ['binding-b']);
  assert.equal(nextState.dependencyRequirements, requirements);
  assert.equal(nextState.environmentRef?.env_id, 'env-a');
});

test('adoptDependencyEnvironmentUpstreamRequirements projects connected requirements once', () => {
  const requirements = {
    model_id: 'model-a',
    platform_key: 'linux',
    dependency_contract_version: 1,
    validation_state: 'resolved' as const,
    validation_errors: [],
    selected_binding_ids: [],
    bindings: [
      {
        binding_id: 'binding-a',
        profile_id: 'profile-a',
        profile_version: 1,
        validation_state: 'resolved' as const,
        validation_errors: [],
        requirements: [],
      },
      {
        binding_id: 'binding-b',
        profile_id: 'profile-b',
        profile_version: 1,
        validation_state: 'resolved' as const,
        validation_errors: [],
        requirements: [],
      },
    ],
  };

  const adoption = adoptDependencyEnvironmentUpstreamRequirements(null, [], requirements);

  assert.equal(adoption?.dependencyRequirements, requirements);
  assert.deepEqual(adoption?.selectedBindingIds, ['binding-a', 'binding-b']);
  assert.deepEqual(adoption?.nodeData, {
    dependency_requirements: requirements,
    selected_binding_ids: ['binding-a', 'binding-b'],
  });
  assert.equal(
    adoptDependencyEnvironmentUpstreamRequirements(requirements, [], requirements),
    null,
  );
});

test('appendDependencyActivityLogLine formats and trims retained log lines', () => {
  assert.deepEqual(
    appendDependencyActivityLogLine(['[12:00:00] old'], '   ', '12:00:01', 2),
    ['[12:00:00] old'],
  );

  assert.deepEqual(
    appendDependencyActivityLogLine(
      ['[12:00:00] first', '[12:00:01] second'],
      'third',
      '12:00:02',
      2,
    ),
    ['[12:00:01] second', '[12:00:02] third'],
  );
});

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

test('dependency override summary helpers count and classify local patches', () => {
  const patches = [
    {
      contract_version: 1,
      binding_id: 'binding-a',
      scope: 'binding' as const,
      fields: { python_executable: '/usr/bin/python3' },
    },
    {
      contract_version: 1,
      binding_id: 'binding-a',
      scope: 'requirement' as const,
      requirement_name: 'Torch',
      fields: { index_url: 'https://packages.example/simple' },
    },
  ];

  assert.equal(countDependencyBindingPatches(patches, 'binding-a'), 2);
  assert.equal(countDependencyRequirementPatches(patches, 'binding-a', 'torch'), 1);
  assert.equal(hasDependencyBindingOverrideFields(patches, 'binding-a'), true);
  assert.equal(hasDependencyRequirementOverrideFields(patches, 'binding-a', 'torch'), true);
  assert.equal(hasDependencyRequirementOverrideFields(patches, 'binding-a', 'numpy'), false);
  assert.equal(
    readDependencyStringOverrideField(
      patches,
      'binding-a',
      'binding',
      undefined,
      'python_executable',
    ),
    '/usr/bin/python3',
  );
});

test('dependency override read and clear helpers preserve scope boundaries', () => {
  const patches = [
    {
      contract_version: 1,
      binding_id: 'binding-a',
      scope: 'binding' as const,
      fields: { python_executable: '/usr/bin/python3' },
    },
    {
      contract_version: 1,
      binding_id: 'binding-a',
      scope: 'requirement' as const,
      requirement_name: 'torch',
      fields: {
        extra_index_urls: ['https://a.example/simple', 'https://b.example/simple'],
      },
    },
    {
      contract_version: 1,
      binding_id: 'binding-b',
      scope: 'requirement' as const,
      requirement_name: 'torch',
      fields: { index_url: 'https://packages.example/simple' },
    },
  ];

  assert.equal(
    readDependencyExtraIndexUrls(patches, 'binding-a', 'torch'),
    'https://a.example/simple, https://b.example/simple',
  );
  assert.equal(clearDependencyBindingOverrides(patches, 'binding-a').length, 1);
  assert.deepEqual(
    clearDependencyRequirementOverrides(patches, 'binding-a', 'torch').map(
      (patch) => `${patch.binding_id}:${patch.scope}:${patch.requirement_name ?? ''}`,
    ),
    ['binding-a:binding:', 'binding-b:requirement:torch'],
  );
});

test('dependency binding selection helpers filter and toggle bindings', () => {
  const requirements = {
    model_id: 'model-a',
    platform_key: 'linux',
    dependency_contract_version: 1,
    validation_state: 'resolved' as const,
    validation_errors: [],
    selected_binding_ids: [],
    bindings: [
      {
        binding_id: 'binding-a',
        profile_id: 'profile-a',
        profile_version: 1,
        validation_state: 'resolved' as const,
        validation_errors: [],
        requirements: [],
      },
      {
        binding_id: 'binding-b',
        profile_id: 'profile-b',
        profile_version: 1,
        validation_state: 'resolved' as const,
        validation_errors: [],
        requirements: [],
      },
    ],
  };

  assert.deepEqual(filterDependencyEnvironmentBindings(requirements, []), requirements.bindings);
  assert.deepEqual(
    filterDependencyEnvironmentBindings(requirements, ['binding-b']).map(
      (binding) => binding.binding_id,
    ),
    ['binding-b'],
  );
  assert.equal(isDependencyEnvironmentBindingSelected([], 'binding-a'), true);
  assert.equal(isDependencyEnvironmentBindingSelected(['binding-b'], 'binding-a'), false);
  assert.deepEqual(toggleDependencyEnvironmentBindingSelection(['binding-a'], 'binding-a'), []);
  assert.deepEqual(toggleDependencyEnvironmentBindingSelection([], 'binding-a'), ['binding-a']);
  assert.deepEqual(toggleDependencyEnvironmentAllBindings(requirements, ['binding-a']), [
    'binding-a',
    'binding-b',
  ]);
  assert.deepEqual(
    toggleDependencyEnvironmentAllBindings(requirements, ['binding-a', 'binding-b']),
    [],
  );
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
  assert.equal(
    formatDependencyActivityTimestamp(new Date(2026, 3, 22, 13, 14, 15)),
    '13:14:15',
  );
});

test('setupDependencyEnvironmentActivityListener wires matching events and auto run', async () => {
  const activityLines: string[] = [];
  const calls: string[] = [];
  const matchingEvent = {
    timestamp: '2026-04-22T00:00:00Z',
    node_type: 'dependency-environment',
    model_path: '/models/model.gguf',
    phase: 'check',
    message: 'Checking torch',
  };
  const ignoredEvent = {
    ...matchingEvent,
    node_type: 'model-provider',
  };

  const unlisten = await setupDependencyEnvironmentActivityListener({
    listenEvent: async <Payload>(
      _eventName: string,
      handler: (event: { payload: Payload }) => void,
    ) => {
      handler({ payload: ignoredEvent as Payload });
      handler({ payload: matchingEvent as Payload });
      return () => {
        calls.push('unlisten');
      };
    },
    matchesActivityEvent: (payload) =>
      matchesDependencyActivityEvent(payload, '/models/model.gguf'),
    renderActivityEvent: renderDependencyActivityEvent,
    appendActivityLine: (line) => {
      activityLines.push(line);
    },
    persistNodeState: () => {
      calls.push('persist');
    },
    shouldRunModeAction: () => true,
    runModeAction: async () => {
      calls.push('run');
    },
  });

  assert.deepEqual(activityLines, ['check: Checking torch']);
  assert.deepEqual(calls, ['persist', 'run']);
  unlisten();
  assert.deepEqual(calls, ['persist', 'run', 'unlisten']);
});

test('setupDependencyEnvironmentActivityListener preserves listener errors for logs', async () => {
  const error = new Error('event bridge unavailable');

  await assert.rejects(
    setupDependencyEnvironmentActivityListener({
      listenEvent: async () => {
        throw error;
      },
      matchesActivityEvent: () => true,
      renderActivityEvent: renderDependencyActivityEvent,
      appendActivityLine: () => {},
      persistNodeState: () => {},
      shouldRunModeAction: () => false,
      runModeAction: async () => {},
    }),
    error,
  );

  assert.equal(
    formatDependencyEnvironmentListenerError(error),
    'listener: error="event bridge unavailable"',
  );
});

test('upsertStringOverrideField adds updates and removes empty patches', () => {
  assert.equal(
    formatDependencyOverrideUpdatedAt(new Date('2026-04-22T00:00:00.000Z')),
    '2026-04-22T00:00:00.000Z',
  );
  assert.equal(
    readDependencyOverrideInputValue(
      { target: { value: ' /usr/bin/python3 ' } } as unknown as Event,
    ),
    ' /usr/bin/python3 ',
  );
  assert.equal(readDependencyOverrideInputValue({ target: {} } as unknown as Event), '');

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

test('runDependencyEnvironmentActionRequest applies backend node data and busy state', async () => {
  const busyStates: boolean[] = [];
  const appliedNodeData: Record<string, unknown>[] = [];
  const activityLines: string[] = [];

  const didRun = await runDependencyEnvironmentActionRequest({
    action: 'check',
    payload: {
      action: 'check',
      mode: 'manual',
      modelPath: '/models/model.gguf',
    },
    invokeAction: async () => ({
      nodeData: {
        dependency_status: { state: 'ready' },
      },
    }),
    applyNodeData: (nodeData) => {
      appliedNodeData.push(nodeData);
    },
    appendActivityLine: (line) => {
      activityLines.push(line);
    },
    setBusy: (busy) => {
      busyStates.push(busy);
    },
  });

  assert.equal(didRun, true);
  assert.deepEqual(busyStates, [true, false]);
  assert.deepEqual(appliedNodeData, [{ dependency_status: { state: 'ready' } }]);
  assert.deepEqual(activityLines, []);
});

test('runDependencyEnvironmentActionRequest skips empty payloads and logs failures', async () => {
  const busyStates: boolean[] = [];
  const activityLines: string[] = [];
  const error = new Error('backend unavailable');

  const didRun = await runDependencyEnvironmentActionRequest({
    action: 'install',
    payload: null,
    invokeAction: async () => {
      throw new Error('unexpected invocation');
    },
    applyNodeData: () => {
      throw new Error('unexpected apply');
    },
    appendActivityLine: (line) => {
      activityLines.push(line);
    },
    setBusy: (busy) => {
      busyStates.push(busy);
    },
  });

  assert.equal(didRun, false);
  assert.equal(busyStates.length, 0);
  assert.equal(activityLines.length, 0);

  await assert.rejects(
    runDependencyEnvironmentActionRequest({
      action: 'install',
      payload: {
        action: 'install',
        mode: 'manual',
        modelPath: '/models/model.gguf',
      },
      invokeAction: async () => {
        throw error;
      },
      applyNodeData: () => {
        throw new Error('unexpected apply');
      },
      appendActivityLine: (line) => {
        activityLines.push(line);
      },
      setBusy: (busy) => {
        busyStates.push(busy);
      },
    }),
    error,
  );

  assert.deepEqual(busyStates, [true, false]);
  assert.deepEqual(activityLines, [formatDependencyEnvironmentActionError('install', error)]);
});

test('resolveDependencyEnvironmentUpstreamState projects connected model and override inputs', () => {
  const requirements = {
    model_id: 'model-a',
    platform_key: 'linux-x86_64',
    backend_key: 'llama_cpp',
    dependency_contract_version: 1,
    validation_state: 'resolved' as const,
    validation_errors: [],
    bindings: [],
    selected_binding_ids: ['binding-a'],
  };
  const state = resolveDependencyEnvironmentUpstreamState(
    'dependency-node',
    [
      {
        id: 'model-node',
        data: {
          modelPath: '/models/model.gguf',
          model_id: 'model-a',
          model_type: 'embedding',
          taskTypePrimary: 'embed',
          backendKey: 'llama_cpp',
          platform_context: { os: 'linux' },
          dependency_requirements: requirements,
        },
      },
      {
        id: 'override-node',
        data: {
          output: JSON.stringify([
            {
              contract_version: 1,
              binding_id: 'binding-a',
              scope: 'binding',
              fields: { python_executable: '/usr/bin/python3' },
            },
          ]),
        },
      },
    ],
    [
      {
        source: 'model-node',
        sourceHandle: 'model_path',
        target: 'dependency-node',
        targetHandle: 'model_path',
      },
      {
        source: 'override-node',
        sourceHandle: 'output',
        target: 'dependency-node',
        targetHandle: 'manual_overrides',
      },
    ],
  );

  assert.equal(state.modelPath, '/models/model.gguf');
  assert.equal(state.modelId, 'model-a');
  assert.equal(state.taskType, 'embed');
  assert.equal(state.backendKey, 'llama_cpp');
  assert.equal(state.requirements, requirements);
  assert.equal(state.manualOverrides.length, 1);
  assert.equal(state.manualOverrides[0].fields.python_executable, '/usr/bin/python3');
});
