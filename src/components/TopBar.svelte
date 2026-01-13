<script lang="ts">
  import { Logger } from '../services/Logger';
  import { AgentService } from '../services/AgentService';
  import { componentRegistry } from '../services/HotLoadRegistry';
  import { panelWidth, openSidePanel } from '../stores/panelStore';

  let inputValue = '';
  let isLoading = false;
  let errorMessage = '';

  const handleGo = async () => {
    if (!inputValue.trim() || isLoading) return;

    console.log('[TopBar] handleGo called with:', inputValue);
    Logger.log('COMMAND_SUBMITTED', { text: inputValue });

    isLoading = true;
    errorMessage = '';
    openSidePanel();

    try {
      console.log('[TopBar] Calling AgentService.run...');
      // Run the agent - it handles canvas export internally
      const response = await AgentService.run(inputValue);
      console.log('[TopBar] AgentService.run returned:', response);

      // Register the generated components
      for (const update of response.component_updates) {
        console.log('[TopBar] Registering component:', update.id);
        await componentRegistry.registerFromUpdate(update);
      }

      Logger.log('UI_GENERATION_COMPLETE', {
        filesChanged: response.file_changes.length,
        componentsUpdated: response.component_updates.length,
      });
    } catch (error) {
      console.error('[TopBar] Error:', error);
      errorMessage = error instanceof Error ? error.message : String(error);
      Logger.log('AGENT_ERROR', { error: String(error) }, 'error');
    } finally {
      isLoading = false;
      inputValue = '';
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
      on:keydown={(e) => e.key === 'Enter' && handleGo()}
    />
    <button
      on:click={handleGo}
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
