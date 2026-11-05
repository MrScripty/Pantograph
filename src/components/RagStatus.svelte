<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { RagService, type RagState, type IndexingEvent } from '../services/RagService';
  import { ConfigService, type ConfigState } from '../services/ConfigService';
  import { LLMService } from '../services/LLMService';
  import { openChunkPreview } from '../stores/chunkPreviewStore';
  import { expandedSection, toggleSection } from '../stores/accordionStore';
  import { StatusGlyph, type GlyphState } from './workbench/ui';

  let ragState: RagState = $state(RagService.getState());
  let configState: ConfigState = $state(ConfigService.getState());
  let unsubscribeRag: (() => void) | null = null;
  let unsubscribeConfig: (() => void) | null = null;
  let isIndexingWithSwitch = $state(false);

  // For external embedding server (legacy mode)
  let embeddingUrl = $state('http://127.0.0.1:8081');
  let isTestingServer = $state(false);
  let useExternalEmbedding = $state(false);

  // For docs download
  let isDownloadingDocs = $state(false);

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

  // Download Svelte docs manually
  const downloadDocs = async () => {
    isDownloadingDocs = true;
    ragState.error = null;
    try {
      await invoke('update_svelte_docs');
      await RagService.refreshStatus();
    } catch (error) {
      console.error('Failed to download docs:', error);
      ragState.error = String(error);
      ragState = { ...ragState }; // Trigger reactivity
    } finally {
      isDownloadingDocs = false;
    }
  };

  let progressPercent = $derived(ragState.indexingProgress
    ? Math.round((ragState.indexingProgress.current / Math.max(ragState.indexingProgress.total, 1)) * 100)
    : 0
  );

  // Can index with sidecar mode switching
  let canIndexWithSwitch = $derived(
    ragState.status.docs_available &&
    configState.config.models.embedding_model_path &&
    !ragState.isIndexing &&
    !isIndexingWithSwitch
  );

  // Can index with external embedding server
  let canIndexExternal = $derived(
    ragState.status.docs_available &&
    ragState.status.vectorizer_available &&
    !ragState.isIndexing
  );

  // Check if server is in inference mode (will need to switch)
  let needsModeSwitch = $derived(
    configState.serverMode.mode === 'sidecar_inference' &&
    !ragState.status.vectorizer_available
  );

  let indexing = $derived(ragState.isIndexing || isIndexingWithSwitch);

  let indexGlyph = $derived<GlyphState>(
    ragState.status.vectors_indexed ? 'succeeded' : indexing ? 'running' : 'idle'
  );

  let docsGlyph = $derived<GlyphState>(
    ragState.status.docs_available ? 'succeeded' : isDownloadingDocs ? 'running' : 'idle'
  );
</script>

