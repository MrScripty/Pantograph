<script lang="ts">
  import ArchBaseNode from './ArchBaseNode.svelte';
  import type { ArchNodeCategory } from '../../../services/architecture/types';

  interface Props {
    id: string;
    data: {
      label: string;
      description?: string;
      filePath?: string;
      category: ArchNodeCategory;
      color: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();
</script>

<ArchBaseNode {id} {data} {selected} />
