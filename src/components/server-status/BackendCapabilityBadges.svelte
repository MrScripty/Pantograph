<script lang="ts">
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
    <span class="rounded bg-green-900/30 px-1.5 py-0.5 text-[10px] text-green-400">Vision</span>
  {/if}
  {#if capabilities.embeddings}
    <span class="rounded bg-purple-900/30 px-1.5 py-0.5 text-[10px] text-purple-400">Embeddings</span>
  {/if}
  {#if capabilities.gpu}
    <span class="rounded bg-amber-900/30 px-1.5 py-0.5 text-[10px] text-amber-400">GPU</span>
  {/if}
  {#if capabilities.device_selection}
    <span class="rounded bg-cyan-900/30 px-1.5 py-0.5 text-[10px] text-cyan-400">Device Select</span>
  {/if}
  {#if capabilities.streaming}
    <span class="rounded bg-blue-900/30 px-1.5 py-0.5 text-[10px] text-blue-400">Streaming</span>
  {/if}
  {#if capabilities.tool_calling}
    <span class="rounded bg-pink-900/30 px-1.5 py-0.5 text-[10px] text-pink-400">Tools</span>
  {/if}
</div>