<section class="pg-card" aria-labelledby="settings-rag-title">
  <div class="flex items-center gap-2">
    <h3 class="m-0 min-w-0 flex-1">
      <button
        type="button"
        id="settings-rag-title"
        class="flex w-full items-center gap-2 text-left text-[13px] font-bold text-pg-text-bright"
        aria-expanded={$expandedSection === 'rag'}
        onclick={() => toggleSection('rag')}
      >
        Documentation and RAG
        <span class="pg-muted text-[11px] font-normal" aria-hidden="true">
          {$expandedSection === 'rag' ? '▴' : '▾'}
        </span>
      </button>
    </h3>
    <span class="flex items-center gap-1.5 pg-small text-pg-text-soft">
      <StatusGlyph state={indexGlyph} small />
      {ragState.status.vectors_indexed ? 'Indexed' : indexing ? 'Indexing' : 'Not indexed'}
    </span>
  </div>

  {#if $expandedSection === 'rag'}
    <div class="mt-3 flex flex-col gap-3 border-t border-pg-border-soft pt-3">
      <div class="flex items-center justify-between gap-2">
        <p class="m-0 flex items-center gap-2 pg-small">
          <StatusGlyph state={docsGlyph} small />
          <span>Svelte 5 docs</span>
          <span class="pg-muted">
            {#if ragState.status.docs_available}
              {ragState.status.docs_count} files
            {:else if isDownloadingDocs}
              Downloading...
            {:else}
              Not downloaded
            {/if}
          </span>
        </p>
        {#if !ragState.status.docs_available && !isDownloadingDocs}
          <button type="button" onclick={downloadDocs} class="pg-btn pg-btn-sm">Download</button>
        {/if}
      </div>

      <div class="flex flex-col gap-2">
        <p class="m-0 flex items-center gap-2 pg-small">
          <StatusGlyph state={indexGlyph} small />
          <span>Vector index</span>
          <span class="pg-muted">
            {#if ragState.status.vectors_indexed}
              {ragState.status.vectors_count} docs
            {:else if indexing}
              Indexing...
            {:else}
              Not indexed
            {/if}
          </span>
        </p>

        {#if indexing && ragState.indexingProgress}
          <div class="flex flex-col gap-1">
            <div class="pg-meter" aria-hidden="true">
              <i class="pg-meter-accent" style="width: {progressPercent}%"></i>
            </div>
            <div class="flex justify-between gap-2 pg-muted pg-small">
              <span>{ragState.indexingProgress.status}</span>
              {#if ragState.indexingProgress.total > 0}
                <span class="pg-mono">{ragState.indexingProgress.current}/{ragState.indexingProgress.total}</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      {#if needsModeSwitch && canIndexWithSwitch}
        <p class="m-0 flex items-center gap-2 pg-muted pg-small">
          <StatusGlyph state="warn" small />
          Server will temporarily switch to embedding mode during indexing
        </p>
      {/if}

      {#if ragState.error}
        <p class="m-0 pg-muted pg-small" role="alert">{ragState.error}</p>
      {/if}

      <div class="flex flex-col gap-2">
        {#if configState.config.models.embedding_model_path}
          <div>
            <button
              type="button"
              onclick={indexWithSwitch}
              disabled={!canIndexWithSwitch}
              class="pg-btn pg-btn-primary"
            >
              {#if indexing}
                Indexing...
              {:else if ragState.status.vectors_indexed}
                Re-index (Sidecar)
              {:else}
                Index now
              {/if}
            </button>
          </div>
        {/if}

        <button
          type="button"
          class="flex items-center gap-1.5 self-start text-left pg-small text-pg-text-soft hover:text-pg-text-bright"
          aria-expanded={useExternalEmbedding}
          onclick={() => (useExternalEmbedding = !useExternalEmbedding)}
        >
          <span class="pg-muted text-[11px]" aria-hidden="true">{useExternalEmbedding ? '▾' : '▸'}</span>
          Use external embedding server
        </button>

        {#if useExternalEmbedding}
          <div class="flex flex-col gap-2 border-l-2 border-pg-border pl-3">
            <label for="rag-embedding-server-url" class="pg-fl">Embedding server URL</label>
            <div class="flex gap-2">
              <input
                id="rag-embedding-server-url"
                type="text"
                bind:value={embeddingUrl}
                placeholder="http://127.0.0.1:8081"
                class="pg-field pg-mono min-w-0 flex-1"
              />
              <button type="button" onclick={testExternalServer} disabled={isTestingServer} class="pg-btn">
                {isTestingServer ? 'Testing...' : 'Test'}
              </button>
            </div>

            {#if ragState.status.vectorizer_available}
              <p class="m-0 flex items-center gap-2 pg-small">
                <StatusGlyph state="succeeded" small />
                Connected
              </p>
              <div>
                <button type="button" onclick={indexExternal} disabled={!canIndexExternal} class="pg-btn">
                  Index with external server
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      {#if ragState.status.vectors_indexed || ragState.status.docs_available}
        <div class="flex flex-wrap gap-2">
          {#if ragState.status.vectors_indexed}
            <button type="button" onclick={clearCache} class="pg-btn">Clear index cache</button>
          {/if}
          {#if ragState.status.docs_available}
            <button type="button" onclick={openChunkPreview} class="pg-btn">Preview chunks</button>
          {/if}
        </div>
      {/if}

      {#if !configState.config.models.embedding_model_path && !ragState.status.vectorizer_available}
        <p class="m-0 pg-muted pg-small">
          Configure an embedding model in Model configuration, or connect to an external embedding
          server to enable RAG search.
        </p>
      {/if}
    </div>
  {:else}
    <p class="m-0 mt-1 flex items-center gap-3 pg-muted pg-small">
      <span>Docs {ragState.status.docs_available ? 'downloaded' : 'not downloaded'}</span>
      <span>
        RAG {ragState.status.vectors_indexed
          ? `indexed (${ragState.status.vectors_count})`
          : indexing
            ? 'indexing'
            : 'not indexed'}
      </span>
    </p>
  {/if}
</section>
