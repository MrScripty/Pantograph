<script lang="ts">
  import { Chip } from '../workbench/ui';

  type BackendCapabilities = {
    vision: boolean;
    embeddings: boolean;
    gpu: boolean;
    device_selection: boolean;
    streaming: boolean;
    tool_calling: boolean;
  };

  type Props = {
    capabilities: BackendCapabilities;
  };

  let { capabilities }: Props = $props();
</script>

<div class="flex flex-wrap gap-1.5">
  {#if capabilities.vision}
    <Chip>Vision</Chip>
  {/if}
  {#if capabilities.embeddings}
    <Chip>Embeddings</Chip>
  {/if}
  {#if capabilities.gpu}
    <Chip>GPU</Chip>
  {/if}
  {#if capabilities.device_selection}
    <Chip>Device select</Chip>
  {/if}
  {#if capabilities.streaming}
    <Chip>Streaming</Chip>
  {/if}
  {#if capabilities.tool_calling}
    <Chip>Tools</Chip>
  {/if}
</div>
