<script lang="ts">
  type Props = {
    externalUrl: string;
    apiKey: string;
    isConnecting: boolean;
    canConnect: boolean;
    isConnected: boolean;
    onConnect: () => Promise<void>;
    onDisconnect: () => Promise<void>;
  };

  let {
    externalUrl = $bindable(),
    apiKey = $bindable(),
    isConnecting,
    canConnect,
    isConnected,
    onConnect,
    onDisconnect,
  }: Props = $props();
</script>

<div class="flex flex-col gap-2">
  <p class="m-0 pg-muted pg-small">Connect to an external server (LM Studio, OpenAI, etc.)</p>
  <label class="pg-fl">
    Server URL
    <input
      type="text"
      bind:value={externalUrl}
      placeholder="http://localhost:1234 or https://api.openai.com/v1"
      class="pg-field pg-mono"
      disabled={!canConnect || isConnecting || isConnected}
    />
  </label>
  <label class="pg-fl">
    API key
    <input
      type="password"
      bind:value={apiKey}
      placeholder="API Key (optional)"
      class="pg-field pg-mono"
      disabled={!canConnect || isConnecting || isConnected}
    />
  </label>
  <div class="flex gap-2">
    {#if isConnected}
      <button type="button" onclick={onDisconnect} class="pg-btn pg-btn-danger">
        Disconnect
      </button>
    {:else}
      <button
        type="button"
        onclick={onConnect}
        disabled={!canConnect || isConnecting || !externalUrl.trim()}
        class="pg-btn pg-btn-primary"
      >
        {isConnecting ? 'Connecting...' : 'Connect'}
      </button>
    {/if}
  </div>
</div>
