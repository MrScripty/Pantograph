<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { slide } from 'svelte/transition';
  import { LLMService, type LLMState, type ChatMessage } from '../services/LLMService';
  import { sidePanelOpen, toggleSidePanel } from '../stores/panelStore';

  let state: LLMState = LLMService.getState();
  let unsubscribe: (() => void) | null = null;
  let messagesContainer: HTMLDivElement;
  let serverUrl = 'http://localhost:1234';
  let isConnecting = false;
  let isUserScrolledUp = false;

  const checkIfScrolledToBottom = () => {
    if (!messagesContainer) return;
    const threshold = 50;
    const { scrollTop, scrollHeight, clientHeight } = messagesContainer;
    isUserScrolledUp = scrollHeight - scrollTop - clientHeight > threshold;
  };

  const handleScroll = () => {
    checkIfScrolledToBottom();
  };

  onMount(() => {
    unsubscribe = LLMService.subscribe((nextState) => {
      state = nextState;
      if (messagesContainer && !isUserScrolledUp) {
        requestAnimationFrame(() => {
          messagesContainer.scrollTop = messagesContainer.scrollHeight;
        });
      }
    });
  });

  onDestroy(() => {
    unsubscribe?.();
  });

  const formatMessage = (msg: ChatMessage): string => {
    if (msg.imageBase64) {
      return `[Image attached]\n\n${msg.content}`;
    }
    return msg.content;
  };

  const connectToServer = async () => {
    if (!serverUrl.trim()) return;
    isConnecting = true;
    try {
      await LLMService.connectToServer(serverUrl);
    } catch (error) {
      console.error('Failed to connect:', error);
    } finally {
      isConnecting = false;
    }
  };

  const disconnect = async () => {
    await LLMService.stop();
  };

  $: statusColor = state.status.ready
    ? 'bg-green-500'
    : state.status.mode !== 'none'
      ? 'bg-yellow-500 animate-pulse'
      : 'bg-neutral-500';

  $: statusText = state.status.ready
    ? `Connected (${state.status.mode})`
    : state.status.mode !== 'none'
      ? 'Connecting...'
      : 'Not connected';
</script>

<div class="fixed right-0 top-0 h-full z-50 flex">
  <!-- Handle - always visible -->
  <button
    on:click={toggleSidePanel}
    class="h-full w-5 bg-neutral-800/90 backdrop-blur-md border-l border-neutral-700 flex items-center justify-center hover:bg-neutral-700/90 transition-colors cursor-pointer"
    aria-label={$sidePanelOpen ? 'Close AI panel' : 'Open AI panel'}
  >
    <div class="flex flex-col gap-1.5">
      <span class="w-1 h-1 rounded-full bg-neutral-400"></span>
      <span class="w-1 h-1 rounded-full bg-neutral-400"></span>
      <span class="w-1 h-1 rounded-full bg-neutral-400"></span>
    </div>
  </button>

  <!-- Panel content -->
  {#if $sidePanelOpen}
    <div
      class="h-full w-80 bg-neutral-900/95 backdrop-blur-md border-l border-neutral-700 flex flex-col"
    >
      <div class="flex items-center justify-between px-4 py-3 border-b border-neutral-700">
        <h2 class="text-sm font-bold tracking-wider uppercase text-neutral-300">
          AI Response
        </h2>
        <span class="w-2 h-2 rounded-full {statusColor}" title={statusText}></span>
      </div>

      {#if !state.status.ready}
        <div class="p-4 border-b border-neutral-700 space-y-3">
          <div class="text-xs uppercase tracking-wider text-neutral-500 mb-2">
            Connect to LLM Server
          </div>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={serverUrl}
              placeholder="http://localhost:1234"
              class="flex-1 bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm font-mono outline-none focus:border-blue-500"
              disabled={isConnecting}
            />
          </div>
          <button
            on:click={connectToServer}
            disabled={isConnecting || !serverUrl.trim()}
            class="w-full py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed rounded text-sm font-bold transition-colors"
          >
            {isConnecting ? 'Connecting...' : 'Connect'}
          </button>
          <div class="text-xs text-neutral-600">
            LM Studio default: http://localhost:1234
          </div>
        </div>
      {:else}
        <div class="px-4 py-2 border-b border-neutral-700 flex items-center justify-between">
          <div class="text-xs text-neutral-500">
            <span class="uppercase tracking-wider">{state.status.mode}</span>
            {#if state.status.url}
              <span class="ml-2 font-mono text-[10px]">{state.status.url}</span>
            {/if}
          </div>
          <button
            on:click={disconnect}
            class="text-xs text-red-400 hover:text-red-300 transition-colors"
          >
            Disconnect
          </button>
        </div>
      {/if}

      <div
        bind:this={messagesContainer}
        on:scroll={handleScroll}
        class="flex-1 overflow-y-auto p-4 space-y-4"
      >
        {#each state.messages as message (message.timestamp)}
          <div
            class="rounded-lg p-3 {message.role === 'user' ? 'bg-blue-900/30 ml-4' : 'bg-neutral-800/50 mr-4'}"
            transition:slide
          >
            <div class="text-xs text-neutral-500 mb-1 uppercase tracking-wider">
              {message.role}
            </div>
            <div class="text-sm whitespace-pre-wrap break-words">
              {formatMessage(message)}
            </div>
          </div>
        {/each}

        {#if state.isGenerating && state.currentResponse}
          <div class="rounded-lg p-3 bg-neutral-800/50 mr-4" transition:slide>
            <div class="text-xs text-neutral-500 mb-1 uppercase tracking-wider">
              Assistant
            </div>
            <div class="text-sm whitespace-pre-wrap break-words">
              {state.currentResponse}
              <span class="inline-block w-2 h-4 bg-blue-500 animate-pulse ml-1"></span>
            </div>
          </div>
        {/if}

        {#if state.isGenerating && !state.currentResponse}
          <div class="flex items-center gap-2 text-neutral-500 text-sm p-3">
            <div class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></div>
            <span>Thinking...</span>
          </div>
        {/if}

        {#if state.error}
          <div class="rounded-lg p-3 bg-red-900/30 border border-red-700 text-red-300 text-sm">
            Error: {state.error}
          </div>
        {/if}

        {#if state.messages.length === 0 && !state.isGenerating && !state.error && state.status.ready}
          <div class="text-center text-neutral-600 text-sm py-8">
            Draw something and enter a prompt to get started
          </div>
        {/if}

        {#if !state.status.ready && state.messages.length === 0}
          <div class="text-center text-neutral-600 text-sm py-8">
            Connect to an LLM server to get started
          </div>
        {/if}
      </div>

      <div class="px-4 py-3 border-t border-neutral-700">
        <button
          on:click={() => LLMService.clearHistory()}
          disabled={state.messages.length === 0}
          class="w-full py-2 text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-300 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Clear History
        </button>
      </div>
    </div>
  {/if}
</div>
