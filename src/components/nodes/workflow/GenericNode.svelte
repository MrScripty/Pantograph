<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();
</script>

<!-- BaseNode's header glyph (with its screen-reader text and title) carries the
     execution state; category is not colour-coded on the canvas. -->
<div class="generic-node-wrapper">
  <BaseNode {id} {data} {selected} />
</div>
