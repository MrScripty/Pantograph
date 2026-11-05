<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ConfigService, type ConfigState, type ModelConfig } from '../services/ConfigService';
  import { expandedSection, toggleSection } from '../stores/accordionStore';
  import { StatusGlyph } from './workbench/ui';

  let state: ConfigState = $state(ConfigService.getState());
  let unsubscribe: (() => void) | null = null;
  let isSaving = $state(false);

  // Local form state
  let vlmModelPath: string = $state('');
  let vlmMmprojPath: string = $state('');
  let embeddingModelPath: string = $state('');
  let candleEmbeddingModelPath: string = $state('');

  onMount(() => {
    unsubscribe = ConfigService.subscribe((nextState) => {
      state = nextState;
      // Sync local form state
      vlmModelPath = nextState.config.models.vlm_model_path || '';
      vlmMmprojPath = nextState.config.models.vlm_mmproj_path || '';
      embeddingModelPath = nextState.config.models.embedding_model_path || '';
      candleEmbeddingModelPath = nextState.config.models.candle_embedding_model_path || '';
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

  let hasChanges = $derived(
    vlmModelPath !== (state.config.models.vlm_model_path || '') ||
    vlmMmprojPath !== (state.config.models.vlm_mmproj_path || '') ||
    embeddingModelPath !== (state.config.models.embedding_model_path || '') ||
    candleEmbeddingModelPath !== (state.config.models.candle_embedding_model_path || '')
  );

  let isConfigured = $derived(vlmModelPath && vlmMmprojPath);

</script>

<section class="pg-card" aria-labelledby="settings-model-title">
  <div class="flex items-center gap-2">
    <h3 class="m-0 min-w-0 flex-1">
      <button
        type="button"
        id="settings-model-title"
        class="flex w-full items-center gap-2 text-left text-[13px] font-bold text-pg-text-bright"
        aria-expanded={$expandedSection === 'model'}
        onclick={() => toggleSection('model')}
      >
        Model configuration
        <span class="pg-muted text-[11px] font-normal" aria-hidden="true">
          {$expandedSection === 'model' ? '▴' : '▾'}
        </span>
      </button>
    </h3>
    <span class="flex items-center gap-1.5 pg-small text-pg-text-soft">
      <StatusGlyph state={isConfigured ? 'succeeded' : 'warn'} small />
      {isConfigured ? 'Configured' : 'Not configured'}
    </span>
  </div>

  {#if $expandedSection === 'model'}
    <div class="mt-3 flex flex-col gap-4 border-t border-pg-border-soft pt-3">
      <div class="flex flex-col gap-2">
        <h4 class="m-0 text-[12.5px] font-bold text-pg-text-soft">Vision-language model (VLM)</h4>

      <div class="flex flex-col gap-1">
        <label for="vlm-model-path" class="pg-fl">Model file</label>
        <div class="flex gap-2">
          <input
            id="vlm-model-path"
            type="text"
            bind:value={vlmModelPath}
            placeholder="Select model file..."
            class="pg-field pg-mono min-w-0 flex-1 truncate"
            title={vlmModelPath}
          />
          <button type="button" onclick={pickVlmModel} class="pg-btn" aria-label="Browse for VLM model file">
            Browse
          </button>
        </div>
        {#if vlmModelPath}
          <p class="m-0 pg-muted pg-small truncate">{getFileName(vlmModelPath)}</p>
        {/if}
      </div>

      <div class="flex flex-col gap-1">
        <label for="vlm-mmproj-path" class="pg-fl">MMProj file</label>
        <div class="flex gap-2">
          <input
            id="vlm-mmproj-path"
            type="text"
            bind:value={vlmMmprojPath}
            placeholder="Select mmproj file..."
            class="pg-field pg-mono min-w-0 flex-1 truncate"
            title={vlmMmprojPath}
          />
          <button type="button" onclick={pickMmproj} class="pg-btn" aria-label="Browse for MMProj file">
            Browse
          </button>
        </div>
        {#if vlmMmprojPath}
          <p class="m-0 pg-muted pg-small truncate">{getFileName(vlmMmprojPath)}</p>
        {/if}
      </div>
      </div>

      <div class="flex flex-col gap-2 border-t border-pg-border-soft pt-3">
        <h4 class="m-0 text-[12.5px] font-bold text-pg-text-soft">Embedding model (for RAG)</h4>

      <div class="flex flex-col gap-1">
        <label for="embedding-model-path" class="pg-fl">GGUF model file (llama.cpp)</label>
        <div class="flex gap-2">
          <input
            id="embedding-model-path"
            type="text"
            bind:value={embeddingModelPath}
            placeholder="Select embedding model..."
            class="pg-field pg-mono min-w-0 flex-1 truncate"
            title={embeddingModelPath}
          />
          <button type="button" onclick={pickEmbeddingModel} class="pg-btn" aria-label="Browse for GGUF embedding model file">
            Browse
          </button>
        </div>
        {#if embeddingModelPath}
          <p class="m-0 pg-muted pg-small truncate">{getFileName(embeddingModelPath)}</p>
        {/if}
      </div>

      <div class="flex flex-col gap-1">
        <label for="candle-embedding-path" class="pg-fl">SafeTensors model directory (Candle)</label>
        <div class="flex gap-2">
          <input
            id="candle-embedding-path"
            type="text"
            bind:value={candleEmbeddingModelPath}
            placeholder="e.g., ~/models/bge-small-en-v1.5/"
            class="pg-field pg-mono min-w-0 flex-1 truncate"
            title={candleEmbeddingModelPath}
          />
          <button type="button" onclick={pickCandleEmbeddingModel} class="pg-btn" aria-label="Browse for SafeTensors model directory">
            Browse
          </button>
        </div>
        {#if candleEmbeddingModelPath}
          <p class="m-0 pg-muted pg-small truncate">{getFileName(candleEmbeddingModelPath)}</p>
        {/if}
        <p class="m-0 pg-muted pg-small">Directory with config.json, tokenizer.json, and model.safetensors</p>
      </div>
      </div>

      {#if hasChanges}
        <div>
          <button type="button" onclick={saveConfig} disabled={isSaving} class="pg-btn pg-btn-primary">
            {isSaving ? 'Saving...' : 'Save configuration'}
          </button>
        </div>
      {/if}

      <p class="m-0 pg-muted pg-small">
        Configure model paths for the built-in llama.cpp server. VLM models enable vision-language
        inference, embedding models enable RAG search.
      </p>
    </div>
  {:else}
    <p class="m-0 mt-1 pg-muted pg-small truncate">
      {isConfigured ? getFileName(vlmModelPath) : 'Not configured'}
    </p>
  {/if}
</section>
