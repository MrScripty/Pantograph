<script lang="ts">
  import type {
    DiagnosticsRuntimeLifecycleSnapshot,
    DiagnosticsRuntimeSnapshot,
  } from '../../services/diagnostics/types';
  import {
    formatDiagnosticsDuration,
    formatDiagnosticsBytes,
    formatDiagnosticsTimestamp,
    getRuntimeInstallStateClasses,
  } from './presenters';

  export let runtime: DiagnosticsRuntimeSnapshot;

  function lifecycleStateLabel(snapshot: DiagnosticsRuntimeLifecycleSnapshot | null): string {
    if (!snapshot) {
      return 'Unavailable';
    }
    if (snapshot.lastError) {
      return 'Error';
    }
    return snapshot.active ? 'Active' : 'Idle';
  }
</script>

<div class="h-full overflow-auto px-4 py-4">
  {#if !runtime.workflowId}
    <div class="flex h-full items-center justify-center rounded-2xl border border-dashed border-neutral-800 bg-neutral-950/70 px-6 text-center text-sm text-neutral-500">
      Select a workflow to inspect host capabilities, required runtimes, and runtime installation state.
    </div>
  {:else}
    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Workflow</div>
        <div class="mt-3 text-sm font-medium text-neutral-100">{runtime.workflowId}</div>
        <div class="mt-2 text-xs text-neutral-500">
          Refreshed {formatDiagnosticsTimestamp(runtime.capturedAtMs)}
        </div>
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Limits</div>
        <div class="mt-3 space-y-2 text-sm text-neutral-300">
          <div class="flex items-center justify-between">
            <span>Input Bindings</span>
            <span>{runtime.maxInputBindings ?? 'n/a'}</span>
          </div>
          <div class="flex items-center justify-between">
            <span>Output Targets</span>
            <span>{runtime.maxOutputTargets ?? 'n/a'}</span>
          </div>
          <div class="flex items-center justify-between">
            <span>Max Value Bytes</span>
            <span>{runtime.maxValueBytes === null ? 'n/a' : formatDiagnosticsBytes(runtime.maxValueBytes)}</span>
          </div>
        </div>
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Memory Envelope</div>
        <div class="mt-3 space-y-2 text-sm text-neutral-300">
          <div class="flex items-center justify-between gap-3">
            <span>Peak VRAM</span>
            <span>{runtime.runtimeRequirements?.estimated_peak_vram_mb ?? 'n/a'} MB</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Peak RAM</span>
            <span>{runtime.runtimeRequirements?.estimated_peak_ram_mb ?? 'n/a'} MB</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span>Confidence</span>
            <span>{runtime.runtimeRequirements?.estimation_confidence ?? 'unknown'}</span>
          </div>
        </div>
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="text-[11px] uppercase tracking-[0.28em] text-neutral-500">Requirements</div>
        <div class="mt-3 text-sm text-neutral-300">
          <div>{runtime.runtimeRequirements?.required_backends.length ?? 0} backends</div>
          <div>{runtime.runtimeRequirements?.required_models.length ?? 0} models</div>
          <div>{runtime.runtimeRequirements?.required_extensions.length ?? 0} extensions</div>
        </div>
      </article>
    </div>

    {#if runtime.lastError}
      <div class="mt-4 rounded-xl border border-red-900/80 bg-red-950/40 px-4 py-3 text-sm text-red-200">
        {runtime.lastError}
      </div>
    {/if}

    <div class="mt-4 grid gap-4 xl:grid-cols-2">
      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="flex items-start justify-between gap-3">
          <div>
            <div class="text-sm font-medium text-neutral-100">Active Runtime Lifecycle</div>
            <div class="text-xs text-neutral-500">
              Current backend-owned lifecycle snapshot for the active runtime.
            </div>
          </div>
          <span class="rounded-full border border-neutral-700 px-2 py-1 text-[11px] font-medium text-neutral-300">
            {lifecycleStateLabel(runtime.activeRuntime)}
          </span>
        </div>

        {#if runtime.activeRuntime}
          <div class="mt-3 grid gap-2 text-sm text-neutral-300 md:grid-cols-2">
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Runtime: {runtime.activeRuntime.runtimeId ?? 'unknown'}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Instance: {runtime.activeRuntime.runtimeInstanceId ?? 'unreported'}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2 md:col-span-2">
              Target: {runtime.activeModelTarget ?? 'unreported'}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Warmup: {formatDiagnosticsDuration(runtime.activeRuntime.warmupDurationMs ?? null)}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Reused: {runtime.activeRuntime.runtimeReused === null ? 'unknown' : runtime.activeRuntime.runtimeReused ? 'yes' : 'no'}
            </div>
          </div>
          <div class="mt-3 text-xs text-neutral-500">
            Started {formatDiagnosticsTimestamp(runtime.activeRuntime.warmupStartedAtMs ?? null)}
            • Completed {formatDiagnosticsTimestamp(runtime.activeRuntime.warmupCompletedAtMs ?? null)}
          </div>
          <div class="mt-2 text-xs text-neutral-400">
            Reason: {runtime.activeRuntime.lifecycleDecisionReason ?? 'unreported'}
          </div>
          {#if runtime.activeRuntime.lastError}
            <div class="mt-3 rounded-lg border border-red-900/70 bg-red-950/40 px-3 py-2 text-xs text-red-200">
              {runtime.activeRuntime.lastError}
            </div>
          {/if}
        {:else}
          <div class="mt-3 text-sm text-neutral-500">
            No active runtime lifecycle snapshot has been reported yet.
          </div>
        {/if}
      </article>

      <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
        <div class="flex items-start justify-between gap-3">
          <div>
            <div class="text-sm font-medium text-neutral-100">Embedding Runtime Lifecycle</div>
            <div class="text-xs text-neutral-500">
              Dedicated embedding runtime snapshot when parallel embedding is active.
            </div>
          </div>
          <span class="rounded-full border border-neutral-700 px-2 py-1 text-[11px] font-medium text-neutral-300">
            {lifecycleStateLabel(runtime.embeddingRuntime)}
          </span>
        </div>

        {#if runtime.embeddingRuntime}
          <div class="mt-3 grid gap-2 text-sm text-neutral-300 md:grid-cols-2">
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Runtime: {runtime.embeddingRuntime.runtimeId ?? 'unknown'}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Instance: {runtime.embeddingRuntime.runtimeInstanceId ?? 'unreported'}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2 md:col-span-2">
              Target: {runtime.embeddingModelTarget ?? 'unreported'}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Warmup: {formatDiagnosticsDuration(runtime.embeddingRuntime.warmupDurationMs ?? null)}
            </div>
            <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
              Reused: {runtime.embeddingRuntime.runtimeReused === null ? 'unknown' : runtime.embeddingRuntime.runtimeReused ? 'yes' : 'no'}
            </div>
          </div>
          <div class="mt-3 text-xs text-neutral-500">
            Started {formatDiagnosticsTimestamp(runtime.embeddingRuntime.warmupStartedAtMs ?? null)}
            • Completed {formatDiagnosticsTimestamp(runtime.embeddingRuntime.warmupCompletedAtMs ?? null)}
          </div>
          <div class="mt-2 text-xs text-neutral-400">
            Reason: {runtime.embeddingRuntime.lifecycleDecisionReason ?? 'unreported'}
          </div>
          {#if runtime.embeddingRuntime.lastError}
            <div class="mt-3 rounded-lg border border-red-900/70 bg-red-950/40 px-3 py-2 text-xs text-red-200">
              {runtime.embeddingRuntime.lastError}
            </div>
          {/if}
        {:else}
          <div class="mt-3 text-sm text-neutral-500">
            No dedicated embedding runtime snapshot has been reported for this workflow context.
          </div>
        {/if}
      </article>
    </div>

    <div class="mt-4 grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(20rem,1fr)]">
      <section class="rounded-xl border border-neutral-800 bg-neutral-950/80">
        <header class="border-b border-neutral-800 px-4 py-3">
          <div class="text-sm font-medium text-neutral-100">Runtime Capabilities</div>
          <div class="text-xs text-neutral-500">
            Host-reported runtimes available for the current workflow.
          </div>
        </header>

        {#if runtime.runtimeCapabilities.length === 0}
          <div class="px-4 py-6 text-sm text-neutral-500">
            No runtime capabilities were reported for this workflow.
          </div>
        {:else}
          <div class="grid gap-3 p-4 md:grid-cols-2">
            {#each runtime.runtimeCapabilities as capability (capability.runtime_id)}
              <article class="rounded-xl border border-neutral-800 bg-neutral-950 px-4 py-4">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="text-sm font-medium text-neutral-100">{capability.display_name}</div>
                    <div class="text-xs text-neutral-500">{capability.runtime_id}</div>
                  </div>
                  <span class={`inline-flex rounded-full border px-2 py-1 text-[11px] font-medium ${getRuntimeInstallStateClasses(capability.install_state, capability.available)}`}>
                    {capability.install_state}
                  </span>
                </div>

                <div class="mt-3 grid grid-cols-2 gap-2 text-xs text-neutral-400">
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
                    Available: {capability.available ? 'yes' : 'no'}
                  </div>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
                    Configured: {capability.configured ? 'yes' : 'no'}
                  </div>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
                    Install: {capability.can_install ? 'allowed' : 'blocked'}
                  </div>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
                    Remove: {capability.can_remove ? 'allowed' : 'blocked'}
                  </div>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
                    Source: {capability.source_kind}
                  </div>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
                    Selected: {capability.selected ? 'yes' : 'no'}
                  </div>
                  <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">
                    External Attach: {capability.supports_external_connection ? 'supported' : 'not supported'}
                  </div>
                </div>

                {#if capability.backend_keys.length > 0}
                  <div class="mt-3 text-xs text-neutral-500">
                    Backends: {capability.backend_keys.join(', ')}
                  </div>
                {/if}
                {#if capability.missing_files.length > 0}
                  <div class="mt-2 text-xs text-amber-300">
                    Missing files: {capability.missing_files.join(', ')}
                  </div>
                {/if}
                {#if capability.unavailable_reason}
                  <div class="mt-2 text-xs text-red-300">
                    {capability.unavailable_reason}
                  </div>
                {/if}
              </article>
            {/each}
          </div>
        {/if}
      </section>

      <section class="space-y-4">
        <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
          <div class="text-sm font-medium text-neutral-100">Required Backends</div>
          {#if runtime.runtimeRequirements?.required_backends.length}
            <div class="mt-3 flex flex-wrap gap-2">
              {#each runtime.runtimeRequirements.required_backends as backendKey (backendKey)}
                <span class="rounded-full border border-neutral-700 px-2 py-1 text-xs text-neutral-300">{backendKey}</span>
              {/each}
            </div>
          {:else}
            <div class="mt-3 text-sm text-neutral-500">No specific backend requirements reported.</div>
          {/if}
        </article>

        <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
          <div class="text-sm font-medium text-neutral-100">Required Models</div>
          {#if runtime.runtimeRequirements?.required_models.length}
            <div class="mt-3 space-y-2 text-sm text-neutral-300">
              {#each runtime.runtimeRequirements.required_models as modelId (modelId)}
                <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-2">{modelId}</div>
              {/each}
            </div>
          {:else}
            <div class="mt-3 text-sm text-neutral-500">No model requirements reported.</div>
          {/if}
        </article>

        <article class="rounded-xl border border-neutral-800 bg-neutral-950/80 p-4">
          <div class="text-sm font-medium text-neutral-100">Capability Models</div>
          {#if runtime.models.length}
            <div class="mt-3 space-y-2 text-xs text-neutral-300">
              {#each runtime.models as model (model.model_id)}
                <div class="rounded-lg border border-neutral-800 bg-neutral-900/70 px-3 py-3">
                  <div class="font-medium text-neutral-100">{model.model_id}</div>
                  <div class="mt-1 text-neutral-500">
                    Revision {model.model_revision_or_hash ?? 'unreported'} • Type {model.model_type ?? 'unknown'}
                  </div>
                  <div class="mt-2 text-neutral-400">
                    Nodes: {model.node_ids.join(', ') || 'none'}
                  </div>
                  <div class="text-neutral-400">
                    Roles: {model.roles.join(', ') || 'none'}
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class="mt-3 text-sm text-neutral-500">The workflow capabilities response did not report model inventory.</div>
          {/if}
        </article>
      </section>
    </div>
  {/if}
</div>
