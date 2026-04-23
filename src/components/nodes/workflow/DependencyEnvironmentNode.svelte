<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import DependencyEnvironmentActivityLog from './DependencyEnvironmentActivityLog.svelte';
  import DependencyEnvironmentBindingsPanel from './DependencyEnvironmentBindingsPanel.svelte';
  import DependencyEnvironmentModeControls from './DependencyEnvironmentModeControls.svelte';
  import DependencyEnvironmentNodeHeader from './DependencyEnvironmentNodeHeader.svelte';
  import DependencyEnvironmentRefPanel from './DependencyEnvironmentRefPanel.svelte';
  import DependencyEnvironmentStatusPanel from './DependencyEnvironmentStatusPanel.svelte';
  import {
    buildDependencyEnvironmentActionPayload,
    adoptDependencyEnvironmentUpstreamRequirements,
    appendDependencyActivityLogLine,
    applyDependencyEnvironmentActionNodeData,
    buildDependencyEnvironmentNodeData,
    clearDependencyBindingOverrides,
    clearDependencyRequirementOverrides,
    createDependencyEnvironmentNodeDataState,
    countDependencyBindingPatches,
    countDependencyRequirementPatches,
    dependencyBadgeFor,
    filterDependencyEnvironmentBindings,
    formatDependencyActivityTimestamp,
    formatDependencyOverrideUpdatedAt,
    hasDependencyBindingOverrideFields,
    hasDependencyRequirementOverrideFields,
    isDependencyEnvironmentBindingSelected,
    formatDependencyEnvironmentListenerError,
    matchesDependencyActivityEvent,
    mergeOverridePatches,
    readDependencyExtraIndexUrls,
    readDependencyStringOverrideField,
    renderDependencyActivityEvent,
    resolveDependencyEnvironmentUpstreamState,
    runDependencyEnvironmentActionRequest,
    readDependencyOverrideInputValue,
    setupDependencyEnvironmentActivityListener,
    toggleDependencyEnvironmentAllBindings,
    toggleDependencyEnvironmentBindingSelection,
    upsertExtraIndexUrls,
    upsertStringOverrideField,
    type DependencyActivityEvent,
    type DependencyEnvironmentActionRequest,
    type DependencyEnvironmentActionResponse,
    type DependencyEnvironmentNodeDataState,
    type DependencyEnvironmentNodeProps,
    type DependencyOverridePatchV1,
    type EnvironmentRef,
    type ModelDependencyRequirements,
    type ModelDependencyStatus,
    type StringOverrideField,
  } from './dependencyEnvironmentState';
  import { edges, nodes, updateNodeData } from '../../../stores/workflowStore';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let { id, data, selected = false }: DependencyEnvironmentNodeProps = $props();

  const initialNodeState = untrack(() => createDependencyEnvironmentNodeDataState(data));

  let mode = $state<'auto' | 'manual'>(initialNodeState.mode);
  let selectedBindingIds = $state<string[]>(initialNodeState.selectedBindingIds);
  let dependencyRequirements = $state<ModelDependencyRequirements | null>(
    initialNodeState.dependencyRequirements
  );
  let dependencyStatus = $state<ModelDependencyStatus | null>(initialNodeState.dependencyStatus);
  let environmentRef = $state<EnvironmentRef | null>(initialNodeState.environmentRef);
  let manualOverrides = $state<DependencyOverridePatchV1[]>(initialNodeState.manualOverrides);
  let activityLog = $state<string[]>(initialNodeState.activityLog);
  let isBusy = $state(false);

  const MAX_ACTIVITY_LOG_LINES = 200;

  let upstreamState = $derived(resolveDependencyEnvironmentUpstreamState(id, $nodes, $edges));
  let upstreamModelPath = $derived(upstreamState.modelPath);
  let upstreamModelId = $derived(upstreamState.modelId);
  let upstreamModelType = $derived(upstreamState.modelType);
  let upstreamTaskType = $derived(upstreamState.taskType);
  let upstreamBackendKey = $derived(upstreamState.backendKey);
  let upstreamPlatformContext = $derived(upstreamState.platformContext);
  let upstreamRequirements = $derived(upstreamState.requirements);
  let upstreamManualOverrides = $derived(upstreamState.manualOverrides);

  let effectiveManualOverrides = $derived.by(() =>
    mergeOverridePatches(upstreamManualOverrides, manualOverrides)
  );

  $effect(() => {
    const adoption = adoptDependencyEnvironmentUpstreamRequirements(
      dependencyRequirements,
      selectedBindingIds,
      upstreamRequirements
    );
    if (!adoption) return;

    dependencyRequirements = adoption.dependencyRequirements;
    selectedBindingIds = adoption.selectedBindingIds;
    updateNodeData(id, adoption.nodeData);
  });

  const dependencyBadge = $derived(dependencyBadgeFor(dependencyRequirements, dependencyStatus));

  function currentNodeState(): DependencyEnvironmentNodeDataState {
    return {
      mode,
      selectedBindingIds,
      dependencyRequirements,
      dependencyStatus,
      environmentRef,
      manualOverrides,
      activityLog,
    };
  }

  function persistNodeState() {
    updateNodeData(id, buildDependencyEnvironmentNodeData(currentNodeState()));
  }

  function dependencyActionPayload(action: DependencyEnvironmentActionRequest['action']): DependencyEnvironmentActionRequest | null {
    return buildDependencyEnvironmentActionPayload({
      action,
      mode,
      upstreamModelPath,
      upstreamModelId,
      upstreamModelType,
      upstreamTaskType,
      upstreamBackendKey,
      upstreamPlatformContext,
      selectedBindingIds,
      upstreamRequirements,
      dependencyRequirements,
      effectiveManualOverrides,
    });
  }

  function applyDependencyActionNodeData(nodeData: Record<string, unknown>) {
    updateNodeData(id, nodeData);
    const nextState = applyDependencyEnvironmentActionNodeData(currentNodeState(), nodeData);
    mode = nextState.mode;
    selectedBindingIds = nextState.selectedBindingIds;
    dependencyRequirements = nextState.dependencyRequirements;
    dependencyStatus = nextState.dependencyStatus;
    environmentRef = nextState.environmentRef;
  }

  async function runDependencyEnvironmentAction(
    action: DependencyEnvironmentActionRequest['action']
  ) {
    const payload = dependencyActionPayload(action);
    await runDependencyEnvironmentActionRequest({
      action,
      payload,
      invokeAction: (request) =>
        invoke<DependencyEnvironmentActionResponse>('run_dependency_environment_action', {
          request,
        }),
      applyNodeData: applyDependencyActionNodeData,
      appendActivityLine,
      setBusy: (busy) => {
        isBusy = busy;
      },
    });
  }

  function activityTimestamp(): string {
    return formatDependencyActivityTimestamp(new Date());
  }

  function appendActivityLine(line: string) {
    const next = appendDependencyActivityLogLine(
      activityLog,
      line,
      activityTimestamp(),
      MAX_ACTIVITY_LOG_LINES
    );
    if (next === activityLog) return;
    activityLog = next;
    persistNodeState();
  }

  function matchesActivityEvent(payload: DependencyActivityEvent): boolean {
    return matchesDependencyActivityEvent(payload, upstreamModelPath);
  }

  function renderActivityEvent(payload: DependencyActivityEvent): string {
    return renderDependencyActivityEvent(payload);
  }

  function setStringOverrideField(
    bindingId: string,
    scope: 'binding' | 'requirement',
    requirementName: string | undefined,
    field: StringOverrideField,
    rawValue: string
  ) {
    manualOverrides = upsertStringOverrideField(
      manualOverrides,
      bindingId,
      scope,
      requirementName,
      field,
      rawValue,
      formatDependencyOverrideUpdatedAt(new Date())
    );
    persistNodeState();
  }

  function setExtraIndexUrls(
    bindingId: string,
    requirementName: string,
    rawValue: string
  ) {
    manualOverrides = upsertExtraIndexUrls(
      manualOverrides,
      bindingId,
      requirementName,
      rawValue,
      formatDependencyOverrideUpdatedAt(new Date())
    );
    persistNodeState();
  }

  function getStringOverrideField(
    bindingId: string,
    scope: 'binding' | 'requirement',
    requirementName: string | undefined,
    field: StringOverrideField
  ): string {
    return readDependencyStringOverrideField(
      effectiveManualOverrides,
      bindingId,
      scope,
      requirementName,
      field
    );
  }

  function getExtraIndexUrls(bindingId: string, requirementName: string): string {
    return readDependencyExtraIndexUrls(effectiveManualOverrides, bindingId, requirementName);
  }

  function clearBindingOverrides(bindingId: string) {
    manualOverrides = clearDependencyBindingOverrides(manualOverrides, bindingId);
    persistNodeState();
  }

  function clearBindingPythonExecutable(bindingId: string) {
    setStringOverrideField(bindingId, 'binding', undefined, 'python_executable', '');
  }

  function clearAllOverrides() {
    manualOverrides = [];
    persistNodeState();
  }

  function clearActivityLog() {
    activityLog = [];
    persistNodeState();
  }

  function handleBindingPythonExecutableChange(bindingId: string, event: Event) {
    setStringOverrideField(
      bindingId,
      'binding',
      undefined,
      'python_executable',
      readDependencyOverrideInputValue(event)
    );
  }

  function handleRequirementFieldChange(
    bindingId: string,
    requirementName: string,
    field: Exclude<StringOverrideField, 'python_executable'>,
    event: Event
  ) {
    setStringOverrideField(
      bindingId,
      'requirement',
      requirementName,
      field,
      readDependencyOverrideInputValue(event)
    );
  }

  function handleRequirementExtraUrlsChange(
    bindingId: string,
    requirementName: string,
    event: Event
  ) {
    setExtraIndexUrls(bindingId, requirementName, readDependencyOverrideInputValue(event));
  }

  function clearRequirementOverrides(bindingId: string, requirementName: string) {
    manualOverrides = clearDependencyRequirementOverrides(
      manualOverrides,
      bindingId,
      requirementName
    );
    persistNodeState();
  }

  function toggleSelectedBindingsToAll(requirements: ModelDependencyRequirements) {
    selectedBindingIds = toggleDependencyEnvironmentAllBindings(requirements, selectedBindingIds);
    persistNodeState();
  }

  async function resolveDependencyRequirements() {
    await runDependencyEnvironmentAction('resolve');
  }

  async function checkDependencies() {
    await runDependencyEnvironmentAction('check');
  }

  async function installDependencies() {
    await runDependencyEnvironmentAction('install');
  }

  async function runModeAction() {
    await runDependencyEnvironmentAction('run');
  }

  function toggleBinding(bindingId: string) {
    selectedBindingIds = toggleDependencyEnvironmentBindingSelection(selectedBindingIds, bindingId);
    persistNodeState();
  }

  function setMode(next: 'auto' | 'manual') {
    mode = next;
    persistNodeState();
  }

  onMount(() => {
    let unlisten: (() => void) | null = null;

    setupDependencyEnvironmentActivityListener({
      listenEvent: listen,
      matchesActivityEvent,
      renderActivityEvent,
      appendActivityLine,
      persistNodeState,
      shouldRunModeAction: () => mode === 'auto' && Boolean(upstreamModelPath),
      runModeAction,
    })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch((error) => {
        appendActivityLine(formatDependencyEnvironmentListenerError(error));
      });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  });
