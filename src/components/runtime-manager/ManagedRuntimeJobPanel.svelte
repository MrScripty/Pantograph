<script lang="ts">
  import type { ManagedRuntimeManagerRuntimeView } from '../../services/managedRuntime';
  import { StatusGlyph } from '../workbench/ui';
  import { managedRuntimeJobGlyph, sentenceCaseState } from './managedRuntimePresenters';

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
    class="mt-3 rounded border border-pg-border bg-pg-panel-head px-3 py-3"
    role="status"
    aria-live="polite"
  >
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="min-w-0 flex-1">
        <h5 class="m-0 text-[12.5px] font-bold text-pg-text-soft">Active transfer</h5>
        <div class="mt-1 text-[13px] text-pg-text-bright">{runtime.active_job.status}</div>
        {#if progressCaption}
          <div class="mt-1 pg-muted pg-small">{progressCaption}</div>
        {/if}
      </div>
      <div class="text-right pg-small">
        <div class="pg-muted">State</div>
        <div class="mt-1 flex items-center justify-end gap-1.5">
          <StatusGlyph state={managedRuntimeJobGlyph(runtime.active_job.state)} small />
          {sentenceCaseState(runtime.active_job.state)}
        </div>
      </div>
    </div>

    {#if runtime.active_job.total > 0}
      <div class="mt-3 flex items-end justify-between gap-3">
        <div class="pg-mono text-[20px] text-pg-text-bright">
          {Math.round(progressPercent)}%
        </div>
        <div class="text-right pg-muted pg-small">
          <div>{formatBytes(runtime.active_job.current)} downloaded</div>
          <div>{formatBytes(runtime.active_job.total)} total</div>
        </div>
      </div>
      <div class="pg-meter mt-2" aria-hidden="true">
        <i class="pg-meter-accent" style={`width: ${progressPercent}%`}></i>
      </div>
      <div class="mt-2 pg-mono pg-muted">
        {formatBytes(runtime.active_job.current)} / {formatBytes(runtime.active_job.total)}
      </div>
    {/if}

    {#if runtime.job_artifact}
      <div class="mt-3 rounded border border-pg-border bg-pg-panel-sunken px-3 py-2 pg-small">
        <div class="text-pg-text-bright">
          Retained artifact <span class="pg-mono">{runtime.job_artifact.archive_name}</span>
          ({runtime.job_artifact.version})
        </div>
        <div class="mt-1 pg-mono pg-muted">
          {formatBytes(runtime.job_artifact.downloaded_bytes)} / {formatBytes(runtime.job_artifact.total_bytes)}
        </div>
      </div>
    {/if}

    <div class="mt-3 flex flex-wrap gap-2">
      {#if canPauseRuntime}
        <button
          type="button"
          class="pg-btn"
          onclick={onPauseRuntime}
          disabled={pauseRequested || cancelRequested}
        >
          {pauseRequested ? 'Requesting pause...' : 'Pause download'}
        </button>
      {/if}

      {#if runtime.active_job.cancellable || canDiscardRetainedDownload}
        <button
          type="button"
          class="pg-btn pg-btn-danger"
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
