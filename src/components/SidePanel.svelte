<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { LLMService, type LLMState } from '../services/LLMService';
  import { AgentService } from '../services/AgentService';
  import { sidePanelOpen, toggleSidePanel } from '../stores/panelStore';
  import { activeSidePanelTab, type SidePanelTab } from '../stores/sidePanelTabStore';
  import { promptHistoryStore } from '../stores/promptHistoryStore';
  import { componentRegistry } from '../services/HotLoadRegistry';
  import { Logger } from '../services/Logger';
  import { SettingsTab, ActivityLog, FollowUpInput } from './side-panel';

  let state: LLMState = LLMService.getState();
  let agentState = AgentService.getState();
  let unsubscribeLLM: (() => void) | null = null;
  let unsubscribeAgent: (() => void) | null = null;
  let messagesContainer: HTMLDivElement;
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

  const handleFollowUp = async (event: CustomEvent<string>) => {
    const submittedPrompt = event.detail;
    console.log('[SidePanel] Follow-up prompt:', submittedPrompt);
    Logger.log('FOLLOW_UP_SUBMITTED', { text: submittedPrompt });

    try {
      const response = await AgentService.run(submittedPrompt);
      console.log('[SidePanel] Follow-up response:', response);

      // Register any generated components
      for (const update of response.component_updates) {
        console.log('[SidePanel] Registering component:', update.id);
        await componentRegistry.registerFromUpdate(update);
      }

      Logger.log('FOLLOW_UP_COMPLETE', {
        filesChanged: response.file_changes.length,
        componentsUpdated: response.component_updates.length,
      });

      // Add to persistent history
      promptHistoryStore.addPrompt(submittedPrompt);
    } catch (error) {
      console.error('[SidePanel] Follow-up error:', error);
      Logger.log('FOLLOW_UP_ERROR', { error: String(error) }, 'error');
    }
  };

  const handleStopAgent = () => {
    AgentService.stop();
  };

  const setActiveTab = (tab: SidePanelTab) => {
    activeSidePanelTab.set(tab);
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
      <!-- Header with status -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-neutral-700">
        <h2 class="text-sm font-bold tracking-wider uppercase text-neutral-300">
          AI Assistant
        </h2>
        <span class="w-2 h-2 rounded-full {statusColor}" title={statusText}></span>
      </div>

      <!-- Tab Bar -->
      <div class="flex border-b border-neutral-700">
        <button
          on:click={() => setActiveTab('settings')}
          class="flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider transition-colors {$activeSidePanelTab === 'settings' ? 'text-neutral-100 bg-neutral-800 border-b-2 border-blue-500' : 'text-neutral-500 hover:text-neutral-300 hover:bg-neutral-800/50'}"
        >
          Settings
        </button>
        <button
          on:click={() => setActiveTab('history')}
          class="flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider transition-colors {$activeSidePanelTab === 'history' ? 'text-neutral-100 bg-neutral-800 border-b-2 border-blue-500' : 'text-neutral-500 hover:text-neutral-300 hover:bg-neutral-800/50'}"
        >
          History
        </button>
      </div>

      <!-- Settings Tab Content -->
      {#if $activeSidePanelTab === 'settings'}
        <SettingsTab />
      {/if}

      <!-- History Tab Content -->
      {#if $activeSidePanelTab === 'history'}
        <div
          bind:this={messagesContainer}
          on:scroll={handleScroll}
          class="flex-1 overflow-y-auto p-4"
        >
          <ActivityLog
            activityLog={agentState.activityLog}
            streamingText={agentState.streamingText}
            streamingReasoning={agentState.streamingReasoning}
            isAgentRunning={agentState.isRunning}
            messages={state.messages}
            isGenerating={state.isGenerating}
            currentResponse={state.currentResponse}
            error={state.error}
            isReady={state.status.ready}
          />
        </div>

        <FollowUpInput
          isAgentRunning={agentState.isRunning}
          isReady={state.status.ready}
          on:submit={handleFollowUp}
          on:stop={handleStopAgent}
        />
      {/if}
    </div>
  {/if}
</div>
