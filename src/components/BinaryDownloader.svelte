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
    job_artifact: null,
    install_history: []
  });
  let downloading = $state(false);
  let cancelling = $state(false);
  let selectionUpdating = $state(false);
  let progress: ManagedRuntimeProgress = $state({
    runtime_id: 'llama_cpp',
    status: '',
    current: 0,
    total: 0,
    done: false,
    error: null,
    runtime: status
  });
  let error: string | null = $state(null);

  async function loadStatus() {
    try {
      status = await managedRuntimeService.inspectRuntime('llama_cpp');
    } catch (e) {
      console.error('Failed to check binary status:', e);
    }
  }

  onMount(async () => {
    await loadStatus();
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
      error: null,
      runtime: status
    };

    try {
      await managedRuntimeService.installRuntime('llama_cpp', async (event) => {
        progress = event;
        status = event.runtime;
        if (event.error) {
          error = event.error;
          downloading = false;
          cancelling = false;
        }
        if (event.done && !event.error) {
          downloading = false;
          cancelling = false;
        }
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      downloading = false;
      cancelling = false;
    }
  }

  async function cancelDownload() {
    cancelling = true;
    error = null;

    try {
      await managedRuntimeService.cancelRuntimeJob('llama_cpp');
      progress = {
        ...progress,
        status: 'Cancellation requested...'
      };
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      cancelling = false;
    }
  }

  async function updateSelectedVersion(version: string | null) {
    selectionUpdating = true;
    error = null;

    try {
      status = await managedRuntimeService.selectRuntimeVersion('llama_cpp', version);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      selectionUpdating = false;
    }
  }

  async function updateDefaultVersion(version: string | null) {
    selectionUpdating = true;
    error = null;

    try {
      status = await managedRuntimeService.setDefaultRuntimeVersion('llama_cpp', version);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      selectionUpdating = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatHistoryEvent(event: string): string {
    return event
      .split('_')
      .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
      .join(' ');
  }

  function formatHistoryTime(atMs: number): string {
    return new Date(atMs).toLocaleString();
  }

  let progressPercent = $derived(progress.total > 0 ? (progress.current / progress.total) * 100 : 0);
  let installedVersions = $derived(
    status.versions.filter(
      (version) => version.install_state === 'installed' || version.install_state === 'system_provided'
    )
  );
  let selectableVersions = $derived(
    installedVersions.filter((version) => version.version !== null)
  );
  let latestHistoryEntry = $derived(status.install_history.at(-1) ?? null);
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
      <button
        type="button"
        onclick={cancelDownload}
        class="mt-3 w-full py-2 px-3 border border-amber-700 text-amber-300 hover:bg-amber-950/40 rounded text-sm font-medium transition-colors disabled:text-neutral-600 disabled:border-neutral-800"
        disabled={cancelling}
      >
        {cancelling ? 'Requesting cancel...' : 'Cancel download'}
      </button>
    {:else}
      <p class="text-sm text-neutral-400 mb-3">
        llama.cpp is required for local inference.
      </p>
      <div class="grid grid-cols-2 gap-2 text-xs text-neutral-400 mb-3">
        <div>
          <span class="text-neutral-500">Readiness</span>
          <div class="text-neutral-300">{status.readiness_state}</div>
        </div>
        <div>
          <span class="text-neutral-500">Selected</span>
          <div class="text-neutral-300">{status.selection.selected_version ?? 'None'}</div>
        </div>
        <div>
          <span class="text-neutral-500">Default</span>
          <div class="text-neutral-300">{status.selection.default_version ?? 'None'}</div>
        </div>
        <div>
          <span class="text-neutral-500">Installed</span>
          <div class="text-neutral-300">{installedVersions.length}</div>
        </div>
      </div>
      {#if status.active_job}
        <div class="text-xs text-neutral-500 mb-3">
          Active job: {status.active_job.status}
        </div>
        {#if status.job_artifact}
          <div class="text-xs text-neutral-500 mb-3">
            Retained artifact: {formatBytes(status.job_artifact.downloaded_bytes)} / {formatBytes(status.job_artifact.total_bytes)}
            <div class="text-[11px] text-neutral-600">
              {status.job_artifact.archive_name} ({status.job_artifact.version})
            </div>
          </div>
        {/if}
        {#if status.active_job.cancellable}
          <button
            type="button"
            onclick={cancelDownload}
            class="mb-3 w-full py-2 px-3 border border-amber-700 text-amber-300 hover:bg-amber-950/40 rounded text-sm font-medium transition-colors disabled:text-neutral-600 disabled:border-neutral-800"
            disabled={cancelling}
          >
            {cancelling ? 'Requesting cancel...' : 'Cancel download'}
          </button>
        {/if}
      {/if}
      {#if latestHistoryEntry}
        <div class="text-xs text-neutral-500 mb-3">
          Last event: {formatHistoryEvent(latestHistoryEntry.event)}
          {#if latestHistoryEntry.version}
            ({latestHistoryEntry.version})
          {/if}
          <div class="text-[11px] text-neutral-600">
            {formatHistoryTime(latestHistoryEntry.at_ms)}
          </div>
          {#if latestHistoryEntry.detail}
            <div class="text-[11px] text-neutral-500">{latestHistoryEntry.detail}</div>
          {/if}
        </div>
      {/if}
      {#if selectableVersions.length > 0}
        <div class="space-y-2 mb-3">
          <label class="block text-xs text-neutral-500">
            Selected version
            <select
              class="mt-1 w-full rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm text-neutral-200"
              value={status.selection.selected_version ?? ''}
              disabled={selectionUpdating || downloading}
              onchange={(event) =>
                updateSelectedVersion((event.currentTarget as HTMLSelectElement).value || null)}
            >
              <option value="">Automatic</option>
              {#each selectableVersions as version (version.display_label)}
                <option value={version.version ?? ''}>{version.display_label}</option>
              {/each}
            </select>
          </label>
          <label class="block text-xs text-neutral-500">
            Default version
            <select
              class="mt-1 w-full rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm text-neutral-200"
              value={status.selection.default_version ?? ''}
              disabled={selectionUpdating || downloading}
              onchange={(event) =>
                updateDefaultVersion((event.currentTarget as HTMLSelectElement).value || null)}
            >
              <option value="">Unset</option>
              {#each selectableVersions as version (version.display_label)}
                <option value={version.version ?? ''}>{version.display_label}</option>
              {/each}
            </select>
          </label>
        </div>
      {/if}
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
