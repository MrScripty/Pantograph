<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    chunkPreviewState,
    closeChunkPreview,
    setDocs,
    setLoading,
    setPreview,
    setError,
    type DocInfo,
    type ChunkPreview,
  } from '../stores/chunkPreviewStore';

  let state = $state<typeof $chunkPreviewState>($chunkPreviewState);
  let selectedDocId = $state<string>('');

  // Subscribe to store
  $effect(() => {
    const unsubscribe = chunkPreviewState.subscribe((s) => {
      state = s;
      if (s.docId && !selectedDocId) {
        selectedDocId = s.docId;
      }
    });
    return unsubscribe;
  });

  onMount(async () => {
    await loadDocs();
  });

  async function loadDocs() {
    try {
      const docs = await invoke<DocInfo[]>('list_chunkable_docs');
      setDocs(docs);
      // Auto-select first doc if none selected
      if (docs.length > 0 && !selectedDocId) {
        selectedDocId = docs[0].id;
        await loadPreview(docs[0].id);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function loadPreview(docId: string) {
    if (!docId) return;
    setLoading(true);
    try {
      const preview = await invoke<ChunkPreview>('preview_doc_chunks', { docId });
      setPreview(docId, preview);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function handleDocChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    selectedDocId = select.value;
    await loadPreview(selectedDocId);
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      closeChunkPreview();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closeChunkPreview();
    }
  }

  // Group docs by section for the dropdown
  function groupDocsBySection(docs: DocInfo[]): Map<string, DocInfo[]> {
    const grouped = new Map<string, DocInfo[]>();
    for (const doc of docs) {
      const section = doc.section || 'General';
      if (!grouped.has(section)) {
        grouped.set(section, []);
      }
      grouped.get(section)!.push(doc);
    }
    return grouped;
  }

  $effect(() => {
    if (state.open) {
      loadDocs();
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if state.open}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    tabindex="0"
    role="dialog"
    aria-modal="true"
  >
    <!-- Modal - larger for document preview -->
    <div class="w-full max-w-4xl mx-4 h-[80vh] bg-neutral-900 rounded-xl shadow-2xl border border-neutral-700 flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-neutral-700 shrink-0">
        <div class="flex items-center gap-4">
          <h2 class="text-lg font-semibold text-neutral-100">Chunk Preview</h2>
          {#if state.preview}
            <span class="text-sm text-neutral-500">
              {state.preview.total_chunks} chunks
            </span>
          {/if}
        </div>
        <button
          onclick={closeChunkPreview}
          class="p-1 rounded-lg text-neutral-400 hover:text-neutral-100 hover:bg-neutral-700 transition-colors"
          aria-label="Close"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Document Selector -->
      <div class="px-6 py-3 border-b border-neutral-800 shrink-0">
        <label for="chunk-preview-doc-select" class="block text-xs text-neutral-500 uppercase tracking-wider mb-2">Document</label>
        <select
          id="chunk-preview-doc-select"
          class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-neutral-500"
          value={selectedDocId}
          onchange={handleDocChange}
        >
          {#if state.docs.length === 0}
            <option value="">No documents available</option>
          {:else}
            {#each [...groupDocsBySection(state.docs)] as [section, docs]}
              <optgroup label={section}>
                {#each docs as doc}
                  <option value={doc.id}>
                    {doc.title} ({Math.round(doc.char_count / 1000)}k chars)
                  </option>
                {/each}
              </optgroup>
            {/each}
          {/if}
        </select>
      </div>

      <!-- Content Area -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if state.loading}
          <div class="flex items-center justify-center h-full">
            <div class="text-neutral-500">Loading preview...</div>
          </div>
        {:else if state.error}
          <div class="bg-red-900/20 border border-red-800/50 rounded-lg p-4">
            <div class="text-red-400">{state.error}</div>
          </div>
        {:else if state.preview}
          <div class="space-y-6">
            {#each state.preview.chunks as chunk}
              <!-- Chunk Divider -->
              <div class="relative">
                <div class="absolute inset-0 flex items-center">
                  <div class="w-full border-t-2 border-amber-600/50"></div>
                </div>
                <div class="relative flex justify-center">
                  <span class="bg-neutral-900 px-4 py-1 text-xs font-medium text-amber-500 uppercase tracking-wider rounded-full border border-amber-600/50">
                    Chunk {chunk.chunk_index + 1} &middot; {chunk.char_count.toLocaleString()} chars
                    {#if chunk.has_code}
                      &middot; <span class="text-blue-400">Has Code</span>
                    {/if}
                  </span>
                </div>
              </div>

              <!-- Chunk Content -->
              <div class="space-y-2">
                <!-- Header Context Breadcrumb -->
                {#if chunk.header_context}
                  <div class="text-xs text-neutral-500 font-mono">
                    {chunk.header_context}
                  </div>
                {/if}

                <!-- Chunk Title -->
                {#if chunk.title}
                  <h3 class="text-lg font-semibold text-neutral-200">
                    {chunk.title}
                  </h3>
                {/if}

                <!-- Chunk Content -->
                <div class="bg-neutral-800/50 rounded-lg p-4 border border-neutral-700/50">
                  <pre class="text-sm text-neutral-300 whitespace-pre-wrap font-mono leading-relaxed overflow-x-auto">{chunk.full_content}</pre>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="flex items-center justify-center h-full">
            <div class="text-neutral-500">Select a document to preview chunks</div>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-neutral-700 shrink-0">
        <div class="flex items-center justify-between">
          <div class="text-xs text-neutral-500">
            Chunks are split at H2/H3 headers with context preservation
          </div>
          <button
            onclick={closeChunkPreview}
            class="px-4 py-2 bg-neutral-700 hover:bg-neutral-600 text-neutral-200 rounded-lg text-sm font-medium transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
