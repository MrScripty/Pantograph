<script lang="ts">
  import type { Snippet } from 'svelte';
  let { header, children }: { header?: Snippet; children?: Snippet } = $props();
</script>

<section class="base-node">
  <header class="node-header">{@render header?.()}</header>
  {@render children?.()}
</section>
