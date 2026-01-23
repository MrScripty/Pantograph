<script lang="ts">
  import OrchestrationBaseNode from './OrchestrationBaseNode.svelte';

  interface Props {
    id: string;
    data: { label?: string; config?: Record<string, unknown> };
  }

  let { id, data }: Props = $props();

  const outputHandles = [{ id: 'next', label: 'Start' }];
</script>

<OrchestrationBaseNode {id} label="Start" color="#22c55e" {outputHandles}>
  {#snippet icon()}
    <svg viewBox="0 0 24 24" fill="currentColor">
      <path d="M8 5v14l11-7z" />
    </svg>
  {/snippet}
</OrchestrationBaseNode>