</script>

<div class="dependency-env-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <DependencyEnvironmentNodeHeader label={data.label || 'Dependency Environment'} />
    {/snippet}

      <div class="space-y-2">
        <DependencyEnvironmentStatusPanel
          hasModelPath={Boolean(upstreamModelPath)}
          {dependencyBadge}
          {dependencyStatus}
          {isBusy}
          onRun={runModeAction}
          onResolve={resolveDependencyRequirements}
          onCheck={checkDependencies}
          onInstall={installDependencies}
        />

        <DependencyEnvironmentModeControls {mode} onSetMode={setMode} />

        {#if dependencyRequirements && dependencyRequirements.bindings.length > 0}
          <DependencyEnvironmentBindingsPanel
            requirements={dependencyRequirements}
            bindings={filterDependencyEnvironmentBindings(dependencyRequirements, selectedBindingIds)}
            {mode}
            {isBusy}
            {effectiveManualOverrides}
            {upstreamManualOverrides}
            {manualOverrides}
            bindingHasSelection={(bindingId) =>
              isDependencyEnvironmentBindingSelected(selectedBindingIds, bindingId)}
            bindingPatchCount={(bindingId) =>
              countDependencyBindingPatches(effectiveManualOverrides, bindingId)}
            bindingLocalPatchCount={(bindingId) =>
              countDependencyBindingPatches(manualOverrides, bindingId)}
            requirementPatchCount={(bindingId, requirementName) =>
              countDependencyRequirementPatches(
                effectiveManualOverrides,
                bindingId,
                requirementName
              )}
            hasRequirementLocalOverrides={(bindingId, requirementName) =>
              hasDependencyRequirementOverrideFields(
                manualOverrides,
                bindingId,
                requirementName
              )}
            hasBindingLocalOverride={(bindingId) =>
              hasDependencyBindingOverrideFields(manualOverrides, bindingId)}
            {getStringOverrideField}
            {getExtraIndexUrls}
            onClearAllOverrides={clearAllOverrides}
            onToggleAllBindings={() => toggleSelectedBindingsToAll(dependencyRequirements)}
            onToggleBinding={toggleBinding}
            onBindingPythonExecutableChange={handleBindingPythonExecutableChange}
            onClearBindingPythonExecutable={clearBindingPythonExecutable}
            onClearBindingOverrides={clearBindingOverrides}
            onRequirementFieldChange={handleRequirementFieldChange}
            onRequirementExtraUrlsChange={handleRequirementExtraUrlsChange}
            onClearRequirementOverrides={clearRequirementOverrides}
          />
        {/if}

        {#if environmentRef}
          <DependencyEnvironmentRefPanel {environmentRef} />
        {/if}

        {#if upstreamModelPath}
          <DependencyEnvironmentActivityLog {activityLog} {isBusy} onClear={clearActivityLog} />
        {/if}
      </div>
  </BaseNode>
</div>

<style>
  .dependency-env-wrapper :global(.base-node) {
    border-color: rgba(8, 145, 178, 0.5);
  }

  .dependency-env-wrapper :global(.node-header) {
    background-color: rgba(8, 145, 178, 0.2);
    border-color: rgba(8, 145, 178, 0.3);
  }

</style>
