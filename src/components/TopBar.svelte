<script lang="ts">
  import { Logger } from '../services/Logger';
  import { canvasExport } from '../services/CanvasExport';
  import { LLMService } from '../services/LLMService';
  import { panelWidth, openSidePanel } from '../stores/panelStore';

  let inputValue = '';
  let isLoading = false;

  const handleGo = async () => {
    if (!inputValue.trim() || isLoading) return;

    Logger.log('COMMAND_SUBMITTED', { text: inputValue });

    const imageBase64 = canvasExport.exportToBase64();
    if (!imageBase64) {
      Logger.log('CANVAS_EXPORT_FAILED', {}, 'error');
      return;
    }

    isLoading = true;
    openSidePanel();

    try {
      await LLMService.sendVisionPrompt(inputValue, imageBase64);
    } catch (error) {
      Logger.log('LLM_SUBMIT_ERROR', { error: String(error) }, 'error');
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
</div>
