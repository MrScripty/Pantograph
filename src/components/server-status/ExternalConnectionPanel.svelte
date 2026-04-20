<script lang="ts">
  type Props = {
    externalUrl: string;
    apiKey: string;
    isConnecting: boolean;
    isConnected: boolean;
    onConnect: () => Promise<void>;
    onDisconnect: () => Promise<void>;
  };

  let {
    externalUrl = $bindable(),
    apiKey = $bindable(),
    isConnecting,
    isConnected,
    onConnect,
    onDisconnect,
  }: Props = $props();
</script>

<div class="space-y-2">
  <div class="text-[10px] uppercase tracking-wider text-neutral-600">
    Connect to external server (LM Studio, OpenAI, etc.)
  </div>
  <input
    type="text"
    bind:value={externalUrl}
    placeholder="http://localhost:1234 or https://api.openai.com/v1"
    class="w-full rounded border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-xs text-neutral-200 focus:border-neutral-500 focus:outline-none"
    disabled={isConnecting || isConnected}
  />
  <input
    type="password"
    bind:value={apiKey}
    placeholder="API Key (optional)"
    class="w-full rounded border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-xs text-neutral-200 focus:border-neutral-500 focus:outline-none"
    disabled={isConnecting || isConnected}
  />
  <div class="flex gap-2">
    {#if isConnected}
      <button
        type="button"
        onclick={onDisconnect}
        class="flex-1 rounded bg-red-600 px-3 py-1.5 text-xs transition-colors hover:bg-red-500"
      >
        Disconnect
      </button>
    {:else}
      <button
        type="button"
        onclick={onConnect}
        disabled={isConnecting || !externalUrl.trim()}
        class="flex-1 rounded bg-blue-600 px-3 py-1.5 text-xs transition-colors hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500"
      >
        {isConnecting ? 'Connecting...' : 'Connect'}
      </button>
    {/if}
  </div>
</div>
