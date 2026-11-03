<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      text?: string;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let text = $derived(data.text || '');
  let streamContent = $derived(data.streamContent || '');
  let displayText = $derived(streamContent || text);
  let isStreaming = $derived(executionState === 'running' && streamContent.length > 0);

  let outputContainer: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (outputContainer && streamContent) {
      outputContainer.scrollTop = outputContainer.scrollHeight;
    }
  });

</script>

<div class="output-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Text Output'} />
    {/snippet}

      {#if displayText}
        <div
          bind:this={outputContainer}
          class="copyable-output nodrag nopan nowheel p-2 bg-pg-panel-sunken rounded text-xs text-pg-text-soft max-h-32 overflow-y-auto whitespace-pre-wrap"
        >{displayText}{#if isStreaming}<span class="stream-caret" aria-hidden="true">▍</span>{/if}</div>
      {:else}
        <div class="text-xs text-pg-text-muted italic">
          No output yet
        </div>
      {/if}
  </BaseNode>
</div>

<style>
  .stream-caret {
    color: var(--pg-accent);
  }

  .copyable-output {
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
</style>
