<script lang="ts">
  import OrchestrationBaseNode from './OrchestrationBaseNode.svelte';

  interface Props {
    id: string;
    data: { label?: string; config?: Record<string, unknown> };
  }

  let { id, data }: Props = $props();

  const inputHandles = [
    { id: 'a', label: 'A' },
    { id: 'b', label: 'B' },
    { id: 'c', label: 'C' },
    { id: 'd', label: 'D' },
  ];
  const outputHandles = [{ id: 'next', label: 'Out' }];
</script>

<OrchestrationBaseNode
  {id}
  label="Merge"
  color="#06b6d4"
  {inputHandles}
  {outputHandles}
>
  {#snippet icon()}
    <svg viewBox="0 0 24 24" fill="currentColor">
      <path
        d="M17 20.41L18.41 19 15 15.59 13.59 17 17 20.41zM7.5 8H11v5.59L5.59 19 7 20.41l6-6V8h3.5L12 3.5 7.5 8z"
      />
    </svg>
  {/snippet}
</OrchestrationBaseNode>
