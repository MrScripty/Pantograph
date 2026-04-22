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
    type DependencyActivityEvent,
    type DependencyEnvironmentActionRequest,
    type DependencyEnvironmentActionResponse,
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

  let mode = $state<'auto' | 'manual'>(data.mode ?? 'auto');
  let selectedBindingIds = $state<string[]>(
    Array.isArray(data.selected_binding_ids) ? data.selected_binding_ids : []
  );
  let dependencyRequirements = $state<ModelDependencyRequirements | null>(
    (data.dependency_requirements as ModelDependencyRequirements | null) ?? null
  );
  let dependencyStatus = $state<ModelDependencyStatus | null>(
    (data.dependency_status as ModelDependencyStatus | null) ?? null
  );
  let environmentRef = $state<EnvironmentRef | null>((data.environment_ref as EnvironmentRef | null) ?? null);
  let manualOverrides = $state<DependencyOverridePatchV1[]>(
    Array.isArray(data.manual_overrides) ? (data.manual_overrides as DependencyOverridePatchV1[]) : []
  );
  let activityLog = $state<string[]>(
    Array.isArray(data.activity_log) ? (data.activity_log as string[]) : []
  );
  let isBusy = $state(false);

  const MAX_ACTIVITY_LOG_LINES = 200;

  let modelSourceNode = $derived.by(() => {
    const edge = $edges.find((e) => e.target === id && e.targetHandle === 'model_path');
    if (!edge) return null;
    return $nodes.find((n) => n.id === edge.source) ?? null;
  });

  let requirementsSourceNode = $derived.by(() => {
    const edge = $edges.find((e) => e.target === id && e.targetHandle === 'dependency_requirements');
    if (!edge) return null;
    return $nodes.find((n) => n.id === edge.source) ?? null;
  });

  let manualOverridesSourceEdge = $derived.by(() => {
    return $edges.find((e) => e.target === id && e.targetHandle === 'manual_overrides') ?? null;
  });

  let manualOverridesSourceNode = $derived.by(() => {
    if (!manualOverridesSourceEdge) return null;
    return $nodes.find((n) => n.id === manualOverridesSourceEdge.source) ?? null;
  });

  let upstreamModelPath = $derived(
    (modelSourceNode?.data?.modelPath as string | undefined) ??
      (modelSourceNode?.data?.model_path as string | undefined) ??
      null
  );
  let upstreamModelId = $derived(
    (modelSourceNode?.data?.model_id as string | undefined) ??
      (modelSourceNode?.data?.modelId as string | undefined) ??
      null
  );
  let upstreamModelType = $derived(
    (modelSourceNode?.data?.model_type as string | undefined) ??
      (modelSourceNode?.data?.modelType as string | undefined) ??
      null
  );
  let upstreamTaskType = $derived(
    (modelSourceNode?.data?.task_type_primary as string | undefined) ??
      (modelSourceNode?.data?.taskTypePrimary as string | undefined) ??
      null
  );
  let upstreamBackendKey = $derived(
    (modelSourceNode?.data?.backend_key as string | undefined) ??
      (modelSourceNode?.data?.backendKey as string | undefined) ??
      null
  );
  let upstreamPlatformContext = $derived(
    (modelSourceNode?.data?.platform_context as Record<string, string> | undefined) ??
      (modelSourceNode?.data?.platformContext as Record<string, string> | undefined) ??
      null
  );
  let upstreamRequirements = $derived(
    ((requirementsSourceNode?.data?.dependency_requirements as ModelDependencyRequirements | undefined) ??
      (modelSourceNode?.data?.dependency_requirements as ModelDependencyRequirements | undefined) ??
      null)
  );

  let upstreamManualOverrides = $derived.by(() => {
    if (!manualOverridesSourceNode || !manualOverridesSourceEdge) return [] as DependencyOverridePatchV1[];
    const sourceData = (manualOverridesSourceNode.data as Record<string, unknown>) ?? {};
    const sourceHandle = manualOverridesSourceEdge.sourceHandle ?? '';
    const candidates: unknown[] = [];
    if (sourceHandle.length > 0) candidates.push(sourceData[sourceHandle]);
    candidates.push(
      sourceData.manual_overrides,
      sourceData.manualOverrides,
      sourceData.dependency_override_patches,
      sourceData.dependencyOverridePatches,
      sourceData.output,
      sourceData.value,
      sourceData.json
    );
    for (const candidate of candidates) {
      const parsed = parseOverridePatches(candidate);
      if (parsed.length > 0) return parsed;
    }
    return [] as DependencyOverridePatchV1[];
  });

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

  function persistNodeState() {
    updateNodeData(id, {
      mode,
      selected_binding_ids: selectedBindingIds,
      dependency_requirements: dependencyRequirements,
      dependency_status: dependencyStatus,
      environment_ref: environmentRef,
      manual_overrides: manualOverrides,
      dependency_override_patches: manualOverrides,
      activity_log: activityLog,
    });
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
    mode = (nodeData.mode as 'auto' | 'manual' | undefined) ?? mode;
    selectedBindingIds = Array.isArray(nodeData.selected_binding_ids)
      ? (nodeData.selected_binding_ids as string[])
      : selectedBindingIds;
    dependencyRequirements =
      (nodeData.dependency_requirements as ModelDependencyRequirements | null | undefined) ??
      dependencyRequirements;
    dependencyStatus =
      (nodeData.dependency_status as ModelDependencyStatus | null | undefined) ?? dependencyStatus;
    environmentRef =
      (nodeData.environment_ref as EnvironmentRef | null | undefined) ?? environmentRef;
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
    const formatted = formatDependencyActivityLine(line, activityTimestamp());
    if (!formatted) return;
    const next = [...activityLog, formatted];
    activityLog = next.length > MAX_ACTIVITY_LOG_LINES ? next.slice(next.length - MAX_ACTIVITY_LOG_LINES) : next;
    persistNodeState();
  }

  function matchesActivityEvent(payload: DependencyActivityEvent): boolean {
    return matchesDependencyActivityEvent(payload, upstreamModelPath);
  }

  function renderActivityEvent(payload: DependencyActivityEvent): string {
    return renderDependencyActivityEvent(payload);
  }

  function getPatch(
    bindingId: string,
    scope: 'binding' | 'requirement',
    requirementName?: string
  ): DependencyOverridePatchV1 | undefined {
    return getPatchFrom(manualOverrides, bindingId, scope, requirementName);
  }

  function getEffectivePatch(
    bindingId: string,
    scope: 'binding' | 'requirement',
    requirementName?: string
  ): DependencyOverridePatchV1 | undefined {
    return getPatchFrom(effectiveManualOverrides, bindingId, scope, requirementName);
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
    const patch = getEffectivePatch(bindingId, scope, requirementName);
    return (patch?.fields[field] ?? '').toString();
  }

  function getExtraIndexUrls(bindingId: string, requirementName: string): string {
    const patch = getEffectivePatch(bindingId, 'requirement', requirementName);
    const urls = patch?.fields.extra_index_urls ?? [];
    return urls.join(', ');
  }

  function clearBindingOverrides(bindingId: string) {
    manualOverrides = manualOverrides.filter((patch) => patch.binding_id !== bindingId);
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

  function bindingPatchCount(bindingId: string): number {
    return effectiveManualOverrides.filter((patch) => patch.binding_id === bindingId).length;
  }

  function bindingLocalPatchCount(bindingId: string): number {
    return manualOverrides.filter((patch) => patch.binding_id === bindingId).length;
  }

  function requirementPatchCount(bindingId: string, requirementName: string): number {
    return effectiveManualOverrides.filter((patch) =>
      isPatchTarget(patch, bindingId, 'requirement', requirementName)
    ).length;
  }

  function hasRequirementLocalOverrides(bindingId: string, requirementName: string): boolean {
    const patch = getPatch(bindingId, 'requirement', requirementName);
    return patch ? hasOverrideFields(patch.fields) : false;
  }

  function hasBindingLocalOverride(bindingId: string): boolean {
    const patch = getPatch(bindingId, 'binding');
    return patch ? hasOverrideFields(patch.fields) : false;
  }

  function clearRequirementOverrides(bindingId: string, requirementName: string) {
    manualOverrides = manualOverrides.filter(
      (patch) => !isPatchTarget(patch, bindingId, 'requirement', requirementName)
    );
    persistNodeState();
  }

  function filteredBindings(requirements: ModelDependencyRequirements) {
    if (selectedBindingIds.length === 0) return requirements.bindings;
    return requirements.bindings.filter((binding) => selectedBindingIds.includes(binding.binding_id));
  }

  function bindingHasSelection(bindingId: string): boolean {
    if (selectedBindingIds.length === 0) return true;
    return selectedBindingIds.includes(bindingId);
  }

  function toggleSelectedBindingsToAll(requirements: ModelDependencyRequirements) {
    if (selectedBindingIds.length === requirements.bindings.length) {
      selectedBindingIds = [];
    } else {
      selectedBindingIds = requirements.bindings.map((binding) => binding.binding_id);
    }
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
    if (selectedBindingIds.includes(bindingId)) {
      selectedBindingIds = selectedBindingIds.filter((id) => id !== bindingId);
    } else {
      selectedBindingIds = [...selectedBindingIds, bindingId];
    }
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
            bindings={filteredBindings(dependencyRequirements)}
            {mode}
            {isBusy}
            {effectiveManualOverrides}
            {upstreamManualOverrides}
            {manualOverrides}
            {bindingHasSelection}
            {bindingPatchCount}
            {bindingLocalPatchCount}
            {requirementPatchCount}
            {hasRequirementLocalOverrides}
            {hasBindingLocalOverride}
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
