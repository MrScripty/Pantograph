<script lang="ts">
  import { slide } from 'svelte/transition';
  import type { AgentActivityItem } from '../../services/AgentService';
  import type { ChatMessage } from '../../services/LLMService';

  // Props
  interface Props {
    activityLog?: AgentActivityItem[];
    streamingText?: string;
    streamingReasoning?: string;
    isAgentRunning?: boolean;
    messages?: ChatMessage[];
    isGenerating?: boolean;
    currentResponse?: string;
    error?: string | null;
    isReady?: boolean;
  }

  let {
    activityLog = [],
    streamingText = '',
    streamingReasoning = '',
    isAgentRunning = false,
    messages = [],
    isGenerating = false,
    currentResponse = '',
    error = null,
    isReady = false,
  }: Props = $props();

  // Local state
  let expandedItems: Set<string> = $state(new Set());
  let hoveredItemId: string | null = $state(null);

  // Handle Ctrl+C to copy hovered item content
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.ctrlKey && event.key === 'c' && hoveredItemId) {
      const item = activityLog.find(i => i.id === hoveredItemId);
      if (item) {
        event.preventDefault();
        let textToCopy = item.content;

        // For tool calls, include tool name and args
        if (item.type === 'tool_call' && item.metadata) {
          textToCopy = `Tool: ${item.metadata.toolName}\nArguments: ${item.metadata.toolArgs || ''}`;
          if (item.metadata.errorMessage) {
            textToCopy += `\nError: ${item.metadata.errorMessage}`;
          }
        }

        navigator.clipboard.writeText(textToCopy).then(() => {
          console.log('Copied to clipboard:', truncate(textToCopy, 50));
        }).catch(err => {
          console.error('Failed to copy:', err);
        });
      }
    }
  };

  const toggleExpanded = (id: string) => {
    if (expandedItems.has(id)) {
      expandedItems.delete(id);
    } else {
      expandedItems.add(id);
    }
    expandedItems = new Set(expandedItems);
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

  const formatMessage = (msg: ChatMessage): string => {
    if (msg.imageBase64) {
      return `[Image attached]\n\n${msg.content}`;
    }
    return msg.content;
  };

  // Check if there's any content to display
  let hasContent = $derived(
    messages.length > 0 ||
    activityLog.length > 0 ||
    streamingText ||
    isAgentRunning
  );
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="space-y-3">
  <!-- Agent Activity Log -->
  {#each activityLog as item (item.id)}
    <div transition:slide>
      {#if item.type === 'system_prompt'}
        <!-- System Prompt Card (collapsible) -->
        <button
          onclick={() => toggleExpanded(item.id)}
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
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
          onclick={() => toggleExpanded(item.id)}
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
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

      {:else if item.type === 'tool_call_streaming'}
        <!-- Streaming Tool Call Card - shows tool call as it's being generated -->
        <button
          onclick={() => toggleExpanded(item.id)}
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
          class="w-full text-left rounded-lg p-3 bg-cyan-900/20 border border-cyan-800/50 hover:bg-cyan-900/30 transition-colors"
        >
          <div class="flex items-center gap-2 text-xs text-cyan-400 mb-1">
            <svg class="w-3 h-3 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            <span class="uppercase tracking-wider">Generating Tool Call</span>
            {#if item.metadata?.toolName}
              <span class="font-mono text-cyan-300">{item.metadata.toolName}</span>
            {/if}
            <span class="ml-auto inline-block w-2 h-4 bg-cyan-500 animate-pulse"></span>
          </div>
          {#if expandedItems.has(item.id) && item.metadata?.toolArgs}
            <div class="text-xs text-neutral-400 whitespace-pre-wrap font-mono mt-2 max-h-48 overflow-y-auto bg-neutral-900/50 rounded p-2">
              {formatArgs(item.metadata.toolArgs)}<span class="inline-block w-1 h-3 bg-cyan-500 animate-pulse ml-0.5"></span>
            </div>
          {:else if item.metadata?.toolArgs}
            <div class="text-xs text-neutral-500 truncate font-mono">
              {truncate(item.metadata.toolArgs, 60)}<span class="inline-block w-1 h-3 bg-cyan-500 animate-pulse ml-0.5"></span>
            </div>
          {/if}
        </button>

      {:else if item.type === 'tool_result'}
        <!-- Tool Result Card - Legacy, kept for backwards compatibility but results now merge into tool_call -->
        <button
          onclick={() => toggleExpanded(item.id)}
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
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
        <div
          class="rounded-lg p-3 bg-neutral-800/50"
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
          role="button"
          tabindex="0"
        >
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
          onclick={() => toggleExpanded(item.id)}
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
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
        <div
          class="flex items-center gap-2 text-neutral-500 text-xs px-2"
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
          role="button"
          tabindex="0"
        >
          <div class="w-1.5 h-1.5 rounded-full bg-blue-500"></div>
          <span>{item.content}</span>
        </div>

      {:else if item.type === 'error'}
        <!-- Error Message -->
        <div
          class="rounded-lg p-3 bg-red-900/30 border border-red-700 text-red-300 text-sm"
          onmouseenter={() => hoveredItemId = item.id}
          onmouseleave={() => hoveredItemId = null}
          role="button"
          tabindex="0"
        >
          {item.content}
        </div>
      {/if}
    </div>
  {/each}

  <!-- Streaming Reasoning -->
  {#if streamingReasoning}
    <div class="rounded-lg p-3 bg-blue-900/20 border border-blue-800/50" transition:slide>
      <div class="flex items-center gap-2 text-xs text-blue-400 mb-1">
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
        </svg>
        <span class="uppercase tracking-wider">Thinking</span>
        <span class="ml-auto inline-block w-2 h-4 bg-blue-500 animate-pulse"></span>
      </div>
      <div class="text-xs text-neutral-400 whitespace-pre-wrap">
        {streamingReasoning}
        <span class="inline-block w-1 h-3 bg-blue-500 animate-pulse ml-0.5"></span>
      </div>
    </div>
  {/if}

  <!-- Streaming Text -->
  {#if streamingText}
    <div class="rounded-lg p-3 bg-neutral-800/50" transition:slide>
      <div class="text-xs text-neutral-500 mb-1 uppercase tracking-wider">
        Assistant
      </div>
      <div class="text-sm whitespace-pre-wrap break-words text-neutral-200">
        {streamingText}
        <span class="inline-block w-2 h-4 bg-blue-500 animate-pulse ml-1"></span>
      </div>
    </div>
  {/if}

  <!-- Agent Running Indicator -->
  {#if isAgentRunning && !streamingText && !streamingReasoning && activityLog.length === 0}
    <div class="flex items-center gap-2 text-neutral-500 text-sm p-3" transition:slide>
      <div class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></div>
      <span>Starting agent...</span>
    </div>
  {/if}

  <!-- LLM Messages (legacy chat) -->
  {#each messages as message (message.timestamp)}
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

  {#if isGenerating && currentResponse}
    <div class="rounded-lg p-3 bg-neutral-800/50 mr-4" transition:slide>
      <div class="text-xs text-neutral-500 mb-1 uppercase tracking-wider">
        Assistant
      </div>
      <div class="text-sm whitespace-pre-wrap break-words">
        {currentResponse}
        <span class="inline-block w-2 h-4 bg-blue-500 animate-pulse ml-1"></span>
      </div>
    </div>
  {/if}

  {#if isGenerating && !currentResponse}
    <div class="flex items-center gap-2 text-neutral-500 text-sm p-3">
      <div class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></div>
      <span>Thinking...</span>
    </div>
  {/if}

  {#if error}
    <div class="rounded-lg p-3 bg-red-900/30 border border-red-700 text-red-300 text-sm">
      Error: {error}
    </div>
  {/if}

  {#if !hasContent && isReady}
    <div class="text-center text-neutral-600 text-sm py-8">
      Draw something and enter a prompt to get started
    </div>
  {/if}

  {#if !isReady && !hasContent}
    <div class="text-center text-neutral-600 text-sm py-8">
      Connect to an LLM server to get started
    </div>
  {/if}
</div>
