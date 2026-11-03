<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData, edges, nodes } from '../../../stores/workflowStore';
  import {
    findConnectedTargetPort,
    normalizePortDefaultValue,
  } from './primitiveInputMetadata';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      text?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  const DEFAULT_PLACEHOLDER = 'Enter text...';
  let text = $derived(data.text || '');



  // Check if the 'text' input is connected
  let isTextConnected = $derived(
    $edges.some((edge) => edge.target === id && edge.targetHandle === 'text')
  );

  let targetPort = $derived.by(() => findConnectedTargetPort(id, 'text', $nodes, $edges));
  let defaultText = $derived(normalizePortDefaultValue(targetPort?.default_value));

  $effect(() => {
    if (data.text !== undefined || typeof defaultText !== 'string') {
      return;
    }

    updateNodeData(id, { text: defaultText });
  });


  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    text = target.value;
    updateNodeData(id, { text });
  }
</script>

<div class="text-input-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Text Input'} />
    {/snippet}

    {#if isTextConnected}
      <div class="text-xs text-pg-text-muted italic py-1">
        Connected to external input
      </div>
    {:else}
      <div class="flex flex-col gap-1">
        {#if targetPort}
          <div class="text-[10px] text-pg-text-muted">{targetPort.label}</div>
        {/if}
        <textarea
          class="w-full bg-pg-panel-sunken border border-pg-border rounded px-2 py-1 text-sm text-pg-text resize-none"
          rows="3"
          placeholder={targetPort?.description || DEFAULT_PLACEHOLDER}
          value={text}
          oninput={handleInput}
        ></textarea>
      </div>
    {/if}
  </BaseNode>
</div>
