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

  // Get execution info (new format with state and errorMessage)
  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');

  // Check if model_path input is connected
  let isModelConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'model_path')
  );

  let upstreamDependencyNode = $derived.by(() => {
    const edge = $edges.find((e) => e.target === id && e.targetHandle === 'environment_ref');
    if (!edge) return null;
    return $nodes.find((n) => n.id === edge.source) ?? null;
  });

  let dependencyState = $derived(
    (upstreamDependencyNode?.data?.dependency_status as { state?: string } | undefined)?.state ?? null
  );
  let dependencyCode = $derived(
    (upstreamDependencyNode?.data?.dependency_status as { code?: string } | undefined)?.code ?? null
  );

  // PyTorch orange
  const nodeColor = '#ee4c2c';

  let statusText = $derived(
    {
      idle: 'Ready',
      running: 'Generating...',
      success: 'Complete',
      error: 'Error',
    }[executionState]
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

<div class="pytorch-node-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12.005 1.401l-5.36 5.36a7.58 7.58 0 000 10.72 7.58 7.58 0 0010.72 0 7.58 7.58 0 000-10.72l-1.07 1.07a6.06 6.06 0 010 8.58 6.06 6.06 0 01-8.58 0 6.06 6.06 0 010-8.58l3.79-3.79.53-.53V1.4zm2.65 3.17a1.14 1.14 0 100 2.28 1.14 1.14 0 000-2.28z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'PyTorch Inference'}</span>
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
  .pytorch-node-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .pytorch-node-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>
