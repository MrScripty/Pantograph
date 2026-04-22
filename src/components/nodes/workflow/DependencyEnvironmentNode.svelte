<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import DependencyEnvironmentActivityLog from './DependencyEnvironmentActivityLog.svelte';
  import DependencyEnvironmentStatusPanel from './DependencyEnvironmentStatusPanel.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import {
    dependencyTokenLabel,
    getPatchFrom,
    hasOverrideFields,
    isPatchTarget,
    mergeOverridePatches,
    parseOverridePatches,
    type DependencyActivityEvent,
    type DependencyOverridePatchV1,
    type EnvironmentRef,
    type ModelDependencyBinding,
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

  function deriveDisplayState(): string | null {
    if (dependencyStatus) return dependencyStatus.state;
    if (!dependencyRequirements) return null;
    switch (dependencyRequirements.validation_state) {
      case 'resolved':
        return 'resolved';
      case 'unknown_profile':
        return 'unresolved';
      default:
        return 'invalid';
    }
  }

  const dependencyBadge = $derived.by(() => {
    const state = deriveDisplayState();
    if (!state) return { label: 'requirements unknown', className: 'text-neutral-400 border-neutral-700' };
    switch (state) {
      case 'ready':
        return { label: 'deps ready', className: 'text-emerald-400 border-emerald-500/40' };
      case 'missing':
        return { label: 'deps missing', className: 'text-amber-400 border-amber-500/40' };
      case 'resolved':
        return { label: 'requirements resolved', className: 'text-cyan-300 border-cyan-500/40' };
      case 'checking':
        return { label: 'deps checking', className: 'text-cyan-400 border-cyan-500/40' };
      case 'installing':
        return { label: 'deps installing', className: 'text-sky-400 border-sky-500/40' };
      case 'unresolved':
        return { label: 'requirements unresolved', className: 'text-violet-400 border-violet-500/40' };
      case 'invalid':
        return { label: 'requirements invalid', className: 'text-orange-400 border-orange-500/40' };
      case 'failed':
        return { label: 'deps failed', className: 'text-red-400 border-red-500/40' };
      default:
        return {
          label: `deps ${dependencyTokenLabel(state)}`,
          className: 'text-neutral-300 border-neutral-600/50',
        };
    }
  });

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

  interface DependencyEnvironmentActionRequest {
    action: 'resolve' | 'check' | 'install' | 'run';
    mode?: 'auto' | 'manual';
    modelPath: string;
    modelId?: string;
    modelType?: string;
    taskTypePrimary?: string;
    backendKey?: string;
    platformContext?: Record<string, string>;
    selectedBindingIds?: string[];
    dependencyRequirements?: ModelDependencyRequirements;
    dependencyOverridePatches?: DependencyOverridePatchV1[];
  }

  interface DependencyEnvironmentActionResponse {
    nodeData: Record<string, unknown>;
  }

  function dependencyActionPayload(action: DependencyEnvironmentActionRequest['action']): DependencyEnvironmentActionRequest | null {
    const modelPath = (upstreamModelPath ?? '').trim();
    if (!modelPath) return null;
    return {
      action,
      mode,
      modelPath,
      modelId: upstreamModelId ?? dependencyRequirements?.model_id ?? undefined,
      modelType: upstreamModelType ?? undefined,
      taskTypePrimary: upstreamTaskType ?? undefined,
      backendKey: upstreamBackendKey ?? dependencyRequirements?.backend_key ?? undefined,
      platformContext: upstreamPlatformContext ?? undefined,
      selectedBindingIds,
      dependencyRequirements: upstreamRequirements ?? dependencyRequirements ?? undefined,
      dependencyOverridePatches: effectiveManualOverrides.length > 0 ? effectiveManualOverrides : undefined,
    };
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
    const normalized = line.trim();
    if (normalized.length === 0) return;
    const next = [...activityLog, `[${activityTimestamp()}] ${normalized}`];
    activityLog = next.length > MAX_ACTIVITY_LOG_LINES ? next.slice(next.length - MAX_ACTIVITY_LOG_LINES) : next;
    persistNodeState();
  }

  function matchesActivityEvent(payload: DependencyActivityEvent): boolean {
    const upstreamPath = (upstreamModelPath ?? '').trim();
    if (upstreamPath.length === 0) return false;
    const eventPath = (payload.model_path ?? '').trim();
    if (eventPath.length === 0 || eventPath !== upstreamPath) return false;
    return (payload.node_type ?? '').trim() === 'dependency-environment';
  }

  function renderActivityEvent(payload: DependencyActivityEvent): string {
    const parts = [payload.phase];
    if (payload.binding_id) parts.push(payload.binding_id);
    if (payload.requirement_name) parts.push(payload.requirement_name);
    if (payload.stream) parts.push(payload.stream);
    return `${parts.join(' | ')}: ${payload.message}`;
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
    const value = rawValue.trim();
    let next = [...manualOverrides];
    const idx = next.findIndex((patch) => isPatchTarget(patch, bindingId, scope, requirementName));
    const patch: DependencyOverridePatchV1 =
      idx >= 0
        ? {
            ...next[idx],
            fields: { ...next[idx].fields },
          }
        : {
            contract_version: 1,
            binding_id: bindingId,
            scope,
            requirement_name: scope === 'requirement' ? requirementName : undefined,
            fields: {},
            source: 'user',
          };

    if (value.length === 0) {
      delete patch.fields[field];
    } else {
      patch.fields[field] = value;
    }
    patch.source = 'user';
    patch.updated_at = new Date().toISOString();

    if (!hasOverrideFields(patch.fields)) {
      if (idx >= 0) {
        next.splice(idx, 1);
      }
    } else if (idx >= 0) {
      next[idx] = patch;
    } else {
      next.push(patch);
    }

    manualOverrides = next;
    persistNodeState();
  }

  function setExtraIndexUrls(
    bindingId: string,
    requirementName: string,
    rawValue: string
  ) {
    const parts = rawValue
      .split(',')
      .map((part) => part.trim())
      .filter((part) => part.length > 0);
    const deduped = Array.from(new Set(parts));

    let next = [...manualOverrides];
    const idx = next.findIndex((patch) => isPatchTarget(patch, bindingId, 'requirement', requirementName));
    const patch: DependencyOverridePatchV1 =
      idx >= 0
        ? {
            ...next[idx],
            fields: { ...next[idx].fields },
          }
        : {
            contract_version: 1,
            binding_id: bindingId,
            scope: 'requirement',
            requirement_name: requirementName,
            fields: {},
            source: 'user',
          };

    if (deduped.length === 0) {
      delete patch.fields.extra_index_urls;
    } else {
      patch.fields.extra_index_urls = deduped;
    }
    patch.source = 'user';
    patch.updated_at = new Date().toISOString();

    if (!hasOverrideFields(patch.fields)) {
      if (idx >= 0) {
        next.splice(idx, 1);
      }
    } else if (idx >= 0) {
      next[idx] = patch;
    } else {
      next.push(patch);
    }

    manualOverrides = next;
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

  function filteredBindings(requirements: ModelDependencyRequirements): ModelDependencyBinding[] {
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

        <div class="flex gap-1 text-[10px]">
          <button
            type="button"
            class="px-2 py-0.5 rounded transition-colors {mode === 'auto'
              ? 'bg-cyan-600/30 text-cyan-300'
              : 'text-neutral-500 hover:text-neutral-300'}"
            onclick={() => setMode('auto')}
          >
            Auto
          </button>
          <button
            type="button"
            class="px-2 py-0.5 rounded transition-colors {mode === 'manual'
              ? 'bg-cyan-600/30 text-cyan-300'
              : 'text-neutral-500 hover:text-neutral-300'}"
            onclick={() => setMode('manual')}
          >
            Manual
          </button>
        </div>

        {#if mode === 'manual'}
          <div class="rounded border border-neutral-700 px-2 py-1 space-y-2">
            <div class="flex items-center gap-2">
              <span class="text-[10px] text-neutral-400">Structured Override Controls</span>
              <span class="ml-auto text-[10px] text-neutral-500">
                {effectiveManualOverrides.length} effective patch(es)
              </span>
            </div>
            <div class="text-[10px] text-neutral-500">
              Configure per binding and per requirement below. Overrides are applied immediately.
            </div>
            {#if upstreamManualOverrides.length > 0}
              <div class="text-[10px] text-amber-300">
                {upstreamManualOverrides.length} patch(es) are provided by connected input. Local form edits override conflicts.
              </div>
            {/if}
            <div class="flex items-center gap-2">
              <button
                type="button"
                class="text-[10px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                onclick={clearAllOverrides}
                disabled={isBusy || manualOverrides.length === 0}
              >
                Clear All Overrides
              </button>
              {#if dependencyRequirements}
                <button
                  type="button"
                  class="text-[10px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                  onclick={() => toggleSelectedBindingsToAll(dependencyRequirements)}
                  disabled={isBusy}
                >
                  Toggle All Bindings
                </button>
              {/if}
            </div>
          </div>
        {/if}

        {#if dependencyRequirements && dependencyRequirements.bindings.length > 0}
          <div class="rounded border border-neutral-700 px-2 py-1 text-[10px] space-y-1">
            {#each filteredBindings(dependencyRequirements) as binding (binding.binding_id)}
              <div class="rounded border border-neutral-800 px-2 py-1">
                <div class="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={bindingHasSelection(binding.binding_id)}
                    disabled={isBusy}
                    onchange={() => toggleBinding(binding.binding_id)}
                  />
                  <span class="text-neutral-200 truncate" title={binding.binding_id}>{binding.binding_id}</span>
                  {#if mode === 'manual' && bindingPatchCount(binding.binding_id) > 0}
                    <span class="ml-auto text-[9px] text-cyan-300">
                      {bindingPatchCount(binding.binding_id)} override patch(es)
                    </span>
                  {/if}
                </div>
                <div class="text-[9px] text-neutral-500 truncate" title={binding.profile_id + ' v' + binding.profile_version}>
                  {binding.profile_id} v{binding.profile_version}
                </div>
                <div class="text-[9px] text-neutral-400">validation: {dependencyTokenLabel(binding.validation_state)}</div>
                {#if binding.requirements.length > 0}
                  <div class="space-y-1 mt-1">
                    {#if mode === 'manual'}
                      <div class="rounded border border-cyan-900/40 bg-cyan-950/10 px-2 py-1 space-y-1">
                        <div class="text-[10px] text-cyan-300">Binding Override</div>
                        <label class="text-[9px] text-neutral-400">Python executable</label>
                        <input
                          type="text"
                          class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                          value={getStringOverrideField(binding.binding_id, 'binding', undefined, 'python_executable')}
                          placeholder="/path/to/python or python3"
                          onchange={(event) => handleBindingPythonExecutableChange(binding.binding_id, event)}
                        />
                        <div class="flex items-center gap-2">
                          <button
                            type="button"
                            class="text-[9px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                            onclick={() => setStringOverrideField(binding.binding_id, 'binding', undefined, 'python_executable', '')}
                            disabled={isBusy || !hasBindingLocalOverride(binding.binding_id)}
                          >
                            Clear Local Binding Override
                          </button>
                          <button
                            type="button"
                            class="text-[9px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                            onclick={() => clearBindingOverrides(binding.binding_id)}
                            disabled={isBusy || bindingLocalPatchCount(binding.binding_id) === 0}
                          >
                            Clear Local Binding Patches
                          </button>
                        </div>
                      </div>
                    {/if}

                    {#each binding.requirements as requirement (`${binding.binding_id}:${requirement.name}`)}
                      <div class="rounded border border-neutral-800 px-2 py-1 space-y-1">
                        <div class="text-[9px] text-neutral-200">
                          {requirement.name}{requirement.exact_pin}
                        </div>
                        {#if mode === 'manual'}
                          <div class="space-y-1">
                            <label class="text-[9px] text-neutral-400">Index URL</label>
                            <input
                              type="text"
                              class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                              value={getStringOverrideField(binding.binding_id, 'requirement', requirement.name, 'index_url')}
                              placeholder="https://..."
                              onchange={(event) => handleRequirementFieldChange(binding.binding_id, requirement.name, 'index_url', event)}
                            />
                            <label class="text-[9px] text-neutral-400">Extra index URLs (comma-separated)</label>
                            <input
                              type="text"
                              class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                              value={getExtraIndexUrls(binding.binding_id, requirement.name)}
                              placeholder="https://a, https://b"
                              onchange={(event) => handleRequirementExtraUrlsChange(binding.binding_id, requirement.name, event)}
                            />
                            <label class="text-[9px] text-neutral-400">Wheel source path</label>
                            <input
                              type="text"
                              class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                              value={getStringOverrideField(binding.binding_id, 'requirement', requirement.name, 'wheel_source_path')}
                              placeholder="/path/to/wheels"
                              onchange={(event) => handleRequirementFieldChange(binding.binding_id, requirement.name, 'wheel_source_path', event)}
                            />
                            <label class="text-[9px] text-neutral-400">Package source override</label>
                            <input
                              type="text"
                              class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                              value={getStringOverrideField(binding.binding_id, 'requirement', requirement.name, 'package_source_override')}
                              placeholder="custom source descriptor"
                              onchange={(event) => handleRequirementFieldChange(binding.binding_id, requirement.name, 'package_source_override', event)}
                            />
                            <div class="flex items-center gap-2">
                              <button
                                type="button"
                                class="text-[9px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                                onclick={() => clearRequirementOverrides(binding.binding_id, requirement.name)}
                                disabled={isBusy || !hasRequirementLocalOverrides(binding.binding_id, requirement.name)}
                              >
                                Clear Local Requirement Overrides
                              </button>
                              {#if requirementPatchCount(binding.binding_id, requirement.name) > 0}
                                <span class="text-[9px] text-cyan-300">
                                  {requirementPatchCount(binding.binding_id, requirement.name)} patch(es)
                                </span>
                              {/if}
                            </div>
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if environmentRef}
          <div class="rounded border border-cyan-900/50 bg-cyan-950/20 px-2 py-1 text-[10px] space-y-1">
            <div class="text-cyan-300">environment state: {environmentRef.state}</div>
            {#if environmentRef.env_id}
              <div class="text-neutral-300 truncate" title={environmentRef.env_id}>
                env: {environmentRef.env_id}
              </div>
            {/if}
            {#if environmentRef.python_executable}
              <div class="text-neutral-400 truncate" title={environmentRef.python_executable}>
                python: {environmentRef.python_executable}
              </div>
            {/if}
          </div>
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
