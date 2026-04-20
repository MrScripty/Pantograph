<script lang="ts">
  import {
    managedRuntimeService,
    type ManagedRuntimeId,
    type ManagedRuntimeManagerRuntimeView,
    type ManagedRuntimeVersionStatus,
  } from '../../services/managedRuntime';
  import ManagedRuntimeActivityPanel from './ManagedRuntimeActivityPanel.svelte';
  import ManagedRuntimeCatalogPanel from './ManagedRuntimeCatalogPanel.svelte';
  import ManagedRuntimeHeader from './ManagedRuntimeHeader.svelte';
  import ManagedRuntimeJobPanel from './ManagedRuntimeJobPanel.svelte';
  import ManagedRuntimeSummaryGrid from './ManagedRuntimeSummaryGrid.svelte';

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
  <ManagedRuntimeHeader
    {runtime}
    installedVersionCount={installedVersions.length}
    {error}
  />

  <ManagedRuntimeSummaryGrid
    available={runtime.available}
    installState={runtime.install_state}
    selection={runtime.selection}
    installedVersionCount={installedVersions.length}
    downloadSizeLabel={DOWNLOAD_SIZE_LABELS[runtime.id] ?? 'Unknown'}
  />

  <ManagedRuntimeJobPanel
    {runtime}
    {progressPercent}
    {canPauseRuntime}
    {canDiscardRetainedDownload}
    {pauseRequested}
    {cancelRequested}
    {formatBytes}
    onPauseRuntime={pauseRuntime}
    onCancelRuntime={cancelRuntime}
  />

  <div class="mt-3 grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
    <ManagedRuntimeCatalogPanel
      {runtime}
      {selectableVersions}
      {selectionUpdating}
      onUpdateSelected={updateSelectedVersion}
      onUpdateDefault={updateDefaultVersion}
      {versionBadgeLabel}
    />

    <ManagedRuntimeActivityPanel
      {runtime}
      {recentHistory}
      {installActionLabel}
      {installRequested}
      {removeRequested}
      onInstallRuntime={installRuntime}
      onRemoveRuntime={removeRuntime}
      {formatHistoryEvent}
      {formatHistoryTime}
    />
  </div>
</section>
