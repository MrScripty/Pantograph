<script lang="ts">
  import {
    managedRuntimeService,
    type ManagedRuntimeId,
    type ManagedRuntimeManagerRuntimeView,
    type ManagedRuntimeVersionStatus,
  } from '../../services/managedRuntime';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
  };

  const DOWNLOAD_SIZE_LABELS: Record<ManagedRuntimeId, string> = {
    llama_cpp: '~60 MB',
    ollama: '~1.6 GB',
  };

  const HISTORY_LIMIT = 4;

  let { runtime }: Props = $props();

  let installRequested = $state(false);
  let removeRequested = $state(false);
  let pauseRequested = $state(false);
  let cancelRequested = $state(false);
  let selectionUpdating = $state(false);
  let error: string | null = $state(null);

  async function installRuntime() {
    installRequested = true;
    error = null;

    try {
      await managedRuntimeService.installRuntime(runtime.id, () => {});
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      installRequested = false;
    }
  }

  async function removeRuntime() {
    removeRequested = true;
    error = null;

    try {
      await managedRuntimeService.removeRuntime(runtime.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      removeRequested = false;
    }
  }

  async function pauseRuntime() {
    pauseRequested = true;
    error = null;

    try {
      await managedRuntimeService.pauseRuntimeJob(runtime.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      pauseRequested = false;
    }
  }

  async function cancelRuntime() {
    cancelRequested = true;
    error = null;

    try {
      await managedRuntimeService.cancelRuntimeJob(runtime.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      cancelRequested = false;
    }
  }

  async function updateSelectedVersion(version: string | null) {
    selectionUpdating = true;
    error = null;

    try {
      await managedRuntimeService.selectRuntimeVersion(runtime.id, version);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      selectionUpdating = false;
    }
  }

  async function updateDefaultVersion(version: string | null) {
    selectionUpdating = true;
    error = null;

    try {
      await managedRuntimeService.setDefaultRuntimeVersion(runtime.id, version);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      selectionUpdating = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) {
      return '0 B';
    }

    const kilo = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const sizeIndex = Math.floor(Math.log(bytes) / Math.log(kilo));

    return `${parseFloat((bytes / Math.pow(kilo, sizeIndex)).toFixed(1))} ${sizes[sizeIndex]}`;
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

  function versionBadgeLabel(version: ManagedRuntimeVersionStatus): string {
    if (version.install_state === 'system_provided') {
      return 'System';
    }

    if (version.install_state === 'installed') {
      return 'Installed';
    }

    if (version.readiness_state === 'ready') {
      return 'Ready';
    }

    return version.readiness_state;
  }

  let installedVersions = $derived(
    runtime.versions.filter(
      (version) =>
        version.install_state === 'installed' ||
        version.install_state === 'system_provided'
    )
  );

  let selectableVersions = $derived(
    installedVersions.filter((version) => version.version !== null)
  );

  let progressPercent = $derived(
    runtime.active_job && runtime.active_job.total > 0
      ? (runtime.active_job.current / runtime.active_job.total) * 100
      : 0
  );

  let recentHistory = $derived(runtime.install_history.slice(-HISTORY_LIMIT).reverse());

  let canPauseRuntime = $derived(
    Boolean(
      runtime.active_job?.state === 'downloading' &&
        runtime.active_job.cancellable
    )
  );

  let canDiscardRetainedDownload = $derived(
    Boolean(runtime.active_job?.state === 'paused' && runtime.job_artifact?.retained)
  );

  let installActionLabel = $derived.by(() => {
    if (runtime.active_job?.state === 'paused' && runtime.job_artifact?.retained) {
      return 'Resume download';
    }

    if (installedVersions.length > 0) {
      return 'Install latest available version';
    }

    return `Install ${runtime.display_name}`;
  });
</script>

<section class="rounded-lg border border-neutral-700 bg-neutral-900/60 p-3">
  <div class="flex items-start justify-between gap-3">
    <div>
      <h4 class="text-sm font-medium text-neutral-100">{runtime.display_name}</h4>
      <p class="mt-1 text-xs text-neutral-500">
        Readiness {runtime.readiness_state}
        {#if runtime.selection.active_version}
          • Active {runtime.selection.active_version}
        {/if}
      </p>
    </div>
    <div class="text-right text-[11px] text-neutral-500">
      <div>{runtime.available ? 'Ready for use' : 'Not ready'}</div>
      <div>{installedVersions.length} installed version{installedVersions.length === 1 ? '' : 's'}</div>
    </div>
  </div>

  {#if runtime.unavailable_reason}
    <div class="mt-3 rounded border border-amber-800/50 bg-amber-950/20 p-2 text-xs text-amber-300">
      {runtime.unavailable_reason}
    </div>
  {/if}

  {#if error}
    <div class="mt-3 rounded border border-red-800/50 bg-red-950/20 p-2 text-xs text-red-300">
      {error}
    </div>
  {/if}

  <div class="mt-3 grid gap-2 text-xs text-neutral-400 md:grid-cols-4">
    <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
      <div class="text-neutral-500">Selected</div>
      <div class="mt-1 text-neutral-200">{runtime.selection.selected_version ?? 'Automatic'}</div>
    </div>
    <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
      <div class="text-neutral-500">Default</div>
      <div class="mt-1 text-neutral-200">{runtime.selection.default_version ?? 'Unset'}</div>
    </div>
    <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
      <div class="text-neutral-500">Install State</div>
      <div class="mt-1 text-neutral-200">{runtime.install_state}</div>
    </div>
    <div class="rounded border border-neutral-800 bg-neutral-950/70 p-2">
      <div class="text-neutral-500">Download Size</div>
      <div class="mt-1 text-neutral-200">{DOWNLOAD_SIZE_LABELS[runtime.id] ?? 'Unknown'}</div>
    </div>
  </div>

  {#if runtime.active_job}
    <div class="mt-3 rounded border border-blue-900/50 bg-blue-950/20 p-3">
      <div class="flex items-center justify-between gap-2">
        <div class="text-sm text-blue-200">{runtime.active_job.status}</div>
        <div class="text-[11px] text-blue-300">{runtime.active_job.state}</div>
      </div>

      {#if runtime.active_job.total > 0}
        <div class="mt-2 rounded-full bg-neutral-800 h-2 overflow-hidden">
          <div
            class="h-2 bg-blue-500 transition-all duration-300"
            style={`width: ${progressPercent}%`}
          ></div>
        </div>
        <div class="mt-1 text-[11px] text-neutral-400">
          {formatBytes(runtime.active_job.current)} / {formatBytes(runtime.active_job.total)}
        </div>
      {/if}

      {#if runtime.job_artifact}
        <div class="mt-2 text-[11px] text-neutral-400">
          Retained artifact {runtime.job_artifact.archive_name} ({runtime.job_artifact.version})
          <div class="text-neutral-500">
            {formatBytes(runtime.job_artifact.downloaded_bytes)} / {formatBytes(runtime.job_artifact.total_bytes)}
          </div>
        </div>
      {/if}

      <div class="mt-3 flex flex-wrap gap-2">
        {#if canPauseRuntime}
          <button
            type="button"
            class="rounded border border-amber-700 px-3 py-1.5 text-xs text-amber-200 transition-colors hover:bg-amber-950/40 disabled:border-neutral-800 disabled:text-neutral-600"
            onclick={pauseRuntime}
            disabled={pauseRequested || cancelRequested}
          >
            {pauseRequested ? 'Requesting pause...' : 'Pause download'}
          </button>
        {/if}

        {#if runtime.active_job.cancellable || canDiscardRetainedDownload}
          <button
            type="button"
            class="rounded border border-red-800 px-3 py-1.5 text-xs text-red-200 transition-colors hover:bg-red-950/40 disabled:border-neutral-800 disabled:text-neutral-600"
            onclick={cancelRuntime}
            disabled={cancelRequested || pauseRequested}
          >
            {#if canDiscardRetainedDownload}
              {cancelRequested ? 'Discarding...' : 'Discard retained download'}
            {:else}
              {cancelRequested ? 'Requesting cancel...' : 'Cancel download'}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <div class="mt-3 grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
    <div class="space-y-3">
      <div>
        <h5 class="text-xs uppercase tracking-wider text-neutral-500">Version Policy</h5>
        {#if selectableVersions.length > 0}
          <div class="mt-2 space-y-2">
            <label class="block text-xs text-neutral-400" for={`${runtime.id}-selected-version`}>
              Selected version
            </label>
            <select
              id={`${runtime.id}-selected-version`}
              class="w-full rounded border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-sm text-neutral-200"
              value={runtime.selection.selected_version ?? ''}
              disabled={selectionUpdating || installRequested}
              onchange={(event) =>
                updateSelectedVersion(
                  (event.currentTarget as HTMLSelectElement).value || null
                )}
            >
              <option value="">Automatic</option>
              {#each selectableVersions as version (version.display_label)}
                <option value={version.version ?? ''}>{version.display_label}</option>
              {/each}
            </select>

            <label class="block text-xs text-neutral-400" for={`${runtime.id}-default-version`}>
              Default version
            </label>
            <select
              id={`${runtime.id}-default-version`}
              class="w-full rounded border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-sm text-neutral-200"
              value={runtime.selection.default_version ?? ''}
              disabled={selectionUpdating || installRequested}
              onchange={(event) =>
                updateDefaultVersion(
                  (event.currentTarget as HTMLSelectElement).value || null
                )}
            >
              <option value="">Unset</option>
              {#each selectableVersions as version (version.display_label)}
                <option value={version.version ?? ''}>{version.display_label}</option>
              {/each}
            </select>
          </div>
        {:else}
          <p class="mt-2 text-xs text-neutral-500">
            Install a runtime version before Pantograph can pin selection or default policy.
          </p>
        {/if}
      </div>

      <div>
        <h5 class="text-xs uppercase tracking-wider text-neutral-500">Available Versions</h5>
        <ul class="mt-2 space-y-2">
          {#each runtime.versions as version (version.display_label)}
            <li class="rounded border border-neutral-800 bg-neutral-950/60 p-2 text-xs text-neutral-300">
              <div class="flex flex-wrap items-center gap-2">
                <span class="font-medium">{version.display_label}</span>
                <span class="rounded bg-neutral-800 px-1.5 py-0.5 text-[10px] text-neutral-400">
                  {versionBadgeLabel(version)}
                </span>
                {#if version.selected}
                  <span class="rounded bg-blue-900/40 px-1.5 py-0.5 text-[10px] text-blue-300">Selected</span>
                {/if}
                {#if version.active}
                  <span class="rounded bg-green-900/40 px-1.5 py-0.5 text-[10px] text-green-300">Active</span>
                {/if}
              </div>
              <div class="mt-1 text-[11px] text-neutral-500">
                {version.runtime_key} • {version.platform_key}
              </div>
              {#if version.install_root}
                <div class="mt-1 break-all font-mono text-[11px] text-neutral-600">
                  {version.install_root}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    </div>

    <div class="space-y-3">
      <div>
        <h5 class="text-xs uppercase tracking-wider text-neutral-500">Install History</h5>
        {#if recentHistory.length > 0}
          <ul class="mt-2 space-y-2">
            {#each recentHistory as entry (`${entry.event}-${entry.at_ms}`)}
              <li class="rounded border border-neutral-800 bg-neutral-950/60 p-2 text-xs text-neutral-300">
                <div class="flex items-center justify-between gap-2">
                  <span>{formatHistoryEvent(entry.event)}</span>
                  <span class="text-[11px] text-neutral-500">{formatHistoryTime(entry.at_ms)}</span>
                </div>
                {#if entry.version}
                  <div class="mt-1 text-[11px] text-neutral-500">Version {entry.version}</div>
                {/if}
                {#if entry.detail}
                  <div class="mt-1 text-[11px] text-neutral-600">{entry.detail}</div>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <p class="mt-2 text-xs text-neutral-500">No install history recorded yet.</p>
        {/if}
      </div>

      {#if runtime.missing_files.length > 0}
        <details class="rounded border border-neutral-800 bg-neutral-950/60 p-2 text-xs text-neutral-400">
          <summary class="cursor-pointer hover:text-neutral-300">
            {runtime.missing_files.length} missing file{runtime.missing_files.length === 1 ? '' : 's'}
          </summary>
          <ul class="mt-2 list-disc pl-4">
            {#each runtime.missing_files as file (file)}
              <li class="break-all font-mono text-[11px] text-neutral-500">{file}</li>
            {/each}
          </ul>
        </details>
      {/if}

      <div class="flex flex-wrap gap-2">
        {#if runtime.can_install && !runtime.active_job}
          <button
            type="button"
            class="rounded bg-blue-600 px-3 py-1.5 text-xs text-white transition-colors hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500"
            onclick={installRuntime}
            disabled={installRequested || removeRequested}
          >
            {installRequested ? 'Starting...' : installActionLabel}
          </button>
        {/if}

        {#if runtime.can_remove && !runtime.active_job}
          <button
            type="button"
            class="rounded border border-neutral-700 px-3 py-1.5 text-xs text-neutral-300 transition-colors hover:bg-neutral-800 disabled:border-neutral-800 disabled:text-neutral-600"
            onclick={removeRuntime}
            disabled={removeRequested || installRequested}
          >
            {removeRequested ? 'Removing...' : 'Remove runtime'}
          </button>
        {/if}
      </div>
    </div>
  </div>
</section>
