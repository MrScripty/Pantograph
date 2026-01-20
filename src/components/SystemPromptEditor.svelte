<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { updateNodeData } from '../stores/workflowStore';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let promptContent = $state('');
  let isLoading = $state(true);
  let isSaving = $state(false);
  let error: string | null = $state(null);
  let saveSuccess = $state(false);
  let textareaEl: HTMLTextAreaElement | null = $state(null);
  let lineNumbersEl: HTMLDivElement | null = $state(null);

  // Compute line count and character count
  let lines = $derived(promptContent.split('\n'));
  let lineCount = $derived(lines.length);
  let charCount = $derived(promptContent.length);

  // Sync scroll between textarea and line numbers
  function handleScroll() {
    if (lineNumbersEl && textareaEl) {
      lineNumbersEl.scrollTop = textareaEl.scrollTop;
    }
  }

  onMount(async () => {
    try {
      promptContent = await invoke<string>('get_system_prompt');
      // Update the node with the prompt preview
      updateNodeData('system-prompt', { promptPreview: promptContent });
    } catch (e) {
      error = `Failed to load system prompt: ${e}`;
    } finally {
      isLoading = false;
    }
  });

  async function handleSave() {
    isSaving = true;
    error = null;
    saveSuccess = false;

    try {
      await invoke('set_system_prompt', { content: promptContent });
      // Update the node with the new prompt preview
      updateNodeData('system-prompt', { promptPreview: promptContent });
      saveSuccess = true;
      setTimeout(() => {
        saveSuccess = false;
      }, 2000);
    } catch (e) {
      error = `Failed to save system prompt: ${e}`;
    } finally {
      isSaving = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
    if (e.ctrlKey && e.key === 's') {
      e.preventDefault();
      handleSave();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="fixed inset-0 bg-black/70 backdrop-blur-sm z-[100] flex items-center justify-center p-8">
  <div class="bg-neutral-900 border border-neutral-700 rounded-xl w-full max-w-4xl max-h-[80vh] flex flex-col shadow-2xl">
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-neutral-700">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded bg-purple-600 flex items-center justify-center">
          <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </div>
        <div>
          <h2 class="text-lg font-semibold text-neutral-100">System Prompt Editor</h2>
          <p class="text-xs text-neutral-500">Edit the agent's system prompt</p>
        </div>
      </div>
      <button
        onclick={onClose}
        class="p-2 text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 rounded-lg transition-colors"
        aria-label="Close"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 flex flex-col overflow-hidden p-6 min-h-[400px]">
      {#if isLoading}
        <div class="flex items-center justify-center h-full">
          <div class="flex items-center gap-3 text-neutral-400">
            <div class="w-5 h-5 border-2 border-neutral-600 border-t-purple-500 rounded-full animate-spin"></div>
            <span>Loading system prompt...</span>
          </div>
        </div>
      {:else}
        <div class="flex-1 flex rounded-lg border border-neutral-700 overflow-hidden focus-within:border-purple-500 transition-colors">
          <!-- Line numbers -->
          <div
            bind:this={lineNumbersEl}
            class="bg-neutral-900 text-neutral-500 text-sm font-mono py-4 px-3 text-right select-none overflow-hidden border-r border-neutral-700"
          >
            {#each lines as _, i}
              <div class="leading-[1.5rem]">{i + 1}</div>
            {/each}
          </div>
          <!-- Textarea -->
          <textarea
            bind:this={textareaEl}
            bind:value={promptContent}
            onscroll={handleScroll}
            class="flex-1 bg-neutral-950 p-4 text-sm font-mono text-neutral-200 resize-none focus:outline-none leading-[1.5rem]"
            placeholder="Enter the system prompt..."
            spellcheck="false"
          ></textarea>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-between px-6 py-4 border-t border-neutral-700">
      <div class="flex items-center gap-4 text-xs text-neutral-500">
        <span>{lineCount} {lineCount === 1 ? 'line' : 'lines'}, {charCount.toLocaleString()} {charCount === 1 ? 'character' : 'characters'}</span>
        <span class="text-neutral-600">|</span>
        {#if error}
          <span class="text-red-400">{error}</span>
        {:else if saveSuccess}
          <span class="text-green-400">Saved successfully!</span>
        {:else}
          <span>Ctrl+S to save, Esc to close</span>
        {/if}
      </div>
      <div class="flex items-center gap-3">
        <button
          onclick={onClose}
          class="px-4 py-2 text-sm text-neutral-400 hover:text-neutral-200 transition-colors"
        >
          Cancel
        </button>
        <button
          onclick={handleSave}
          disabled={isLoading || isSaving}
          class="px-4 py-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm font-medium text-white transition-colors"
        >
          {isSaving ? 'Saving...' : 'Save Changes'}
        </button>
      </div>
    </div>
  </div>
</div>
