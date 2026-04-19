<script lang="ts">
  import { onMount } from 'svelte';
  import {
    managedRuntimeService,
    type ManagedRuntimeManagerRuntimeView,
    type ManagedRuntimeProgress
  } from '../services/managedRuntime';

  let status: ManagedRuntimeManagerRuntimeView = $state({
    id: 'llama_cpp',
    display_name: 'llama.cpp',
    install_state: 'installed',
    readiness_state: 'ready',
    available: true,
    can_install: false,
    can_remove: false,
    missing_files: [],
    unavailable_reason: null,
    versions: [],
    selection: {
      selected_version: null,
      active_version: null,
      default_version: null
    },
    active_job: null,
    install_history: []
  });
  let downloading = $state(false);
  let progress: ManagedRuntimeProgress = $state({
    runtime_id: 'llama_cpp',
    status: '',
    current: 0,
    total: 0,
    done: false,
    error: null
  });
  let error: string | null = $state(null);

  onMount(async () => {
    try {
      status = await managedRuntimeService.inspectRuntime('llama_cpp');
    } catch (e) {
      console.error('Failed to check binary status:', e);
    }
  });

  async function download() {
    downloading = true;
    error = null;
    progress = {
      runtime_id: 'llama_cpp',
      status: 'Starting download...',
      current: 0,
      total: 0,
      done: false,
      error: null
    };

    try {
      await managedRuntimeService.installRuntime('llama_cpp', async (event) => {
        progress = event;
        if (event.error) {
          error = event.error;
          downloading = false;
        }
        if (event.done && !event.error) {
          downloading = false;
          status = await managedRuntimeService.inspectRuntime('llama_cpp');
        }
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      downloading = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  let progressPercent = $derived(progress.total > 0 ? (progress.current / progress.total) * 100 : 0);
</script>

{#if !status.available}
  <div class="bg-amber-900/20 border border-amber-800/50 rounded-lg p-3">
    <div class="flex items-center gap-2 text-xs text-amber-400 uppercase tracking-wider mb-2">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
      <span>Dependencies Required</span>
    </div>

    {#if error}
      <div class="text-sm text-red-400 mb-2">
        Error: {error}
      </div>
    {/if}

    {#if downloading}
      <div class="text-sm text-neutral-300 mb-2">
        {progress.status}
      </div>
      {#if progress.total > 0}
        <div class="text-xs text-neutral-500 mb-1">
          {formatBytes(progress.current)} / {formatBytes(progress.total)}
        </div>
      {/if}
      <div class="bg-neutral-700 rounded-full h-2 overflow-hidden">
        <div
          class="bg-amber-500 h-2 transition-all duration-300"
          style="width: {progressPercent}%"
        />
      </div>
    {:else}
      <p class="text-sm text-neutral-400 mb-3">
        llama.cpp is required for local inference.
      </p>
      {#if status.missing_files.length > 0}
        <details class="text-xs text-neutral-500 mb-3">
          <summary class="cursor-pointer hover:text-neutral-400">
            {status.missing_files.length} missing file(s)
          </summary>
          <ul class="mt-1 ml-3 list-disc">
            {#each status.missing_files as file (file)}
              <li class="font-mono">{file}</li>
            {/each}
          </ul>
        </details>
      {/if}
      <button type="button"
        onclick={download}
        class="w-full py-2 px-3 bg-amber-600 hover:bg-amber-500 text-white rounded text-sm font-medium transition-colors flex items-center justify-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
        </svg>
        Download (~60 MB)
      </button>
    {/if}
  </div>
{/if}
