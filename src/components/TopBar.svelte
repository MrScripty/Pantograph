<script lang="ts">
  import { onMount } from 'svelte';
  import { Logger } from '../services/Logger';
  import { AgentService, type ComponentUpdate } from '../services/AgentService';
  import { componentRegistry } from '../services/HotLoadRegistry';
  import { panelWidth, openSidePanel } from '../stores/panelStore';
  import { promptHistoryStore } from '../stores/promptHistoryStore';

  let inputValue = '';
  let isLoading = false;
  let errorMessage = '';

  // Prompt history state - synced with persistent store
  let promptHistory: string[] = [];
  let historyIndex = -1;
  let tempInput = '';

  // Subscribe to the persistent prompt history store
  onMount(() => {
    const unsubscribe = promptHistoryStore.subscribe(history => {
      promptHistory = history;
    });
    return unsubscribe;
  });

  const handleGo = async () => {
    if (!inputValue.trim() || isLoading) return;

    const submittedPrompt = inputValue.trim();
    console.log('[TopBar] handleGo called with:', submittedPrompt);
    Logger.log('COMMAND_SUBMITTED', { text: submittedPrompt });

    isLoading = true;
    errorMessage = '';
    openSidePanel();

    // Track components registered via streaming events to avoid duplicate registration
    const registeredIds = new Set<string>();

    // Subscribe to events for immediate component registration
    const unsubscribe = AgentService.subscribeEvents(async (event) => {
      if (event.event_type === 'component_created' && event.data) {
        const update = event.data as ComponentUpdate;
        console.log('[TopBar] Component created via stream, registering immediately:', update.id);
        registeredIds.add(update.id);
        await componentRegistry.registerFromUpdate(update);
      }
    });

    try {
      console.log('[TopBar] Calling AgentService.run...');
      // Run the agent - it handles canvas export internally
      const response = await AgentService.run(submittedPrompt);
      console.log('[TopBar] AgentService.run returned:', response);

      // Register any components not already registered via streaming
      // (fallback for cases where early termination didn't trigger)
      for (const update of response.component_updates) {
        if (!registeredIds.has(update.id)) {
          console.log('[TopBar] Registering component from final response:', update.id);
          await componentRegistry.registerFromUpdate(update);
        }
      }

      Logger.log('UI_GENERATION_COMPLETE', {
        filesChanged: response.file_changes.length,
        componentsUpdated: response.component_updates.length,
      });

      // Success - add to persistent history and clear input
      promptHistoryStore.addPrompt(submittedPrompt);
      historyIndex = -1;
      tempInput = '';
      inputValue = '';
    } catch (error) {
      console.error('[TopBar] Error:', error);
      errorMessage = error instanceof Error ? error.message : String(error);
      Logger.log('AGENT_ERROR', { error: String(error) }, 'error');
      // On error, preserve the input value (don't clear it)
    } finally {
      unsubscribe();
      isLoading = false;
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleGo();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (promptHistory.length > 0) {
        if (historyIndex === -1) {
          // Save current input before navigating
          tempInput = inputValue;
          historyIndex = promptHistory.length - 1;
        } else if (historyIndex > 0) {
          historyIndex--;
        }
        inputValue = promptHistory[historyIndex];
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex !== -1) {
        if (historyIndex < promptHistory.length - 1) {
          historyIndex++;
          inputValue = promptHistory[historyIndex];
        } else {
          // Return to the temp input
          historyIndex = -1;
          inputValue = tempInput;
        }
      }
    }
  };
</script>

<div
  class="fixed top-8 left-1/2 w-full max-w-xl px-4 z-50 transition-transform duration-300 ease-out"
  style="transform: translateX(calc(-50% - {$panelWidth / 2}px));"
>
  <div class="flex bg-neutral-900/90 backdrop-blur-md border border-neutral-700 rounded-lg overflow-hidden shadow-2xl">
    <input
      type="text"
      bind:value={inputValue}
      placeholder="Describe what you want to do with this drawing..."
      class="flex-1 bg-transparent px-4 py-3 outline-none font-mono text-sm placeholder:text-neutral-600"
      disabled={isLoading}
      onkeydown={handleKeyDown}
    />
    <button
      onclick={handleGo}
      disabled={isLoading || !inputValue.trim()}
      class="px-6 py-3 bg-neutral-800 hover:bg-neutral-700 disabled:opacity-50 disabled:cursor-not-allowed border-l border-neutral-700 transition-colors text-sm font-bold tracking-wider"
    >
      {isLoading ? '...' : 'GO'}
    </button>
  </div>
  {#if errorMessage}
    <div class="mt-2 p-3 bg-red-900/80 border border-red-700 rounded-lg text-red-200 text-sm">
      {errorMessage}
    </div>
  {/if}
</div>
