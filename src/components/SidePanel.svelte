<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { slide } from 'svelte/transition';
  import { LLMService, type LLMState, type ChatMessage } from '../services/LLMService';
  import {
    AgentService,
    type AgentActivityItem,
  } from '../services/AgentService';
  import { sidePanelOpen, toggleSidePanel } from '../stores/panelStore';
  import ServerStatus from './ServerStatus.svelte';
  import ModelConfig from './ModelConfig.svelte';
  import DeviceConfig from './DeviceConfig.svelte';
  import RagStatus from './RagStatus.svelte';
  import SandboxSettings from './SandboxSettings.svelte';

  let state: LLMState = LLMService.getState();
  let agentState = AgentService.getState();
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeAgent: (() => void) | null = null;
  let messagesContainer: HTMLDivElement;
  let isUserScrolledUp = false;

  // Track expanded state for collapsible items
  let expandedItems: Set<string> = new Set();

  const toggleExpanded = (id: string) => {
    if (expandedItems.has(id)) {
      expandedItems.delete(id);
    } else {
      expandedItems.add(id);
    }
    expandedItems = new Set(expandedItems);
  };

  const checkIfScrolledToBottom = () => {
    if (!messagesContainer) return;
    const threshold = 50;
    const { scrollTop, scrollHeight, clientHeight } = messagesContainer;
    isUserScrolledUp = scrollHeight - scrollTop - clientHeight > threshold;
  };

  const handleScroll = () => {
    checkIfScrolledToBottom();
  };

  const scrollToBottom = () => {
    if (messagesContainer && !isUserScrolledUp) {
      requestAnimationFrame(() => {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      });
    }
  };

  onMount(() => {
    unsubscribeLLM = LLMService.subscribe((nextState) => {
      state = nextState;
      scrollToBottom();
    });

    unsubscribeAgent = AgentService.subscribeState((nextState) => {
      agentState = nextState;
      scrollToBottom();
    });
  });

  onDestroy(() => {
    unsubscribeLLM?.();
    unsubscribeAgent?.();
  });

  const formatMessage = (msg: ChatMessage): string => {
    if (msg.imageBase64) {
      return `[Image attached]\n\n${msg.content}`;
    }
    return msg.content;
  };

  const clearAll = () => {
    LLMService.clearHistory();
    AgentService.clearActivityLog();
  };

  // Truncate long text for display
  const truncate = (text: string, maxLength: number = 100): string => {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + '...';
  };

  // Format tool arguments for display
  const formatArgs = (args: string): string => {
    try {
      const parsed = JSON.parse(args);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return args;
    }
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

  // Check if there's any content to display
  $: hasContent =
    state.messages.length > 0 ||
    agentState.activityLog.length > 0 ||
    agentState.streamingText ||
    agentState.isRunning;
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
          AI Assistant
        </h2>
        <span class="w-2 h-2 rounded-full {statusColor}" title={statusText}></span>
      </div>

      <!-- Server Status (includes Backend Selector) -->
      <div class="px-4 py-3 border-b border-neutral-700">
        <ServerStatus />
      </div>

      <!-- Model Configuration -->
      <div class="px-4 py-3 border-b border-neutral-700">
        <ModelConfig />
      </div>

      <!-- Device Configuration -->
      <div class="px-4 py-3 border-b border-neutral-700">
        <DeviceConfig />
      </div>

      <!-- RAG Status Panel -->
      <div class="px-4 py-3 border-b border-neutral-700">
        <RagStatus />
      </div>

      <!-- Sandbox Settings Panel -->
      <div class="px-4 py-3 border-b border-neutral-700">
        <SandboxSettings />
      </div>

      <div
        bind:this={messagesContainer}
        on:scroll={handleScroll}
        class="flex-1 overflow-y-auto p-4 space-y-3"
      >
        <!-- Agent Activity Log -->
        {#each agentState.activityLog as item (item.id)}
          <div transition:slide>
            {#if item.type === 'system_prompt'}
              <!-- System Prompt Card (collapsible) -->
              <button
                on:click={() => toggleExpanded(item.id)}
                class="w-full text-left rounded-lg p-3 bg-purple-900/20 border border-purple-800/50 hover:bg-purple-900/30 transition-colors"
              >
                <div class="flex items-center gap-2 text-xs text-purple-400 mb-1">
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                  </svg>
                  <span class="uppercase tracking-wider">System Prompt</span>
                  <svg class="w-3 h-3 ml-auto transform {expandedItems.has(item.id) ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  </svg>
                </div>
                {#if expandedItems.has(item.id)}
                  <div class="text-xs text-neutral-400 whitespace-pre-wrap font-mono mt-2 max-h-64 overflow-y-auto">
                    {item.content}
                  </div>
                {:else}
                  <div class="text-xs text-neutral-500 truncate">
                    {truncate(item.content, 60)}
                  </div>
                {/if}
              </button>

            {:else if item.type === 'tool_call'}
              <!-- Tool Call Card with status -->
              <button
                on:click={() => toggleExpanded(item.id)}
                class="w-full text-left rounded-lg p-3 {item.metadata?.status === 'error' ? 'bg-red-900/20 border border-red-800/50 hover:bg-red-900/30' : 'bg-amber-900/20 border border-amber-800/50 hover:bg-amber-900/30'} transition-colors"
              >
                <div class="flex items-center gap-2 text-xs {item.metadata?.status === 'error' ? 'text-red-400' : 'text-amber-400'} mb-1">
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                  <span class="uppercase tracking-wider">Tool Call</span>
                  <span class="font-mono {item.metadata?.status === 'error' ? 'text-red-300' : 'text-amber-300'}">{item.metadata?.toolName}</span>
                  <!-- Status indicator -->
                  {#if item.metadata?.status === 'success'}
                    <span class="text-green-400 font-bold">✓</span>
                  {:else if item.metadata?.status === 'error'}
                    <span class="text-red-400 font-bold">✗</span>
                  {:else}
                    <span class="text-neutral-500 animate-pulse">...</span>
                  {/if}
                  <svg class="w-3 h-3 ml-auto transform {expandedItems.has(item.id) ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  </svg>
                </div>
                {#if expandedItems.has(item.id) && item.metadata?.toolArgs}
                  <div class="text-xs text-neutral-400 whitespace-pre-wrap font-mono mt-2 max-h-48 overflow-y-auto bg-neutral-900/50 rounded p-2">
                    {formatArgs(item.metadata.toolArgs)}
                  </div>
                {/if}
                {#if item.metadata?.status === 'error' && item.metadata?.errorMessage}
                  <div class="text-xs text-red-300 whitespace-pre-wrap font-mono mt-2 max-h-48 overflow-y-auto bg-red-900/30 rounded p-2">
                    {item.metadata.errorMessage}
                  </div>
                {/if}
              </button>

            {:else if item.type === 'tool_result'}
              <!-- Tool Result Card - Legacy, kept for backwards compatibility but results now merge into tool_call -->
              <button
                on:click={() => toggleExpanded(item.id)}
                class="w-full text-left rounded-lg p-3 bg-green-900/20 border border-green-800/50 hover:bg-green-900/30 transition-colors"
              >
                <div class="flex items-center gap-2 text-xs text-green-400 mb-1">
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span class="uppercase tracking-wider">Tool Result</span>
                  <svg class="w-3 h-3 ml-auto transform {expandedItems.has(item.id) ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  </svg>
                </div>
                {#if expandedItems.has(item.id)}
                  <div class="text-xs text-neutral-400 whitespace-pre-wrap font-mono mt-2 max-h-48 overflow-y-auto bg-neutral-900/50 rounded p-2">
                    {item.content}
                  </div>
                {:else}
                  <div class="text-xs text-neutral-500 truncate">
                    {truncate(item.content, 80)}
                  </div>
                {/if}
              </button>

            {:else if item.type === 'text'}
              <!-- Final Text Response -->
              <div class="rounded-lg p-3 bg-neutral-800/50">
                <div class="text-xs text-neutral-500 mb-1 uppercase tracking-wider">
                  Assistant
                </div>
                <div class="text-sm whitespace-pre-wrap break-words text-neutral-200">
                  {item.content}
                </div>
              </div>

            {:else if item.type === 'reasoning'}
              <!-- Reasoning (collapsible) -->
              <button
                on:click={() => toggleExpanded(item.id)}
                class="w-full text-left rounded-lg p-3 bg-blue-900/20 border border-blue-800/50 hover:bg-blue-900/30 transition-colors"
              >
                <div class="flex items-center gap-2 text-xs text-blue-400 mb-1">
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                  </svg>
                  <span class="uppercase tracking-wider">Thinking</span>
                  <svg class="w-3 h-3 ml-auto transform {expandedItems.has(item.id) ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  </svg>
                </div>
                {#if expandedItems.has(item.id)}
                  <div class="text-xs text-neutral-400 whitespace-pre-wrap mt-2">
                    {item.content}
                  </div>
                {:else}
                  <div class="text-xs text-neutral-500 truncate">
                    {truncate(item.content, 60)}
                  </div>
                {/if}
              </button>

            {:else if item.type === 'status'}
              <!-- Status Message -->
              <div class="flex items-center gap-2 text-neutral-500 text-xs px-2">
                <div class="w-1.5 h-1.5 rounded-full bg-blue-500"></div>
                <span>{item.content}</span>
              </div>

            {:else if item.type === 'error'}
              <!-- Error Message -->
              <div class="rounded-lg p-3 bg-red-900/30 border border-red-700 text-red-300 text-sm">
                {item.content}
              </div>
            {/if}
          </div>
        {/each}

        <!-- Streaming Text -->
        {#if agentState.streamingText}
          <div class="rounded-lg p-3 bg-neutral-800/50" transition:slide>
            <div class="text-xs text-neutral-500 mb-1 uppercase tracking-wider">
              Assistant
            </div>
            <div class="text-sm whitespace-pre-wrap break-words text-neutral-200">
              {agentState.streamingText}
              <span class="inline-block w-2 h-4 bg-blue-500 animate-pulse ml-1"></span>
            </div>
          </div>
        {/if}

        <!-- Agent Running Indicator -->
        {#if agentState.isRunning && !agentState.streamingText && agentState.activityLog.length === 0}
          <div class="flex items-center gap-2 text-neutral-500 text-sm p-3" transition:slide>
            <div class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></div>
            <span>Starting agent...</span>
          </div>
        {/if}

        <!-- LLM Messages (legacy chat) -->
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

        {#if !hasContent && state.status.ready}
          <div class="text-center text-neutral-600 text-sm py-8">
            Draw something and enter a prompt to get started
          </div>
        {/if}

        {#if !state.status.ready && !hasContent}
          <div class="text-center text-neutral-600 text-sm py-8">
            Connect to an LLM server to get started
          </div>
        {/if}
      </div>

      <div class="px-4 py-3 border-t border-neutral-700">
        <button
          on:click={clearAll}
          disabled={!hasContent}
          class="w-full py-2 text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-300 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Clear History
        </button>
      </div>
    </div>
  {/if}
</div>
