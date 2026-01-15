<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ConfigService, type ConfigState, type ModelConfig } from '../services/ConfigService';

  let state: ConfigState = ConfigService.getState();
  let unsubscribe: (() => void) | null = null;
  let showConfig = false;
  let isSaving = false;

  // Local form state
  let vlmModelPath: string = '';
  let vlmMmprojPath: string = '';
  let embeddingModelPath: string = '';
  let candleEmbeddingModelPath: string = '';
  let ollamaVlmModel: string = '';

  onMount(() => {
    unsubscribe = ConfigService.subscribe((nextState) => {
      state = nextState;
      // Sync local form state
      vlmModelPath = nextState.config.models.vlm_model_path || '';
      vlmMmprojPath = nextState.config.models.vlm_mmproj_path || '';
      embeddingModelPath = nextState.config.models.embedding_model_path || '';
      candleEmbeddingModelPath = nextState.config.models.candle_embedding_model_path || '';
      ollamaVlmModel = nextState.config.models.ollama_vlm_model || '';
    });
  });

  onDestroy(() => {
    unsubscribe?.();
  });

  const pickVlmModel = async () => {
    const path = await ConfigService.pickModelFile('Select VLM Model (GGUF)');
    if (path) {
      vlmModelPath = path;
    }
  };

  const pickMmproj = async () => {
    const path = await ConfigService.pickModelFile('Select MMProj File (GGUF)');
    if (path) {
      vlmMmprojPath = path;
    }
  };

  const pickEmbeddingModel = async () => {
    const path = await ConfigService.pickModelFile('Select Embedding Model (GGUF)');
    if (path) {
      embeddingModelPath = path;
    }
  };

  const pickCandleEmbeddingModel = async () => {
    const path = await ConfigService.pickDirectory('Select Candle Model Directory (SafeTensors)');
    if (path) {
      candleEmbeddingModelPath = path;
    }
  };

  const saveConfig = async () => {
    isSaving = true;
    try {
      const models: ModelConfig = {
        vlm_model_path: vlmModelPath || null,
        vlm_mmproj_path: vlmMmprojPath || null,
        embedding_model_path: embeddingModelPath || null,
        candle_embedding_model_path: candleEmbeddingModelPath || null,
        ollama_vlm_model: ollamaVlmModel || null,
      };
      await ConfigService.setModelConfig(models);
    } catch (error) {
      console.error('Failed to save config:', error);
    } finally {
      isSaving = false;
    }
  };

  const getFileName = (path: string): string => {
    if (!path) return '';
    return path.split('/').pop() || path.split('\\').pop() || path;
  };

  $: hasChanges =
    vlmModelPath !== (state.config.models.vlm_model_path || '') ||
    vlmMmprojPath !== (state.config.models.vlm_mmproj_path || '') ||
    embeddingModelPath !== (state.config.models.embedding_model_path || '') ||
    candleEmbeddingModelPath !== (state.config.models.candle_embedding_model_path || '') ||
    ollamaVlmModel !== (state.config.models.ollama_vlm_model || '');

  $: isConfigured = (vlmModelPath && vlmMmprojPath) || ollamaVlmModel;
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    on:click={() => (showConfig = !showConfig)}
  >
    <div class="flex items-center gap-2">
      <span>Model Configuration</span>
      {#if isConfigured}
        <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
      {:else}
        <span class="w-1.5 h-1.5 rounded-full bg-amber-500"></span>
      {/if}
    </div>
    <svg
      class="w-3 h-3 transform transition-transform {showConfig ? 'rotate-180' : ''}"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if showConfig}
    <div class="space-y-4 p-3 bg-neutral-800/30 rounded-lg">
      <!-- VLM Model Section -->
      <div class="space-y-2">
        <div class="text-[10px] text-neutral-600 uppercase tracking-wider">
          Vision-Language Model (VLM)
        </div>

        <!-- Model Path -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">Model File</label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={vlmModelPath}
              placeholder="Select model file..."
              class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500 truncate"
              title={vlmModelPath}
            />
            <button
              on:click={pickVlmModel}
              class="px-2 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded text-xs transition-colors"
            >
              Browse
            </button>
          </div>
          {#if vlmModelPath}
            <div class="text-[10px] text-neutral-600 truncate">{getFileName(vlmModelPath)}</div>
          {/if}
        </div>

        <!-- MMProj Path -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">MMProj File</label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={vlmMmprojPath}
              placeholder="Select mmproj file..."
              class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500 truncate"
              title={vlmMmprojPath}
            />
            <button
              on:click={pickMmproj}
              class="px-2 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded text-xs transition-colors"
            >
              Browse
            </button>
          </div>
          {#if vlmMmprojPath}
            <div class="text-[10px] text-neutral-600 truncate">{getFileName(vlmMmprojPath)}</div>
          {/if}
        </div>
      </div>

      <!-- Ollama VLM Model Section -->
      <div class="space-y-2 border-t border-neutral-700 pt-3">
        <div class="text-[10px] text-neutral-600 uppercase tracking-wider">
          Ollama VLM Model
        </div>
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">Model Name (for Ollama backend)</label>
          <input
            type="text"
            bind:value={ollamaVlmModel}
            placeholder="e.g., llava:13b, qwen2-vl:7b"
            class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500"
          />
          <div class="text-[10px] text-neutral-500">
            Enter an Ollama model name with vision support. Run <code class="bg-neutral-800 px-1 rounded">ollama pull llava:13b</code> to download.
          </div>
        </div>
      </div>

      <!-- Embedding Model Section -->
      <div class="space-y-2 border-t border-neutral-700 pt-3">
        <div class="text-[10px] text-neutral-600 uppercase tracking-wider">
          Embedding Model (for RAG)
        </div>

        <!-- GGUF Embedding Model (llama.cpp) -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">GGUF Model File (llama.cpp)</label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={embeddingModelPath}
              placeholder="Select embedding model..."
              class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500 truncate"
              title={embeddingModelPath}
            />
            <button
              on:click={pickEmbeddingModel}
              class="px-2 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded text-xs transition-colors"
            >
              Browse
            </button>
          </div>
          {#if embeddingModelPath}
            <div class="text-[10px] text-neutral-600 truncate">{getFileName(embeddingModelPath)}</div>
          {/if}
        </div>

        <!-- SafeTensors Embedding Model (Candle) -->
        <div class="space-y-1">
          <label class="text-xs text-neutral-400">SafeTensors Model Directory (Candle)</label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={candleEmbeddingModelPath}
              placeholder="e.g., ~/models/bge-small-en-v1.5/"
              class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500 truncate"
              title={candleEmbeddingModelPath}
            />
            <button
              on:click={pickCandleEmbeddingModel}
              class="px-2 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded text-xs transition-colors"
            >
              Browse
            </button>
          </div>
          {#if candleEmbeddingModelPath}
            <div class="text-[10px] text-neutral-600 truncate">{getFileName(candleEmbeddingModelPath)}</div>
          {/if}
          <div class="text-[10px] text-neutral-600">
            Directory with config.json, tokenizer.json, and model.safetensors
          </div>
        </div>
      </div>

      <!-- Save Button -->
      {#if hasChanges}
        <button
          on:click={saveConfig}
          disabled={isSaving}
          class="w-full py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-xs transition-colors"
        >
          {isSaving ? 'Saving...' : 'Save Configuration'}
        </button>
      {/if}

      <!-- Help text -->
      <div class="text-[10px] text-neutral-600 leading-relaxed">
        Configure model paths for the built-in llama.cpp server. VLM models enable vision-language
        inference, embedding models enable RAG search.
      </div>
    </div>
  {:else}
    <!-- Collapsed summary -->
    <div class="text-xs text-neutral-500">
      {#if isConfigured}
        <span class="text-neutral-400">{getFileName(vlmModelPath)}</span>
      {:else}
        <span class="text-amber-400">Not configured</span>
      {/if}
    </div>
  {/if}
</div>
