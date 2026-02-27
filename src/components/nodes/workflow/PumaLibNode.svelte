<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, PortOption, PortOptionsResult } from '../../../services/workflow/types';
  import {
    updateNodeData,
    syncInferencePorts,
    syncExpandPorts,
  } from '../../../stores/workflowStore';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  interface InferenceParamSchema {
    key: string;
    label: string;
    param_type: 'Number' | 'Integer' | 'String' | 'Boolean';
    default: unknown;
    description?: string;
    constraints?: { min?: number; max?: number; allowed_values?: unknown[] };
  }

  type DependencyState =
    | 'ready'
    | 'missing'
    | 'installing'
    | 'failed'
    | 'unknown_profile'
    | 'manual_intervention_required'
    | 'profile_conflict'
    | 'required_binding_omitted';

  interface ModelDependencyBinding {
    bindingId: string;
    profileId: string;
    profileVersion: number;
    profileHash?: string;
    bindingKind: string;
    backendKey?: string;
    platformSelector?: string;
    envId: string;
  }

  interface ModelDependencyPlan {
    state: DependencyState;
    code?: string;
    message?: string;
    reviewReasons?: string[];
    planId?: string;
    bindings?: ModelDependencyBinding[];
    selectedBindingIds?: string[];
    requiredBindingIds?: string[];
  }

  interface ModelDependencyBindingStatus {
    bindingId: string;
    envId: string;
    state: DependencyState;
    missingComponents?: string[];
    installedComponents?: string[];
    failedComponents?: string[];
    message?: string;
  }

  interface ModelDependencyStatus {
    state: DependencyState;
    code?: string;
    message?: string;
    reviewReasons?: string[];
    planId?: string;
    bindings?: ModelDependencyBindingStatus[];
    checkedAt?: string;
  }

  interface ModelDependencyInstallResult {
    state: DependencyState;
    code?: string;
    message?: string;
    reviewReasons?: string[];
    planId?: string;
    bindings?: ModelDependencyBindingStatus[];
    installedAt?: string;
  }

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      modelPath?: string;
      modelName?: string;
      model_id?: string;
      model_type?: string;
      task_type_primary?: string;
      backend_key?: string;
      platform_context?: Record<string, string>;
      dependency_plan_id?: string;
      dependency_bindings?: ModelDependencyBinding[];
      review_reasons?: string[];
      selected_binding_ids?: string[];
      dependency_plan?: ModelDependencyPlan;
      dependency_status?: ModelDependencyStatus;
      selectionMode?: 'library' | 'manual';
      inference_settings?: InferenceParamSchema[];
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelPath = $state(data.modelPath || '');
  let selectionMode = $state<'library' | 'manual'>(data.selectionMode || 'library');
  let availableModels: PortOption[] = $state([]);
  let isLoading = $state(false);
  let libraryAvailable = $state(true);
  let searchQuery = $state('');
  let isDependencyActionRunning = $state(false);
  let dependencyPlan = $state<ModelDependencyPlan | null>(data.dependency_plan ?? null);
  let dependencyStatus = $state<ModelDependencyStatus | null>(data.dependency_status ?? null);
  let selectedBindingIds = $state<string[]>(Array.isArray(data.selected_binding_ids) ? data.selected_binding_ids : []);

  let filteredModels = $derived(
    searchQuery
      ? availableModels.filter((m) => {
          const q = searchQuery.toLowerCase();
          return (
            m.label.toLowerCase().includes(q) ||
            (m.description?.toLowerCase().includes(q) ?? false)
          );
        })
      : availableModels,
  );

  const bindingStatusById = $derived.by(() => {
    const map = new Map<string, ModelDependencyBindingStatus>();
    for (const row of dependencyStatus?.bindings ?? []) {
      map.set(row.bindingId, row);
    }
    return map;
  });

  onMount(async () => {
    await loadModels();
    // Re-sync inference settings when model already selected (e.g. workflow reload)
    if (data.modelPath && availableModels.length > 0) {
      const match = availableModels.find((m) => String(m.value) === data.modelPath);
      if (match) {
        const settings = (match.metadata?.inference_settings ?? []) as InferenceParamSchema[];
        if (settings.length > 0) {
          const existingSettings = Array.isArray(data.inference_settings)
            ? data.inference_settings
            : [];
          if (existingSettings.length === 0) {
            updateNodeData(id, {
              modelName: data.modelName || match.label,
              inference_settings: settings,
            });
          }
          syncInferencePorts(id, settings);
          syncExpandPorts(id, settings);
        }
        if (!data.model_id && typeof match.metadata?.id === 'string') {
          const reviewReasons = Array.isArray(match.metadata?.review_reasons)
            ? (match.metadata?.review_reasons as string[])
            : [];
          const dependencyBindings = Array.isArray(match.metadata?.dependency_bindings)
            ? (match.metadata?.dependency_bindings as ModelDependencyBinding[])
            : [];
          const taskTypePrimary = match.metadata?.task_type_primary as string | undefined;
          updateNodeData(id, {
            model_id: match.metadata.id,
            model_type: match.metadata?.model_type,
            task_type_primary: taskTypePrimary,
            backend_key: inferBackendKeyFromTask(taskTypePrimary),
            platform_context: detectPlatformContext(),
            dependency_bindings: dependencyBindings,
            review_reasons: reviewReasons,
          });
        }
      }
    }
    if (data.modelPath) {
      await resolveDependencyPlan();
      await refreshDependencyStatus();
    }
  });

  async function loadModels() {
    isLoading = true;
    try {
      const result = await invoke<PortOptionsResult>('query_port_options', {
        nodeType: 'puma-lib',
        portId: 'model_path',
      });
      availableModels = result.options;
      libraryAvailable = result.options.length > 0;
      if (!libraryAvailable) {
        selectionMode = 'manual';
      }
    } catch {
      libraryAvailable = false;
      selectionMode = 'manual';
    } finally {
      isLoading = false;
    }
  }

  function inferNodeType(): string {
    return data.task_type_primary === 'text-to-audio' ? 'audio-generation' : 'pytorch-inference';
  }

  function inferBackendKeyFromTask(taskTypePrimary?: string): string {
    const task = (taskTypePrimary ?? '').toLowerCase();
    if (task === 'text-to-audio' || task === 'audio-to-text') {
      return 'stable_audio';
    }
    return 'pytorch';
  }

  function inferBackendKey(): string {
    const explicit = ((data.backend_key as string | undefined) ?? '').trim();
    if (explicit.length > 0) {
      return explicit;
    }
    return inferBackendKeyFromTask(data.task_type_primary as string | undefined);
  }

  function detectPlatformContext(): Record<string, string> {
    const ua = navigator.userAgent.toLowerCase();
    let os = 'unknown';
    if (ua.includes('linux')) {
      os = 'linux';
    } else if (ua.includes('mac')) {
      os = 'macos';
    } else if (ua.includes('win')) {
      os = 'windows';
    }

    let arch = 'unknown';
    const platform = navigator.platform?.toLowerCase() ?? '';
    if (platform.includes('x86_64') || platform.includes('x64') || platform.includes('win64')) {
      arch = 'x86_64';
    } else if (platform.includes('arm64') || platform.includes('aarch64')) {
      arch = 'arm64';
    }

    return { os, arch };
  }

  function dependencyRequestPayload() {
    const platformContext =
      (data.platform_context as Record<string, string> | undefined) ?? detectPlatformContext();
    return {
      nodeType: inferNodeType(),
      modelPath,
      modelId: (data.model_id as string | undefined) ?? undefined,
      modelType: (data.model_type as string | undefined) ?? undefined,
      taskTypePrimary: (data.task_type_primary as string | undefined) ?? undefined,
      backendKey: inferBackendKey(),
      platformContext,
      selectedBindingIds,
    };
  }

  function persistBindingSelection() {
    updateNodeData(id, {
      selected_binding_ids: selectedBindingIds,
    });
  }

  function applyPlan(plan: ModelDependencyPlan) {
    dependencyPlan = plan;

    const incoming = Array.isArray(plan.selectedBindingIds) ? plan.selectedBindingIds : [];
    if (incoming.length > 0) {
      selectedBindingIds = incoming;
      persistBindingSelection();
    } else if (selectedBindingIds.length === 0) {
      selectedBindingIds = (plan.bindings ?? []).map((b) => b.bindingId);
      persistBindingSelection();
    }

    updateNodeData(id, {
      dependency_plan: plan,
      dependency_plan_id: plan.planId,
      selected_binding_ids: selectedBindingIds,
    });
  }

  async function resolveDependencyPlan() {
    if (!modelPath) return;
    isDependencyActionRunning = true;
    try {
      const plan = await invoke<ModelDependencyPlan>('resolve_model_dependency_plan', dependencyRequestPayload());
      applyPlan(plan);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      dependencyPlan = {
        state: 'failed',
        message,
      };
      updateNodeData(id, { dependency_plan: dependencyPlan });
    } finally {
      isDependencyActionRunning = false;
    }
  }

  async function refreshDependencyStatus() {
    if (!modelPath) return;
    isDependencyActionRunning = true;
    try {
      const status = await invoke<ModelDependencyStatus>('get_model_dependency_status', dependencyRequestPayload());
      dependencyStatus = status;
      updateNodeData(id, { dependency_status: status });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      dependencyStatus = {
        state: 'failed',
        message,
      };
      updateNodeData(id, { dependency_status: dependencyStatus });
    } finally {
      isDependencyActionRunning = false;
    }
  }

  async function checkDependencies() {
    if (!modelPath) return;
    isDependencyActionRunning = true;
    try {
      const status = await invoke<ModelDependencyStatus>('check_model_dependencies', dependencyRequestPayload());
      dependencyStatus = status;
      updateNodeData(id, { dependency_status: status });
    } finally {
      isDependencyActionRunning = false;
    }
  }

  async function installDependencies() {
    if (!modelPath) return;
    isDependencyActionRunning = true;
    try {
      const result = await invoke<ModelDependencyInstallResult>(
        'install_model_dependencies',
        dependencyRequestPayload(),
      );
      dependencyStatus = {
        state: result.state,
        code: result.code,
        message: result.message,
        reviewReasons: result.reviewReasons,
        planId: result.planId,
        bindings: result.bindings,
      };
      updateNodeData(id, { dependency_status: dependencyStatus });
      await refreshDependencyStatus();
    } finally {
      isDependencyActionRunning = false;
    }
  }

  function handleModelSelect(e: Event) {
    const target = e.target as HTMLSelectElement;
    const selected = availableModels.find((m) => String(m.value) === target.value);
    if (selected) {
      modelPath = String(selected.value);
      const settings = (selected.metadata?.inference_settings ?? []) as InferenceParamSchema[];
      const dependencyBindings = Array.isArray(selected.metadata?.dependency_bindings)
        ? (selected.metadata?.dependency_bindings as ModelDependencyBinding[])
        : [];
      const reviewReasons = Array.isArray(selected.metadata?.review_reasons)
        ? (selected.metadata?.review_reasons as string[])
        : [];
      const taskTypePrimary = selected.metadata?.task_type_primary as string | undefined;
      const backendKey = inferBackendKeyFromTask(taskTypePrimary);

      selectedBindingIds = [];
      updateNodeData(id, {
        modelPath,
        modelName: selected.label,
        model_id: selected.metadata?.id,
        model_type: selected.metadata?.model_type,
        task_type_primary: taskTypePrimary,
        backend_key: backendKey,
        platform_context: detectPlatformContext(),
        dependency_bindings: dependencyBindings,
        review_reasons: reviewReasons,
        selected_binding_ids: selectedBindingIds,
        dependency_plan: null,
        dependency_status: null,
        selectionMode: 'library',
        inference_settings: settings,
      });

      if (settings.length > 0) {
        syncInferencePorts(id, settings);
        syncExpandPorts(id, settings);
      }

      resolveDependencyPlan().then(refreshDependencyStatus).catch(console.error);
    }
  }

  function handleManualInput(e: Event) {
    const target = e.target as HTMLInputElement;
    modelPath = target.value;
    selectedBindingIds = [];
    updateNodeData(id, {
      modelPath,
      selectionMode: 'manual',
      model_id: undefined,
      model_type: undefined,
      task_type_primary: undefined,
      backend_key: undefined,
      platform_context: undefined,
      dependency_bindings: [],
      review_reasons: [],
      dependency_plan_id: undefined,
      dependency_plan: null,
      dependency_status: null,
      selected_binding_ids: selectedBindingIds,
    });
    dependencyPlan = null;
    dependencyStatus = null;
  }

  async function browseForModel() {
    try {
      const result = await open({
        title: 'Select AI Model File',
        filters: [
          { name: 'GGUF Models', extensions: ['gguf'] },
          { name: 'All Files', extensions: ['*'] },
        ],
        multiple: false,
        directory: false,
      });

      if (result && typeof result === 'string') {
        modelPath = result;
        selectedBindingIds = [];
        updateNodeData(id, {
          modelPath,
          selectionMode: 'manual',
          model_id: undefined,
          model_type: undefined,
          task_type_primary: undefined,
          backend_key: undefined,
          platform_context: undefined,
          dependency_bindings: [],
          review_reasons: [],
          dependency_plan_id: undefined,
          dependency_plan: null,
          dependency_status: null,
          selected_binding_ids: selectedBindingIds,
        });
        dependencyPlan = null;
        dependencyStatus = null;
      }
    } catch (error) {
      console.error('File picker error:', error);
    }
  }

  function switchMode(mode: 'library' | 'manual') {
    selectionMode = mode;
    updateNodeData(id, { selectionMode: mode });
  }

  function toggleBinding(bindingId: string, required: boolean) {
    if (required) return;
    if (selectedBindingIds.includes(bindingId)) {
      selectedBindingIds = selectedBindingIds.filter((id) => id !== bindingId);
    } else {
      selectedBindingIds = [...selectedBindingIds, bindingId];
    }
    persistBindingSelection();
  }

  function deriveDisplayState(): DependencyState | null {
    if (dependencyStatus) return dependencyStatus.state;
    if (dependencyPlan) return dependencyPlan.state;
    return null;
  }

  const dependencyBadge = $derived.by(() => {
    const state = deriveDisplayState();
    if (!state) return { label: 'deps unknown', className: 'text-neutral-400 border-neutral-700' };
    switch (state) {
      case 'ready':
        return { label: 'deps ready', className: 'text-emerald-400 border-emerald-500/40' };
      case 'missing':
        return { label: 'deps missing', className: 'text-amber-400 border-amber-500/40' };
      case 'installing':
        return { label: 'deps installing', className: 'text-sky-400 border-sky-500/40' };
      case 'manual_intervention_required':
        return { label: 'manual review', className: 'text-rose-400 border-rose-500/40' };
      case 'unknown_profile':
        return { label: 'unknown profile', className: 'text-violet-400 border-violet-500/40' };
      case 'profile_conflict':
        return { label: 'profile conflict', className: 'text-orange-400 border-orange-500/40' };
      case 'required_binding_omitted':
        return { label: 'binding omitted', className: 'text-fuchsia-400 border-fuchsia-500/40' };
      default:
        return { label: 'deps failed', className: 'text-red-400 border-red-500/40' };
    }
  });
