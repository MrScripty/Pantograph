<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates, edges, nodes } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  let isModelConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'model_path'),
  );

  let upstreamDependencyNode = $derived.by(() => {
    const edge = $edges.find((e) => e.target === id && e.targetHandle === 'environment_ref');
    if (!edge) return null;
    return $nodes.find((n) => n.id === edge.source) ?? null;
  });

  let dependencyState = $derived(
    (upstreamDependencyNode?.data?.dependency_status as { state?: string } | undefined)?.state ??
      null,
  );
  let dependencyCode = $derived(
    (upstreamDependencyNode?.data?.dependency_status as { code?: string } | undefined)?.code ??
      null,
  );

  // ONNX blue
  const nodeColor = '#2563eb';

  let statusText = $derived(
    {
      idle: 'Ready',
      running: 'Generating...',
      success: 'Complete',
      error: 'Error',
    }[executionState],
  );

  function dependencyTokenLabel(value: string): string {
    return value.replaceAll('_', ' ');
  }

  let dependencyText = $derived.by(() => {
    if (dependencyCode === 'unpinned_dependency') {
      return 'pinning required';
    }
    if (dependencyCode === 'modality_resolution_unknown') {
      return 'modality unresolved';
    }
    switch (dependencyState) {
      case 'ready':
        return 'deps ready';
      case 'missing':
        return 'deps missing';
      case 'installing':
        return 'deps installing';
      case 'manual_intervention_required':
        return 'manual review';
      case 'unknown_profile':
        return 'unknown profile';
      case 'profile_conflict':
        return 'profile conflict';
      case 'required_binding_omitted':
        return 'binding omitted';
      case 'failed':
        return 'deps failed';
      default:
        return dependencyState ? `deps ${dependencyTokenLabel(dependencyState)}` : null;
    }
  });
</script>

<div class="onnx-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div
          class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0"
          style="background-color: {nodeColor}"
        >
          <svg class="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 24 24">
            <path d="M5 4h6v2H7v4H5V4zm8 0h6v6h-2V6h-4V4zM5 14h2v4h4v2H5v-6zm12 0h2v6h-6v-2h4v-4zM9 9h6v6H9V9z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'ONNX Inference'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-2">
        <div class="flex items-center gap-2 text-xs text-neutral-400">
          <span>{statusText}</span>
          {#if dependencyText}
            <span class="text-[10px] text-neutral-500">| {dependencyText}</span>
          {/if}
        </div>
        {#if !isModelConnected}
          <div class="text-[10px] text-amber-400">
            Connect a Puma-Lib node
          </div>
        {/if}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .onnx-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .onnx-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>
