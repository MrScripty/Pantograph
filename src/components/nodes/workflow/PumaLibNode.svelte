<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition, PortOption, PortOptionsResult } from '../../../services/workflow/types';
  import {
    updateNodeData,
    syncInferencePorts,
    syncExpandPorts,
  } from '../../../stores/workflowStore';
  import { invoke } from '@tauri-apps/api/core';

  interface InferenceParamSchema {
    key: string;
    label: string;
    param_type: 'Number' | 'Integer' | 'String' | 'Boolean';
    default: unknown;
    description?: string;
    constraints?: { min?: number; max?: number; allowed_values?: unknown[] };
  }

  type DependencyValidationState =
    | 'resolved'
    | 'unknown_profile'
    | 'invalid_profile'
    | 'profile_conflict';

  interface DependencyValidationError {
    code: string;
    scope: 'top_level' | 'binding';
    binding_id?: string;
    field?: string;
    message: string;
  }

  interface ModelDependencyRequirement {
    kind: string;
    name: string;
    exact_pin: string;
  }

  interface ModelDependencyBinding {
    binding_id: string;
    profile_id: string;
    profile_version: number;
    profile_hash?: string;
    backend_key?: string;
    platform_selector?: string;
    environment_kind?: string;
    env_id?: string;
    validation_state: DependencyValidationState;
    validation_errors: DependencyValidationError[];
    requirements: ModelDependencyRequirement[];
  }

  interface ModelDependencyRequirements {
    model_id: string;
    platform_key: string;
    backend_key?: string;
    dependency_contract_version: number;
    validation_state: DependencyValidationState;
    validation_errors: DependencyValidationError[];
    bindings: ModelDependencyBinding[];
    selected_binding_ids: string[];
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
      recommended_backend?: string;
      runtime_engine_hints?: string[];
      requires_custom_code?: boolean;
      custom_code_sources?: string[];
      platform_context?: Record<string, string>;
      dependency_requirements_id?: string;
      dependency_bindings?: ModelDependencyBinding[];
      review_reasons?: string[];
      selected_binding_ids?: string[];
      dependency_requirements?: ModelDependencyRequirements;
      inference_settings?: InferenceParamSchema[];
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let modelPath = $state('');
  let modelId = $state<string | undefined>(undefined);
  let modelType = $state<string | undefined>(undefined);
  let taskTypePrimary = $state<string | undefined>(undefined);
  let backendKey = $state<string | undefined>(undefined);
  let recommendedBackend = $state<string | undefined>(undefined);
  let dependencyBindings = $state<ModelDependencyBinding[]>([]);
  let platformContext = $state<Record<string, string> | undefined>(undefined);
  let availableModels: PortOption[] = $state([]);
  let isLoading = $state(false);
  let loadError = $state<string | null>(null);
  let searchQuery = $state('');
  let isDependencyActionRunning = $state(false);
  let dependencyRequirements = $state<ModelDependencyRequirements | null>(null);
  let requirementsMessage = $state<string | null>(null);
  let requirementsCode = $state<string | null>(null);
  let selectedBindingIds = $state<string[]>([]);

  $effect(() => {
    modelPath = data.modelPath || '';
    modelId = data.model_id as string | undefined;
    modelType = data.model_type as string | undefined;
    taskTypePrimary = data.task_type_primary as string | undefined;
    backendKey = data.backend_key as string | undefined;
    recommendedBackend = data.recommended_backend as string | undefined;
    dependencyBindings = Array.isArray(data.dependency_bindings)
      ? (data.dependency_bindings as ModelDependencyBinding[])
      : [];
    platformContext = data.platform_context as Record<string, string> | undefined;
    dependencyRequirements = (data.dependency_requirements as ModelDependencyRequirements | null) ?? null;
    requirementsMessage = null;
    requirementsCode = null;
    selectedBindingIds = Array.isArray(data.selected_binding_ids) ? data.selected_binding_ids : [];
  });

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

  function normalizeBackendKey(value?: string): string | undefined {
    const token = (value ?? '').trim().toLowerCase();
    if (!token) {
      return undefined;
    }

    switch (token) {
      case 'llama.cpp':
      case 'llama-cpp':
      case 'llamacpp':
        return 'llamacpp';
      case 'onnx-runtime':
      case 'onnxruntime':
      case 'onnx_runtime':
        return 'onnxruntime';
      case 'torch':
      case 'pytorch':
        return 'pytorch';
      case 'stable-audio':
      case 'stable_audio':
        return 'stable_audio';
      default:
        return token;
    }
  }

  function uniqueBindingBackend(bindings: ModelDependencyBinding[]): string | undefined {
    const unique = new Set(
      bindings
        .map((binding) => normalizeBackendKey(binding.backend_key))
        .filter((value): value is string => typeof value === 'string' && value.length > 0),
    );
    if (unique.size === 1) {
      return [...unique][0];
    }
    return undefined;
  }

  function selectedBindingBackend(): string | undefined {
    const sourceBindings = dependencyRequirements?.bindings ?? [];
    if (sourceBindings.length === 0) {
      return undefined;
    }

    const selected = selectedBindingIds.length > 0
      ? sourceBindings.filter((binding) => selectedBindingIds.includes(binding.binding_id))
      : sourceBindings;
    return uniqueBindingBackend(selected);
  }

  onMount(async () => {
    await loadModels();

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
          const nextDependencyBindings = Array.isArray(match.metadata?.dependency_bindings)
            ? (match.metadata?.dependency_bindings as ModelDependencyBinding[])
            : [];
          const taskTypePrimary = match.metadata?.task_type_primary as string | undefined;
          const recommendedBackend = normalizeBackendKey(
            match.metadata?.recommended_backend as string | undefined,
          );
          updateNodeData(id, {
            model_id: match.metadata.id,
            model_type: match.metadata?.model_type,
            task_type_primary: taskTypePrimary,
            backend_key:
              uniqueBindingBackend(nextDependencyBindings) ??
              recommendedBackend ??
              inferBackendKeyFromTask(taskTypePrimary),
            recommended_backend: recommendedBackend,
            runtime_engine_hints: match.metadata?.runtime_engine_hints,
            requires_custom_code: match.metadata?.requires_custom_code,
            custom_code_sources: match.metadata?.custom_code_sources,
            platform_context: detectPlatformContext(),
            dependency_bindings: nextDependencyBindings,
            review_reasons: reviewReasons,
          });
        }
      }
    }

    if (data.modelPath) {
      await resolveDependencyRequirements();
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
      loadError = null;
    } catch (error) {
      loadError = error instanceof Error ? error.message : 'Failed to load models from pumas library';
    } finally {
      isLoading = false;
    }
  }

  function inferNodeType(): string {
    const backend = inferBackendKey();
    if (backend === 'onnxruntime') {
      return 'onnx-inference';
    }
    if (taskTypePrimary === 'text-to-audio') {
      return 'audio-generation';
    }
    return 'pytorch-inference';
  }

  function inferBackendKeyFromTask(taskTypePrimary?: string): string {
    const task = (taskTypePrimary ?? '').toLowerCase();
    if (task === 'text-to-audio' || task === 'audio-to-text') {
      return 'stable_audio';
    }
    return 'pytorch';
  }

  function inferBackendKey(): string {
    const explicit = normalizeBackendKey(backendKey);
    if (explicit) {
      return explicit;
    }
    const selectedBinding = selectedBindingBackend();
    if (selectedBinding) {
      return selectedBinding;
    }
    const recommended = normalizeBackendKey(recommendedBackend);
    if (recommended) {
      return recommended;
    }
    const bindingDefault = uniqueBindingBackend(dependencyBindings);
    if (bindingDefault) {
      return bindingDefault;
    }
    return inferBackendKeyFromTask(taskTypePrimary);
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
    const resolvedPlatformContext = platformContext ?? detectPlatformContext();
    return {
      nodeType: inferNodeType(),
      modelPath,
      modelId: modelId ?? undefined,
      modelType: modelType ?? undefined,
      taskTypePrimary: taskTypePrimary ?? undefined,
      backendKey: inferBackendKey(),
      platformContext: resolvedPlatformContext,
      selectedBindingIds,
    };
  }

  function persistBindingSelection() {
    updateNodeData(id, {
      selected_binding_ids: selectedBindingIds,
    });
  }

  function applyRequirements(requirements: ModelDependencyRequirements) {
    dependencyRequirements = requirements;
    requirementsMessage = null;
    requirementsCode = null;

    const incoming = Array.isArray(requirements.selected_binding_ids)
      ? requirements.selected_binding_ids
      : [];
    if (incoming.length > 0) {
      selectedBindingIds = incoming;
      persistBindingSelection();
    } else if (selectedBindingIds.length === 0) {
      selectedBindingIds = requirements.bindings.map((b) => b.binding_id);
      persistBindingSelection();
    }

    updateNodeData(id, {
      dependency_requirements: requirements,
      dependency_requirements_id: requirements.model_id,
      selected_binding_ids: selectedBindingIds,
    });
  }

  async function resolveDependencyRequirements() {
    if (!modelPath) return;
    isDependencyActionRunning = true;
    try {
      const requirements = await invoke<ModelDependencyRequirements>(
        'resolve_model_dependency_requirements',
        dependencyRequestPayload(),
      );
      applyRequirements(requirements);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      requirementsCode = 'requirements_resolution_failed';
      requirementsMessage = message;
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
      const nextDependencyBindings = Array.isArray(selected.metadata?.dependency_bindings)
        ? (selected.metadata?.dependency_bindings as ModelDependencyBinding[])
        : [];
      const reviewReasons = Array.isArray(selected.metadata?.review_reasons)
        ? (selected.metadata?.review_reasons as string[])
        : [];
      const nextTaskTypePrimary = selected.metadata?.task_type_primary as string | undefined;
      const nextRecommendedBackend = normalizeBackendKey(
        selected.metadata?.recommended_backend as string | undefined,
      );
      const nextBackendKey =
        uniqueBindingBackend(nextDependencyBindings) ??
        nextRecommendedBackend ??
        inferBackendKeyFromTask(nextTaskTypePrimary);
      const nextPlatformContext = detectPlatformContext();

      modelId = selected.metadata?.id as string | undefined;
      modelType = selected.metadata?.model_type as string | undefined;
      taskTypePrimary = nextTaskTypePrimary;
      backendKey = nextBackendKey;
      recommendedBackend = nextRecommendedBackend;
      dependencyBindings = nextDependencyBindings;
      platformContext = nextPlatformContext;
      selectedBindingIds = [];
      updateNodeData(id, {
        modelPath,
        modelName: selected.label,
        model_id: modelId,
        model_type: modelType,
        task_type_primary: taskTypePrimary,
        backend_key: backendKey,
        recommended_backend: recommendedBackend,
        runtime_engine_hints: selected.metadata?.runtime_engine_hints,
        requires_custom_code: selected.metadata?.requires_custom_code,
        custom_code_sources: selected.metadata?.custom_code_sources,
        platform_context: platformContext,
        dependency_bindings: nextDependencyBindings,
        review_reasons: reviewReasons,
        selected_binding_ids: selectedBindingIds,
        dependency_requirements: null,
        inference_settings: settings,
      });

      if (settings.length > 0) {
        syncInferencePorts(id, settings);
        syncExpandPorts(id, settings);
      }

      resolveDependencyRequirements().catch(console.error);
    }
  }

  function dependencyTokenLabel(value: string): string {
    return value.replaceAll('_', ' ');
  }

  function dependencyCodeLabel(code?: string): string | null {
    switch (code) {
      case 'requirements_missing':
        return 'requirements missing';
      case 'dependency_install_failed':
      case 'dependency_check_failed':
        return 'dependency check failed';
      case 'profile_conflict':
        return 'profile conflict';
      case 'unknown_profile':
        return 'unknown profile';
      case 'invalid_profile':
        return 'invalid profile';
      default:
        return code ? dependencyTokenLabel(code) : null;
    }
  }

  function deriveDisplayState(): string | null {
    if (requirementsCode) return 'requirements_error';
    if (!dependencyRequirements) return null;
    switch (dependencyRequirements.validation_state) {
      case 'resolved':
        return 'requirements_resolved';
      case 'unknown_profile':
        return 'requirements_unresolved';
      default:
        return 'requirements_invalid';
    }
  }

  function deriveDisplayCode(): string | null {
    if (requirementsCode) return requirementsCode;
    return dependencyRequirements?.validation_errors?.[0]?.code ?? null;
  }

  function deriveDisplayMessage(): string | null {
    if (requirementsMessage) return requirementsMessage;
    return dependencyRequirements?.validation_errors?.[0]?.message ?? null;
  }

  const dependencyBadge = $derived.by(() => {
    const state = deriveDisplayState();
    if (!state) {
      return { label: 'requirements unknown', className: 'text-neutral-400 border-neutral-700' };
    }
    switch (state) {
      case 'requirements_resolved':
        return { label: 'requirements resolved', className: 'text-cyan-300 border-cyan-500/40' };
      case 'requirements_unresolved':
        return { label: 'requirements unresolved', className: 'text-violet-400 border-violet-500/40' };
      case 'requirements_invalid':
        return { label: 'requirements invalid', className: 'text-orange-400 border-orange-500/40' };
      case 'requirements_error':
        return { label: 'requirements error', className: 'text-red-400 border-red-500/40' };
      default:
        return {
          label: `requirements ${dependencyTokenLabel(state)}`,
          className: 'text-neutral-300 border-neutral-600/50',
        };
    }
  });

  const dependencyCodeText = $derived.by(() => dependencyCodeLabel(deriveDisplayCode() ?? undefined));
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

    <div class="space-y-2">
        {#if modelPath}
          <div class="rounded border px-2 py-1 text-[10px] {dependencyBadge.className}">
            <div class="flex items-center gap-2">
              <span>{dependencyBadge.label}</span>
              <button
                type="button"
                class="ml-auto text-neutral-400 hover:text-neutral-200 disabled:opacity-50"
                onclick={resolveDependencyRequirements}
                disabled={isDependencyActionRunning}
              >
                Resolve
              </button>
            </div>
            {#if deriveDisplayMessage()}
              <div class="mt-1 text-[9px] text-neutral-500 truncate" title={deriveDisplayMessage() ?? undefined}>
                {deriveDisplayMessage()}
              </div>
            {/if}
            {#if dependencyCodeText}
              <div class="mt-1 text-[9px] text-amber-300 truncate" title={deriveDisplayCode() ?? undefined}>
                code: {dependencyCodeText}
              </div>
            {/if}
          </div>
        {/if}

        {#if modelPath && (dependencyRequirements?.bindings?.length ?? 0) > 0}
          <div class="rounded border border-neutral-700 px-2 py-1 text-[10px] space-y-1">
            {#each dependencyRequirements?.bindings ?? [] as binding (binding.binding_id)}
              <div class="rounded border border-neutral-800 px-2 py-1">
                <div class="flex items-center gap-2">
                  <span class="text-neutral-200 truncate" title={binding.binding_id}>{binding.binding_id}</span>
                </div>
                <div class="text-[9px] text-neutral-500 truncate" title={binding.profile_id + ' v' + binding.profile_version}>
                  {binding.profile_id} v{binding.profile_version}
                </div>
                <div class="text-[9px] text-neutral-400">validation: {dependencyTokenLabel(binding.validation_state)}</div>
                {#if binding.validation_errors.length > 0}
                  <div class="text-[9px] text-amber-300 truncate" title={binding.validation_errors[0].message}>
                    {dependencyCodeLabel(binding.validation_errors[0].code) ?? binding.validation_errors[0].code}
                  </div>
                {/if}
                {#if binding.requirements.length > 0}
                  <div class="text-[9px] text-neutral-300 truncate" title={binding.requirements.map((r) => `${r.name}${r.exact_pin}`).join(', ')}>
                    requirements: {binding.requirements.map((r) => `${r.name}${r.exact_pin}`).join(', ')}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        <div class="flex justify-end text-[10px]">
          <button
            type="button"
            class="text-neutral-500 hover:text-neutral-400"
            onclick={loadModels}
            disabled={isLoading}
          >
            {isLoading ? '...' : 'Refresh'}
          </button>
        </div>

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
            {#each filteredModels as model (String(model.value))}
              <option value={String(model.value)} class="bg-neutral-900 text-neutral-200">
                {model.label}
              </option>
            {/each}
          </select>
          {#if loadError}
            <div class="text-[10px] text-red-400 truncate" title={loadError}>
              Failed to load models from pumas library
            </div>
          {:else if !isLoading && availableModels.length === 0}
            <div class="text-[10px] text-neutral-500">
              No models found in pumas library
            </div>
          {/if}
        </div>

        {#if modelPath}
          <div class="text-[10px] text-neutral-500 truncate" title={modelPath}>
            {modelPath.split('/').pop()}
          </div>
        {/if}
    </div>
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
