<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import DependencyEnvironmentActivityLog from './DependencyEnvironmentActivityLog.svelte';
  import DependencyEnvironmentBindingsPanel from './DependencyEnvironmentBindingsPanel.svelte';
  import DependencyEnvironmentModeControls from './DependencyEnvironmentModeControls.svelte';
  import DependencyEnvironmentRefPanel from './DependencyEnvironmentRefPanel.svelte';
  import DependencyEnvironmentStatusPanel from './DependencyEnvironmentStatusPanel.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import {
    buildDependencyEnvironmentActionPayload,
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
    hasDependencyBindingOverrideFields,
    hasDependencyRequirementOverrideFields,
    isDependencyEnvironmentBindingSelected,
    matchesDependencyActivityEvent,
    mergeOverridePatches,
    readDependencyExtraIndexUrls,
    readDependencyStringOverrideField,
    renderDependencyActivityEvent,
    resolveDependencyEnvironmentUpstreamState,
    toggleDependencyEnvironmentAllBindings,
    toggleDependencyEnvironmentBindingSelection,
    upsertExtraIndexUrls,
    upsertStringOverrideField,
    type DependencyActivityEvent,
    type DependencyEnvironmentActionRequest,
    type DependencyEnvironmentActionResponse,
    type DependencyEnvironmentNodeDataState,
    type DependencyOverridePatchV1,
    type EnvironmentRef,
    type ModelDependencyRequirements,
    type ModelDependencyStatus,
    type StringOverrideField,
  } from './dependencyEnvironmentState';
  import { edges, nodes, updateNodeData } from '../../../stores/workflowStore';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      mode?: 'auto' | 'manual';
      selected_binding_ids?: string[];
      dependency_requirements?: ModelDependencyRequirements;
      dependency_status?: ModelDependencyStatus;
      environment_ref?: EnvironmentRef;
      manual_overrides?: DependencyOverridePatchV1[];
      activity_log?: string[];
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  const initialNodeState = createDependencyEnvironmentNodeDataState(data);

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
    if (upstreamRequirements && !dependencyRequirements) {
      dependencyRequirements = upstreamRequirements;
      if (selectedBindingIds.length === 0) {
        selectedBindingIds = upstreamRequirements.selected_binding_ids?.length
          ? upstreamRequirements.selected_binding_ids
          : upstreamRequirements.bindings.map((b) => b.binding_id);
      }
      updateNodeData(id, {
        dependency_requirements: upstreamRequirements,
        selected_binding_ids: selectedBindingIds,
      });
    }
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
    if (!payload) return;

    isBusy = true;
    try {
      const response = await invoke<DependencyEnvironmentActionResponse>(
        'run_dependency_environment_action',
        { request: payload }
      );
      applyDependencyActionNodeData(response.nodeData);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      appendActivityLine(`${action}: error="${message}"`);
      throw error;
    } finally {
      isBusy = false;
    }
  }

  function activityTimestamp(): string {
    return new Date().toLocaleTimeString('en-US', { hour12: false });
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
      new Date().toISOString()
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
      new Date().toISOString()
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
    const target = event.target as HTMLInputElement;
    setStringOverrideField(bindingId, 'binding', undefined, 'python_executable', target.value);
  }

  function handleRequirementFieldChange(
    bindingId: string,
    requirementName: string,
    field: Exclude<StringOverrideField, 'python_executable'>,
    event: Event
  ) {
    const target = event.target as HTMLInputElement;
    setStringOverrideField(bindingId, 'requirement', requirementName, field, target.value);
  }

  function handleRequirementExtraUrlsChange(
    bindingId: string,
    requirementName: string,
    event: Event
  ) {
    const target = event.target as HTMLInputElement;
    setExtraIndexUrls(bindingId, requirementName, target.value);
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
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen<DependencyActivityEvent>('dependency-activity', (event) => {
        const payload = event.payload;
        if (!matchesActivityEvent(payload)) return;
        appendActivityLine(renderActivityEvent(payload));
      });

      persistNodeState();
      if (mode === 'auto' && upstreamModelPath) {
        await runModeAction();
      }
    };

    setup().catch((error) => {
      const message = error instanceof Error ? error.message : String(error);
      appendActivityLine(`listener: error="${message}"`);
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
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-cyan-700 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13a7 7 0 1114 0v3a2 2 0 01-2 2H5a2 2 0 01-2-2v-3z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Dependency Environment'}</span>
      </div>
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
