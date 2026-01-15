<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { RagService, type RagState, type IndexingEvent } from '../services/RagService';
  import { ConfigService, type ConfigState } from '../services/ConfigService';
  import { LLMService } from '../services/LLMService';
  import { openChunkPreview } from '../stores/chunkPreviewStore';
  import { expandedSection, toggleSection } from '../stores/accordionStore';

  let ragState: RagState = RagService.getState();
  let configState: ConfigState = ConfigService.getState();
  let unsubscribeRag: (() => void) | null = null;
  let unsubscribeConfig: (() => void) | null = null;
  let isIndexingWithSwitch = false;

  // For external embedding server (legacy mode)
  let embeddingUrl = 'http://127.0.0.1:8081';
  let isTestingServer = false;
  let useExternalEmbedding = false;

  onMount(async () => {
    unsubscribeRag = RagService.subscribe((state) => {
      ragState = state;
    });

    unsubscribeConfig = ConfigService.subscribe((state) => {
      configState = state;
    });

    try {
      await RagService.refreshStatus();
    } catch {
      // Status fetch failed
    }
  });

  onDestroy(() => {
    unsubscribeRag?.();
    unsubscribeConfig?.();
  });

  const testExternalServer = async () => {
    isTestingServer = true;
    try {
      await RagService.setEmbeddingServerUrl(embeddingUrl);
    } finally {
      isTestingServer = false;
    }
  };

  // Index using automatic mode switching (sidecar mode)
  const indexWithSwitch = async () => {
    console.log('indexWithSwitch called - starting');
    isIndexingWithSwitch = true;
    try {
      console.log('Creating channel');
      const channel = new Channel<IndexingEvent>();

      channel.onmessage = (event: IndexingEvent) => {
        console.log('Channel message received:', event);
        if (event.error) {
          ragState.error = event.error;
          ragState.isIndexing = false;
        } else {
          ragState.indexingProgress = {
            current: event.current,
            total: event.total,
            status: event.status,
          };
          ragState.isIndexing = !event.done;

          if (event.done && !event.error) {
            ragState.status.vectors_indexed = true;
            ragState.status.vectors_count = event.total;
          }
        }
        // Trigger reactivity
        ragState = { ...ragState };
      };

      console.log('Calling invoke with index_docs_with_switch');
      await invoke('index_docs_with_switch', { channel });
      console.log('invoke completed successfully');
      await RagService.refreshStatus();
      await ConfigService.refreshServerMode();
      await LLMService.refreshStatus();
    } catch (error) {
      console.error('Indexing with switch failed:', error);
      ragState.error = String(error);
    } finally {
      console.log('indexWithSwitch finally block');
      isIndexingWithSwitch = false;
      ragState.isIndexing = false;
    }
  };

  // Index using external embedding server (legacy)
  const indexExternal = async () => {
    try {
      await RagService.indexDocuments();
    } catch (error) {
      console.error('Indexing failed:', error);
    }
  };

  const clearCache = async () => {
    try {
      await RagService.clearCache();
    } catch (error) {
      console.error('Clear cache failed:', error);
    }
  };

  $: progressPercent = ragState.indexingProgress
    ? Math.round((ragState.indexingProgress.current / Math.max(ragState.indexingProgress.total, 1)) * 100)
    : 0;

  // Can index with sidecar mode switching
  $: canIndexWithSwitch =
    ragState.status.docs_available &&
    configState.config.models.embedding_model_path &&
    !ragState.isIndexing &&
    !isIndexingWithSwitch;

  // Can index with external embedding server
  $: canIndexExternal =
    ragState.status.docs_available &&
    ragState.status.vectorizer_available &&
    !ragState.isIndexing;

  // Check if server is in embedding mode
  $: isEmbeddingMode = configState.serverMode.is_embedding_mode;

  // Check if server is in inference mode (will need to switch)
  $: needsModeSwitch =
    configState.serverMode.mode === 'sidecar_inference' &&
    !ragState.status.vectorizer_available;
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    on:click={() => toggleSection('rag')}
  >
    <div class="flex items-center gap-2">
      <span>Documentation & RAG</span>
      {#if ragState.status.vectors_indexed}
        <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
      {:else if ragState.isIndexing || isIndexingWithSwitch}
        <span class="w-1.5 h-1.5 rounded-full bg-yellow-500 animate-pulse"></span>
      {/if}
    </div>
    <svg
      class="w-3 h-3 transform transition-transform {$expandedSection === 'rag' ? 'rotate-180' : ''}"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if $expandedSection === 'rag'}
    <div class="space-y-3 p-3 bg-neutral-800/30 rounded-lg">
      <!-- Docs Status -->
      <div class="flex items-center gap-2">
        <span
          class="w-2 h-2 rounded-full {ragState.status.docs_available ? 'bg-green-500' : 'bg-neutral-500'}"
        ></span>
        <span class="text-xs text-neutral-400">Svelte 5 Docs</span>
        {#if ragState.status.docs_available}
          <span class="text-xs text-neutral-500">({ragState.status.docs_count} files)</span>
        {:else}
          <span class="text-xs text-neutral-500">Not downloaded</span>
        {/if}
      </div>

      <!-- Vector Index Status -->
      <div class="space-y-2">
        <div class="flex items-center gap-2">
          <span
            class="w-2 h-2 rounded-full {ragState.status.vectors_indexed
              ? 'bg-green-500'
              : ragState.isIndexing || isIndexingWithSwitch
                ? 'bg-yellow-500 animate-pulse'
                : 'bg-neutral-500'}"
          ></span>
          <span class="text-xs text-neutral-400">Vector Index</span>
          {#if ragState.status.vectors_indexed}
            <span class="text-xs text-neutral-500">({ragState.status.vectors_count} docs)</span>
          {:else if ragState.isIndexing || isIndexingWithSwitch}
            <span class="text-xs text-yellow-400">Indexing...</span>
          {:else}
            <span class="text-xs text-neutral-500">Not indexed</span>
          {/if}
        </div>

        <!-- Progress bar during indexing -->
        {#if (ragState.isIndexing || isIndexingWithSwitch) && ragState.indexingProgress}
          <div class="space-y-1">
            <div class="h-1.5 bg-neutral-700 rounded-full overflow-hidden">
              <div
                class="h-full bg-blue-500 transition-all duration-300"
                style="width: {progressPercent}%"
              ></div>
            </div>
            <div class="flex justify-between text-[10px] text-neutral-500">
              <span>{ragState.indexingProgress.status}</span>
              {#if ragState.indexingProgress.total > 0}
                <span>{ragState.indexingProgress.current}/{ragState.indexingProgress.total}</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <!-- Mode indicator when in VLM mode -->
      {#if needsModeSwitch && canIndexWithSwitch}
        <div class="text-xs text-amber-400 bg-amber-900/20 border border-amber-800/50 rounded p-2">
          Server will temporarily switch to embedding mode during indexing
        </div>
      {/if}

      <!-- Error display -->
      {#if ragState.error}
        <div class="text-xs text-red-400 bg-red-900/20 border border-red-800/50 rounded p-2">
          {ragState.error}
        </div>
      {/if}

      <!-- Index buttons -->
      <div class="space-y-2">
        {#if configState.config.models.embedding_model_path}
          <!-- Primary: Index with automatic mode switching -->
          <button
            on:click={indexWithSwitch}
            disabled={!canIndexWithSwitch}
            class="w-full px-2 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-xs transition-colors"
          >
            {#if ragState.isIndexing || isIndexingWithSwitch}
              Indexing...
            {:else if ragState.status.vectors_indexed}
              Re-index (Sidecar)
            {:else}
              Index Now
            {/if}
          </button>
        {/if}

        <!-- Toggle for external embedding server option -->
        <button
          class="w-full text-left text-[10px] text-neutral-600 hover:text-neutral-500 transition-colors"
          on:click={() => (useExternalEmbedding = !useExternalEmbedding)}
        >
          {useExternalEmbedding ? '▼' : '▶'} Use external embedding server
        </button>

        {#if useExternalEmbedding}
          <div class="space-y-2 pl-2 border-l-2 border-neutral-700">
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={embeddingUrl}
                placeholder="http://127.0.0.1:8081"
                class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500"
              />
              <button
                on:click={testExternalServer}
                disabled={isTestingServer}
                class="px-2 py-1 bg-neutral-700 hover:bg-neutral-600 disabled:bg-neutral-800 disabled:text-neutral-500 rounded text-xs transition-colors"
              >
                {isTestingServer ? '...' : 'Test'}
              </button>
            </div>

            {#if ragState.status.vectorizer_available}
              <div class="flex items-center gap-2 text-xs text-green-400">
                <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
                Connected
              </div>
              <button
                on:click={indexExternal}
                disabled={!canIndexExternal}
                class="w-full px-2 py-1.5 bg-neutral-700 hover:bg-neutral-600 disabled:bg-neutral-800 disabled:text-neutral-500 rounded text-xs transition-colors"
              >
                Index with External Server
              </button>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Clear cache -->
      {#if ragState.status.vectors_indexed}
        <button
          on:click={clearCache}
          class="w-full px-2 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded text-xs transition-colors"
        >
          Clear Index Cache
        </button>
      {/if}

      <!-- Chunk Preview button -->
      {#if ragState.status.docs_available}
        <button
          on:click={openChunkPreview}
          class="w-full px-2 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded text-xs transition-colors flex items-center justify-center gap-2"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2" />
          </svg>
          Preview Chunks
        </button>
      {/if}

      <!-- Help text -->
      {#if !configState.config.models.embedding_model_path && !ragState.status.vectorizer_available}
        <p class="text-[10px] text-neutral-600 leading-relaxed">
          Configure an embedding model in Model Configuration, or connect to an external embedding
          server to enable RAG search.
        </p>
      {/if}
    </div>
  {:else}
    <!-- Collapsed summary -->
    <div class="flex items-center gap-3 text-xs text-neutral-500">
      <div class="flex items-center gap-1">
        <span
          class="w-1.5 h-1.5 rounded-full {ragState.status.docs_available ? 'bg-green-500' : 'bg-neutral-600'}"
        ></span>
        <span>Docs</span>
      </div>
      <div class="flex items-center gap-1">
        <span
          class="w-1.5 h-1.5 rounded-full {ragState.status.vectors_indexed
            ? 'bg-green-500'
            : ragState.isIndexing || isIndexingWithSwitch
              ? 'bg-yellow-500 animate-pulse'
              : 'bg-neutral-600'}"
        ></span>
        <span>RAG</span>
      </div>
      {#if ragState.status.vectors_indexed}
        <span class="text-neutral-600">({ragState.status.vectors_count})</span>
      {/if}
    </div>
  {/if}
</div>
