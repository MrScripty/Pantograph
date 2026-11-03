<script lang="ts">
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';

  let { label }: { label: string } = $props();
</script>

<NodeHeaderTitle {label} />