</script>

<div class="puma-lib-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-amber-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Puma-Lib'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-2">
        {#if modelPath}
          <div class="rounded border px-2 py-1 text-[10px] {dependencyBadge.className}">
            <div class="flex items-center gap-2">
              <span>{dependencyBadge.label}</span>
              <button
                type="button"
                class="ml-auto text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
                onclick={resolveDependencyPlan}
                disabled={isDependencyActionRunning}
              >
                Plan
              </button>
              <button
                type="button"
                class="text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
                onclick={checkDependencies}
                disabled={isDependencyActionRunning}
              >
                Check
              </button>
              <button
                type="button"
                class="text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
                onclick={installDependencies}
                disabled={isDependencyActionRunning}
              >
                Install
              </button>
            </div>
            {#if dependencyStatus?.message || dependencyPlan?.message}
              <div class="mt-1 text-[9px] text-neutral-500 truncate" title={dependencyStatus?.message ?? dependencyPlan?.message}>
                {dependencyStatus?.message ?? dependencyPlan?.message}
              </div>
            {/if}
            {#if (dependencyStatus?.reviewReasons?.length ?? 0) > 0 || (dependencyPlan?.reviewReasons?.length ?? 0) > 0}
              <div class="mt-1 text-[9px] text-rose-300">
                {(dependencyStatus?.reviewReasons ?? dependencyPlan?.reviewReasons ?? []).join(', ')}
              </div>
            {/if}
          </div>
        {/if}

        {#if modelPath && (dependencyPlan?.bindings?.length ?? 0) > 0}
          <div class="rounded border border-neutral-700 px-2 py-1 text-[10px] space-y-1">
            {#each dependencyPlan?.bindings ?? [] as binding}
              {@const required = (dependencyPlan?.requiredBindingIds ?? []).includes(binding.bindingId)}
              {@const row = bindingStatusById.get(binding.bindingId)}
              <div class="rounded border border-neutral-800 px-2 py-1">
                <div class="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={selectedBindingIds.includes(binding.bindingId)}
                    disabled={required || isDependencyActionRunning}
                    onchange={() => toggleBinding(binding.bindingId, required)}
                  />
                  <span class="text-neutral-200 truncate" title={binding.bindingId}>{binding.bindingId}</span>
                  {#if required}
                    <span class="ml-auto text-[9px] text-amber-300">required</span>
                  {/if}
                </div>
                <div class="text-[9px] text-neutral-500 truncate" title={binding.profileId + ' v' + binding.profileVersion}>
                  {binding.profileId} v{binding.profileVersion}
                </div>
                {#if row}
                  <div class="text-[9px] text-neutral-400">{row.state}</div>
                  {#if (row.missingComponents?.length ?? 0) > 0}
                    <div class="text-[9px] text-amber-300 truncate" title={row.missingComponents?.join(', ')}>
                      missing: {row.missingComponents?.join(', ')}
                    </div>
                  {/if}
                  {#if (row.failedComponents?.length ?? 0) > 0}
                    <div class="text-[9px] text-rose-300 truncate" title={row.failedComponents?.join(', ')}>
                      failed: {row.failedComponents?.join(', ')}
                    </div>
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if libraryAvailable}
          <div class="flex gap-1 text-[10px]">
            <button
              type="button"
              class="px-2 py-0.5 rounded transition-colors {selectionMode === 'library'
                ? 'bg-amber-600/30 text-amber-400'
                : 'text-neutral-500 hover:text-neutral-400'}"
              onclick={() => switchMode('library')}
            >
              Library
            </button>
            <button
              type="button"
              class="px-2 py-0.5 rounded transition-colors {selectionMode === 'manual'
                ? 'bg-amber-600/30 text-amber-400'
                : 'text-neutral-500 hover:text-neutral-400'}"
              onclick={() => switchMode('manual')}
            >
              Manual
            </button>
            {#if selectionMode === 'library'}
              <button
                type="button"
                class="ml-auto text-neutral-500 hover:text-neutral-400"
                onclick={loadModels}
                disabled={isLoading}
              >
                {isLoading ? '...' : 'Refresh'}
              </button>
            {/if}
          </div>
        {/if}

        {#if selectionMode === 'library'}
          <div class="space-y-1">
            {#if availableModels.length > 6}
              <input
                type="text"
                class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-300 focus:outline-none focus:border-amber-500"
                placeholder="Filter models..."
                bind:value={searchQuery}
              />
            {/if}
            <select
              class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-amber-500"
              style="color-scheme: dark;"
              onchange={handleModelSelect}
              value={modelPath}
              disabled={isLoading}
            >
              <option value="" class="bg-neutral-900 text-neutral-500">
                {isLoading ? 'Loading...' : 'Select a model'}
              </option>
              {#each filteredModels as model}
                <option value={String(model.value)} class="bg-neutral-900 text-neutral-200">
                  {model.label}
                </option>
              {/each}
            </select>
          </div>

          {#if modelPath}
            <div class="text-[10px] text-neutral-500 truncate" title={modelPath}>
              {modelPath.split('/').pop()}
            </div>
          {/if}
        {:else}
          <div class="space-y-1">
            <label class="text-xs text-neutral-400">Model Path</label>
            <div class="flex gap-1">
              <input
                type="text"
                class="flex-1 bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-amber-500 font-mono truncate"
                placeholder="/path/to/model.gguf"
                value={modelPath}
                oninput={handleManualInput}
              />
              <button
                type="button"
                class="px-2 py-1 bg-amber-600 hover:bg-amber-500 text-white text-xs rounded flex-shrink-0"
                onclick={browseForModel}
              >
                Browse
              </button>
            </div>
            {#if modelPath}
              <div class="text-[10px] text-neutral-500 truncate" title={modelPath}>
                {modelPath.split('/').pop()}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .puma-lib-node-wrapper :global(.base-node) {
    border-color: rgba(217, 119, 6, 0.5);
  }

  .puma-lib-node-wrapper :global(.node-header) {
    background-color: rgba(217, 119, 6, 0.2);
    border-color: rgba(217, 119, 6, 0.3);
  }
</style>
