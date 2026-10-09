<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import JsonFilterPathEditor from './JsonFilterPathEditor.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  interface Props {
    id: string;
    data: { definition?: NodeDefinition; label?: string; path?: unknown };
    selected?: boolean;
  }
  let { id, data, selected = false }: Props = $props();
</script>

<BaseNode {id} {data} {selected}>
  <JsonFilterPathEditor {id} {data} />
</BaseNode>
