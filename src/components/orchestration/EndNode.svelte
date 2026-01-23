<script lang="ts">
  import OrchestrationBaseNode from './OrchestrationBaseNode.svelte';

  interface Props {
    id: string;
    data: { label?: string; config?: Record<string, unknown> };
  }

  let { id, data }: Props = $props();

  const inputHandles = [{ id: 'input', label: 'End' }];
</script>

<OrchestrationBaseNode {id} label="End" color="#ef4444" {inputHandles}>
  {#snippet icon()}
    <svg viewBox="0 0 24 24" fill="currentColor">
      <rect x="6" y="6" width="12" height="12" rx="1" />
    </svg>
  {/snippet}
</OrchestrationBaseNode>
