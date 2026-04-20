<script lang="ts">
  import type { ManagedRuntimeManagerRuntimeView } from '../../services/managedRuntime';

  type Props = {
    runtime: ManagedRuntimeManagerRuntimeView;
    progressPercent: number;
    progressCaption: string | null;
    canPauseRuntime: boolean;
    canDiscardRetainedDownload: boolean;
    pauseRequested: boolean;
    cancelRequested: boolean;
    formatBytes: (bytes: number) => string;
    onPauseRuntime: () => Promise<void>;
    onCancelRuntime: () => Promise<void>;
  };

  let {
    runtime,
    progressPercent,
    progressCaption,
    canPauseRuntime,
    canDiscardRetainedDownload,
    pauseRequested,
    cancelRequested,
    formatBytes,
    onPauseRuntime,
    onCancelRuntime,
  }: Props = $props();
</script>

{#if runtime.active_job}
  <div
    class="mt-3 rounded border border-blue-900/50 bg-blue-950/20 p-3"
    role="status"
    aria-live="polite"
  >
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="min-w-0 flex-1">
        <div class="text-[11px] uppercase tracking-wider text-blue-300">Active transfer</div>
        <div class="mt-1 text-sm font-medium text-blue-100">{runtime.active_job.status}</div>
        {#if progressCaption}
          <div class="mt-1 text-[11px] text-blue-200/80">{progressCaption}</div>
        {/if}
      </div>
      <div class="text-right">
        <div class="text-[11px] uppercase tracking-wider text-blue-300">State</div>
        <div class="mt-1 text-sm capitalize text-blue-100">
          {runtime.active_job.state.replaceAll('_', ' ')}
        </div>
      </div>
    </div>

    {#if runtime.active_job.total > 0}
      <div class="mt-3 flex items-end justify-between gap-3">
        <div class="text-2xl font-semibold text-white">
          {Math.round(progressPercent)}%
        </div>
        <div class="text-right text-[11px] text-neutral-300">
          <div>{formatBytes(runtime.active_job.current)} downloaded</div>
          <div>{formatBytes(runtime.active_job.total)} total</div>
        </div>
      </div>
      <div class="mt-2 h-2.5 overflow-hidden rounded-full bg-neutral-800">
        <div
          class="h-2.5 bg-blue-500 transition-all duration-300"
          style={`width: ${progressPercent}%`}
        ></div>
      </div>
      <div class="mt-2 text-[11px] text-neutral-300">
        {formatBytes(runtime.active_job.current)} / {formatBytes(runtime.active_job.total)}
      </div>
    {/if}

    {#if runtime.job_artifact}
      <div class="mt-3 rounded border border-neutral-800 bg-neutral-950/60 p-2 text-[11px] text-neutral-300">
        <div class="font-medium text-neutral-200">
          Retained artifact {runtime.job_artifact.archive_name} ({runtime.job_artifact.version})
        </div>
        <div class="mt-1 text-neutral-500">
          {formatBytes(runtime.job_artifact.downloaded_bytes)} / {formatBytes(runtime.job_artifact.total_bytes)}
        </div>
      </div>
    {/if}

    <div class="mt-3 flex flex-wrap gap-2">
      {#if canPauseRuntime}
        <button
          type="button"
          class="rounded border border-amber-700 px-3 py-1.5 text-xs text-amber-200 transition-colors hover:bg-amber-950/40 disabled:border-neutral-800 disabled:text-neutral-600"
          onclick={onPauseRuntime}
          disabled={pauseRequested || cancelRequested}
        >
          {pauseRequested ? 'Requesting pause...' : 'Pause download'}
        </button>
      {/if}

      {#if runtime.active_job.cancellable || canDiscardRetainedDownload}
        <button
          type="button"
          class="rounded border border-red-800 px-3 py-1.5 text-xs text-red-200 transition-colors hover:bg-red-950/40 disabled:border-neutral-800 disabled:text-neutral-600"
          onclick={onCancelRuntime}
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
