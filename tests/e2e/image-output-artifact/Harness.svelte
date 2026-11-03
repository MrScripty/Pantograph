<script lang="ts">
  import ImageOutputNode from '../../../src/components/nodes/workflow/ImageOutputNode.svelte';
  import { nodeData } from './store';
</script>

<ImageOutputNode id="image-output-fixture" data={$nodeData} />
