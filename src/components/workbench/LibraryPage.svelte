<script lang="ts">
  import { onMount } from 'svelte';
  import { getWorkflowBrowserHost } from '../../services/workflow/workflowBrowserTransport';
  import {
    browserLibraryActionAllowed,
    loadWorkflowBrowserCapabilities,
    type WorkflowBrowserCapabilities,
  } from '../../services/workflow/workflowBrowserCapabilities';
  import type {
    LibraryUsageProjectionRecord,
    ProjectionStateRecord,
    PumasHfModelSearchResult,
  } from '../../services/diagnostics/types';
  import { subscribeDiagnosticsProjectionInvalidations } from '../../services/workflow/WorkflowProjectionSubscriptionService';
  import type { PortOption } from '../../services/workflow/types';
  import {
    invalidatePumasModelOptionsCache,
    loadPumasModelOptions,
  } from '../../services/workflow/pumaModelOptionsCache';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import {
    buildHfDownloadRequest,
    buildHfSearchRequest,
    buildLibraryModelRows,
    createLibraryRequestGate,
    filterLibraryModelRows,
    filterLibraryModelRowsByKind,
    formatHfDownloadStatus,
    formatHfResultCountLine,
    formatHfSearchStatus,
    formatLibraryDimensionLine,
    formatLibraryModelSubline,
    formatLibraryProjectionFreshness,
    formatLibraryTimeShort,
    formatPumasDeleteStatus,
    groupLibraryModelRows,
    HF_SEARCH_DEFAULT_LIMIT,
    HF_SEARCH_MAX_LIMIT,
    isLibraryAssetLastUsedByRun,
    libraryDeleteBlockReason,
    libraryKindOptions,
    libraryModelChips,
    libraryModelLocationRows,
    libraryModelSourceRows,
    libraryModelUsageRows,
    libraryWorkflowUseRows,
    projectHfSearchResults,
    type LibraryKindFilter,
    type LibraryModelGroupKey,
    type LibraryModelRow,
  } from './libraryUsagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';
  import { Chip, DimensionLine, GroupHeader, KeyValue, Panel, SegmentedControl, TwoStepButton } from './ui';

  type LibraryAction = 'search' | 'download' | 'delete';

  // A plain browser on the loopback workflow host may only search or write when the host says so.
  const browserHost = getWorkflowBrowserHost();
  let hostCapabilities = $state<WorkflowBrowserCapabilities | null>(null);
  const canSearch = $derived(browserLibraryActionAllowed(Boolean(browserHost), hostCapabilities, 'search'));
  const canWrite = $derived(browserLibraryActionAllowed(Boolean(browserHost), hostCapabilities, 'write'));
  const hostSearchBlockReason = $derived(
    canSearch ? null : hostCapabilities ? 'This browser host does not enable Hugging Face search' : 'Checking browser host capabilities',
  );
  const hostWriteBlockReason = $derived(
    canWrite ? null : hostCapabilities ? 'This browser host does not enable library downloads or deletion' : 'Checking browser host capabilities',
  );

  // Projection state. Only the services below write it; the page never patches it locally.
  let assets = $state<LibraryUsageProjectionRecord[]>([]);
  let pumasModels = $state<PortOption[]>([]);
  let projectionState = $state<ProjectionStateRecord | null>(null);
  let usageLoading = $state(false);
  let pumasLoading = $state(false);
  let usageError = $state<string | null>(null);
  let pumasError = $state<string | null>(null);
  /** Invalidation subscription failure: the usage rows still load, they just stop updating live. */
  let liveUpdatesError = $state<string | null>(null);
  /** Clock for the 30-day grouping, taken when projection data is applied. */
  let nowMs = $state(Date.now());

  // Presentation state.
  let hfMode = $state(false);
  let localQuery = $state('');
  let kindFilter = $state<LibraryKindFilter>('all');
  let selectedModelId = $state<string | null>(null);
  let collapsedGroups = $state<Partial<Record<LibraryModelGroupKey, boolean>>>({});

  // Action state (search, download, delete are serialised by `actionBusy`, as before).
  let actionBusy = $state<LibraryAction | null>(null);
  let actionMessage = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let hfQuery = $state('');
  let hfKind = $state('');
  let hfLimit = $state<number | null>(HF_SEARCH_DEFAULT_LIMIT);
  let hfSearched = $state(false);
  let hfResults = $state<PumasHfModelSearchResult[]>([]);
  let hfSearchedLimit = $state<number>(HF_SEARCH_DEFAULT_LIMIT);
  let downloadOptionsOpen = $state(false);
  let downloadOfficialName = $state('');
  let downloadFamily = $state('model');
  let downloadModelType = $state('');
  let downloadingRepoId = $state<string | null>(null);
  let requestedDownloads = $state<Record<string, string>>({});

  // One gate per kind of request, owned by this page instance and disposed on destroy.
  const usageGate = createLibraryRequestGate();
  const modelsGate = createLibraryRequestGate();
  const actionGate = createLibraryRequestGate();

  let activeRunId = $derived($activeWorkflowRun?.workflow_run_id ?? null);
  let modelRows = $derived(buildLibraryModelRows(pumasModels, assets));
  let queriedRows = $derived(filterLibraryModelRows(modelRows, localQuery));
  let kindOptions = $derived(libraryKindOptions(queriedRows));
  let visibleRows = $derived(kindOptions ? filterLibraryModelRowsByKind(queriedRows, kindFilter) : queriedRows);
  let groups = $derived(groupLibraryModelRows(visibleRows, nowMs));
  let selectedRow = $derived(visibleRows.find((row) => row.modelId === selectedModelId) ?? visibleRows[0] ?? null);
  let dimensionLine = $derived(formatLibraryDimensionLine(modelRows, nowMs, pumasError !== null));
  let hfRows = $derived(projectHfSearchResults(hfResults));
  let freshness = $derived(formatLibraryProjectionFreshness(projectionState));
  let deleteBlockReason = $derived(
    selectedRow ? (hostWriteBlockReason ?? libraryDeleteBlockReason(selectedRow, activeRunId, actionBusy !== null)) : null,
  );
  let searchBlockReason = $derived(
    hostSearchBlockReason ?? (actionBusy !== null ? 'Wait for the current library action to finish' : null),
  );
  let downloadBlockReason = $derived(
    hostWriteBlockReason ?? (actionBusy !== null ? 'Wait for the current library action to finish' : null),
  );

  async function refreshLibraryUsage(): Promise<void> {
    const token = usageGate.begin();
    usageLoading = assets.length === 0;
    try {
      const response = await workflowService.queryLibraryUsage({ limit: 250 });
      if (!usageGate.isCurrent(token)) return;
      assets = response.assets;
      projectionState = response.projection_state;
      usageError = null;
      nowMs = Date.now();
    } catch (error) {
      if (!usageGate.isCurrent(token)) return;
      usageError = formatWorkflowCommandError(error);
    } finally {
      if (usageGate.isCurrent(token)) usageLoading = false;
    }
  }

  async function refreshPumasModels(forceRefresh = false): Promise<void> {
    const token = modelsGate.begin();
    pumasLoading = pumasModels.length === 0;
    try {
      const options = await loadPumasModelOptions({ forceRefresh });
      if (!modelsGate.isCurrent(token)) return;
      pumasModels = options;
      pumasError = null;
      nowMs = Date.now();
    } catch (error) {
      if (!modelsGate.isCurrent(token)) return;
      pumasError = formatWorkflowCommandError(error);
    } finally {
      if (modelsGate.isCurrent(token)) pumasLoading = false;
    }
  }

  async function refreshLibraryPage(forcePumasReload = false): Promise<void> {
    if (forcePumasReload) invalidatePumasModelOptionsCache();
    await Promise.all([refreshLibraryUsage(), refreshPumasModels(forcePumasReload)]);
  }

  function beginAction(action: LibraryAction): number {
    actionBusy = action;
    actionError = null;
    actionMessage = null;
    return actionGate.begin();
  }

  async function searchHfModels(): Promise<void> {
    if (!canSearch || actionBusy !== null) return;
    const request = buildHfSearchRequest({ query: hfQuery, kind: hfKind, limit: hfLimit });
    const token = beginAction('search');
    try {
      const response = await workflowService.searchHfModelsWithAudit(request);
      if (!actionGate.isCurrent(token)) return;
      hfResults = response.models;
      hfSearchedLimit = request.limit ?? HF_SEARCH_DEFAULT_LIMIT;
      hfSearched = true;
      actionMessage = formatHfSearchStatus(response);
      await refreshLibraryUsage();
    } catch (error) {
      if (actionGate.isCurrent(token)) actionError = `Search failed. ${formatWorkflowCommandError(error)}`;
    } finally {
      if (actionGate.isCurrent(token)) actionBusy = null;
    }
  }

  async function startHfDownload(repoId: string): Promise<void> {
    if (!canWrite || actionBusy !== null) return;
    const request = buildHfDownloadRequest(repoId, {
      officialName: downloadOfficialName,
      family: downloadFamily,
      modelType: downloadModelType,
    });
    const token = beginAction('download');
    downloadingRepoId = repoId;
    try {
      const response = await workflowService.startHfDownloadWithAudit(request);
      if (!actionGate.isCurrent(token)) return;
      requestedDownloads = { ...requestedDownloads, [repoId]: response.downloadId };
      actionMessage = formatHfDownloadStatus(repoId, response);
      await refreshLibraryUsage();
    } catch (error) {
      if (actionGate.isCurrent(token)) actionError = `Download of ${repoId} failed. ${formatWorkflowCommandError(error)}`;
    } finally {
      if (actionGate.isCurrent(token)) {
        actionBusy = null;
        downloadingRepoId = null;
      }
    }
  }

  async function deleteModel(row: LibraryModelRow): Promise<void> {
    if (!canWrite || libraryDeleteBlockReason(row, activeRunId, actionBusy !== null)) return;
    const token = beginAction('delete');
    try {
      const response = await workflowService.deletePumasModelWithAudit(row.modelId);
      if (!actionGate.isCurrent(token)) return;
      if (response.success) {
        actionMessage = `${row.name}: ${formatPumasDeleteStatus(response)}`;
      } else {
        actionError = `${row.name} was not deleted. ${formatPumasDeleteStatus(response)}`;
      }
      // Re-read both projections: the model list is the authority for what remains on disk.
      await refreshLibraryPage(response.success);
    } catch (error) {
      if (actionGate.isCurrent(token)) actionError = `${row.name} was not deleted. ${formatWorkflowCommandError(error)}`;
    } finally {
      if (actionGate.isCurrent(token)) actionBusy = null;
    }
  }

  function toggleHfMode(): void {
    hfMode = !hfMode;
  }

  function toggleGroup(key: LibraryModelGroupKey): void {
    collapsedGroups = { ...collapsedGroups, [key]: !collapsedGroups[key] };
  }

  function lowerFirst(text: string): string {
    return text.charAt(0).toLowerCase() + text.slice(1);
  }

  onMount(() => {
    let disposed = false;
    let projectionUnsubscribe: (() => void) | null = null;
    void loadWorkflowBrowserCapabilities()
      .then((capabilities) => {
        if (!disposed) hostCapabilities = capabilities;
      })
      .catch((hostError: unknown) => {
        if (!disposed) actionError = formatWorkflowCommandError(hostError);
      });
    void refreshLibraryPage();
    void subscribeDiagnosticsProjectionInvalidations({
      projections: ['library_usage'],
      refresh: () => refreshLibraryUsage(),
      onRefreshError: (error) => {
        if (!disposed) usageError = formatWorkflowCommandError(error);
      },
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        projectionUnsubscribe = unlisten;
      })
      .catch((error: unknown) => {
        if (!disposed) liveUpdatesError = formatWorkflowCommandError(error);
      });

    return () => {
      disposed = true;
      usageGate.dispose();
      modelsGate.dispose();
      actionGate.dispose();
      projectionUnsubscribe?.();
      projectionUnsubscribe = null;
    };
  });
</script>

<div class="pg-main h-full">
  <DimensionLine label={dimensionLine} />

  <div class="lib-body">
    <Panel label={hfMode ? 'Hugging Face search' : 'Models'} width="400px">
      {#snippet top()}
        <div class="lib-search-row">
          {#if hfMode}
            <label for="lib-hf-query" class="pg-sr-only">Search Hugging Face</label>
            <input
              id="lib-hf-query"
              class="pg-field lib-grow"
              type="search"
              form="lib-hf-form"
              placeholder="Search Hugging Face"
              autocomplete="off"
              bind:value={hfQuery}
            />
          {:else}
            <label for="lib-search" class="pg-sr-only">Search models</label>
            <input
              id="lib-search"
              class="pg-field lib-grow"
              type="search"
              placeholder="Search models"
              autocomplete="off"
              bind:value={localQuery}
            />
          {/if}
          <button class="pg-btn lib-hf-toggle" type="button" aria-pressed={hfMode} onclick={toggleHfMode}>
            Find on Hugging Face
          </button>
        </div>
        {#if browserHost}
          <p class="pg-muted pg-small lib-host" data-testid="library-host-notice">
            {hostCapabilities?.description ?? 'Checking browser host capabilities…'}
            {#if hostCapabilities && (!canSearch || !canWrite)}
              Search, download and deletion require a host that enables these actions.
            {/if}
          </p>
        {/if}
        {#if hfMode}
          <form
            id="lib-hf-form"
            class="lib-search-row"
            onsubmit={(event) => {
              event.preventDefault();
              void searchHfModels();
            }}
          >
            <label class="pg-fl lib-grow">
              Kind
              <input class="pg-field" type="text" placeholder="Any, or for example text-generation" bind:value={hfKind} />
            </label>
            <label class="pg-fl lib-limit">
              Limit
              <input class="pg-field" type="number" min="1" max={HF_SEARCH_MAX_LIMIT} bind:value={hfLimit} />
            </label>
            <button
              class="pg-btn pg-btn-sm lib-search-btn"
              type="submit"
              aria-disabled={searchBlockReason !== null ? 'true' : undefined}
              title={searchBlockReason ?? undefined}
            >
              {actionBusy === 'search' ? 'Searching' : 'Search'}
            </button>
          </form>
          <div class="pg-muted pg-small">Searching Hugging Face. Results download into the Pumas library on this machine.</div>
        {:else if kindOptions}
          <SegmentedControl label="Kind" options={kindOptions} value={kindFilter} onchange={(next) => (kindFilter = next)} />
        {/if}
      {/snippet}

      {#if hfMode}
        <div class="lib-disclosure">
          <button
            class="lib-disclosure-btn"
            type="button"
            aria-expanded={downloadOptionsOpen}
            aria-controls="lib-download-options"
            onclick={() => (downloadOptionsOpen = !downloadOptionsOpen)}
          >
            <span aria-hidden="true">{downloadOptionsOpen ? '▾' : '▸'}</span>
            Download options
            <span class="pg-muted">family {downloadFamily.trim() || 'model'}, name {downloadOfficialName.trim() || 'from the repository id'}</span>
          </button>
          {#if downloadOptionsOpen}
            <div id="lib-download-options" class="lib-disclosure-body">
              <label class="pg-fl">
                Official name
                <input class="pg-field" type="text" placeholder="Repository id" bind:value={downloadOfficialName} />
              </label>
              <label class="pg-fl">
                Family
                <input class="pg-field" type="text" placeholder="model" bind:value={downloadFamily} />
              </label>
              <label class="pg-fl">
                Model type
                <input class="pg-field" type="text" placeholder="Optional, for example llm" bind:value={downloadModelType} />
              </label>
              <div class="pg-muted pg-small lib-span">Applied to the next Download. A blank name uses the repository id.</div>
            </div>
          {/if}
        </div>

        {#if !hfSearched}
          <p class="lib-note">Search by name or task, then download a result into the Pumas library.</p>
        {:else}
          {#each hfRows as result (result.repoId)}
            {@const downloadId = requestedDownloads[result.repoId]}
            <div class="lib-res">
              <span class="min-w-0">
                <span class="pg-mono lib-res-id">{result.repoId}</span>
                {#if result.sub}<span class="pg-muted pg-small lib-res-sub">{result.sub}</span>{/if}
              </span>
              <span class="pg-num pg-muted">{result.size ?? ''}</span>
              {#if downloadId}
                <button class="pg-btn pg-btn-sm" type="button" aria-disabled="true" title={`Download ${downloadId} started`}>
                  Requested
                </button>
              {:else}
                <button
                  class="pg-btn pg-btn-sm"
                  type="button"
                  aria-label={`${downloadingRepoId === result.repoId ? 'Requesting' : 'Download'} ${result.repoId}`}
                  aria-disabled={downloadBlockReason !== null ? 'true' : undefined}
                  title={downloadBlockReason ?? undefined}
                  onclick={() => void startHfDownload(result.repoId)}
                >
                  {downloadingRepoId === result.repoId ? 'Requesting' : 'Download'}
                </button>
              {/if}
            </div>
          {/each}
          <p class="lib-note">{formatHfResultCountLine(hfRows.length, hfSearchedLimit)}</p>
        {/if}
      {:else}
        {#if pumasError}
          <p class="lib-note">The Pumas model list could not be read. {pumasError}</p>
        {/if}
        {#if usageError}
          <p class="lib-note">
            {projectionState ? 'Usage could not be read, so rows show the last usage projection.' : 'Usage could not be read.'}
            {usageError}
          </p>
        {/if}
        {#if pumasLoading && modelRows.length === 0}
          <p class="lib-note">Loading the Pumas model list.</p>
        {:else if modelRows.length === 0 && !pumasError}
          <div class="lib-note">
            <p>No models in the Pumas library on this machine yet.</p>
            <button class="pg-btn pg-btn-sm lib-note-action" type="button" onclick={toggleHfMode}>Find on Hugging Face</button>
          </div>
        {:else if visibleRows.length === 0 && modelRows.length > 0}
          <p class="lib-note">No models match this search.</p>
        {/if}

        {#each groups as group (group.key)}
          {@const expanded = !collapsedGroups[group.key]}
          <GroupHeader
            label={group.label}
            count={group.rows.length}
            {expanded}
            controls={`lib-group-${group.key}`}
            ontoggle={() => toggleGroup(group.key)}
          />
          {#if expanded}
            <div id={`lib-group-${group.key}`}>
              {#each group.rows as row (row.modelId)}
                {@const lastUsedByActive = row.usage !== null && isLibraryAssetLastUsedByRun(row.usage, activeRunId)}
                <button
                  type="button"
                  class="pg-item lib-item"
                  aria-current={selectedRow?.modelId === row.modelId ? 'true' : undefined}
                  title={row.modelId}
                  onclick={() => (selectedModelId = row.modelId)}
                >
                  <span class="min-w-0">
                    <span class="pg-name">{row.name}</span>{#if row.modelType}<span class="pg-ver">{row.modelType}</span>{/if}
                    <span class="pg-sub">{formatLibraryModelSubline(row)}</span>
                    {#if lastUsedByActive}
                      <span class="pg-sub lib-detail">Last used by active run</span>
                    {:else if row.unavailableReason}
                      <span class="pg-sub lib-detail">Not ready</span>
                    {/if}
                  </span>
                  <span class="pg-time">{row.usage ? formatLibraryTimeShort(row.usage.last_accessed_at_ms, nowMs) : ''}</span>
                </button>
              {/each}
            </div>
          {/if}
        {/each}
      {/if}

      {#snippet footer()}
        <div class="lib-foot">
          <div class="lib-foot-row">
            <button
              class="pg-btn pg-btn-sm"
              type="button"
              aria-disabled={usageLoading || pumasLoading ? 'true' : undefined}
              title={usageLoading || pumasLoading ? 'Already reading the library' : 'Read the model list and usage again'}
              onclick={() => {
                if (!usageLoading && !pumasLoading) void refreshLibraryPage(true);
              }}
            >
              Refresh
            </button>
            <span
              class="pg-muted pg-small lib-fresh"
              title={liveUpdatesError ? `Live usage updates are off. ${liveUpdatesError}` : 'Library usage projection'}
            >
              Usage {lowerFirst(freshness)}{liveUpdatesError ? ', not updating live' : ''}
            </span>
          </div>
          {#if actionError || actionMessage}
            <div class="pg-muted pg-small lib-status" role="status" title={actionError ?? actionMessage ?? ''}>
              {actionError ?? actionMessage}
            </div>
          {/if}
        </div>
      {/snippet}
    </Panel>

    <Panel label="Model detail">
      {#if !selectedRow}
        <p class="lib-note lib-detail-empty">
          {modelRows.length === 0
            ? 'Models appear here once the Pumas library lists them.'
            : 'Pick a model to see where it lives and how it is used.'}
        </p>
      {:else}
        {@const row = selectedRow}
        {@const usageRows = libraryModelUsageRows(row, nowMs)}
        {@const sourceRows = libraryModelSourceRows(row, nowMs)}
        {@const workflowRows = libraryWorkflowUseRows(row, activeRunId, nowMs)}
        {@const chips = libraryModelChips(row, activeRunId)}
        <div class="lib-head">
          <div class="lib-head-text">
            <h2 class="pg-title">{row.name}{#if row.modelType}<span class="pg-muted lib-title-type">{row.modelType}</span>{/if}</h2>
            <div class="pg-mono pg-muted lib-id">{row.modelId}</div>
            {#if chips.length > 0}
              <div class="lib-chips">
                {#each chips as chip (chip)}
                  <Chip>{chip}</Chip>
                {/each}
              </div>
            {/if}
          </div>
          <div class="lib-actions">
            <button
              class="pg-btn"
              type="button"
              aria-disabled="true"
              title="Not available yet: no service starts a new graph from a library model"
            >
              Use in a new graph
            </button>
            <button
              class="pg-btn"
              type="button"
              aria-disabled="true"
              title="Not available yet: no service opens the model folder"
            >
              Open folder
            </button>
            <!-- Keyed by model: an armed confirm never carries over to another selection. -->
            {#key row.modelId}
              <TwoStepButton
                label={actionBusy === 'delete' ? 'Deleting' : 'Delete from disk'}
                confirmLabel="Confirm delete"
                small={false}
                disabled={deleteBlockReason !== null}
                disabledReason={deleteBlockReason ?? undefined}
                onconfirm={() => void deleteModel(row)}
              />
            {/key}
          </div>
        </div>

        <div class="lib-grid">
          <section class="pg-sec lib-sec-flush" aria-labelledby="lib-where">
            <h3 id="lib-where">Where it lives</h3>
            <KeyValue rows={libraryModelLocationRows(row)} />
          </section>
          <section class="pg-sec lib-sec-flush lib-sec-split" aria-labelledby="lib-how">
            <h3 id="lib-how">How it is used</h3>
            {#if usageRows.length > 0}
              <KeyValue rows={usageRows} />
            {:else if usageError}
              <div class="pg-muted pg-small">Usage could not be read. {usageError}</div>
            {:else}
              <div class="pg-muted pg-small">No usage recorded for this model.</div>
            {/if}
          </section>
        </div>

        <section class="pg-sec" aria-labelledby="lib-workflows">
          <h3 id="lib-workflows">Workflows that use it</h3>
          {#if workflowRows.length > 0}
            <table class="pg-tbl">
              <thead>
                <tr>
                  <th class="lib-th">Workflow</th>
                  <th class="lib-th">Last run</th>
                  <th class="lib-th lib-right">Runs</th>
                  <th class="lib-th">Last used</th>
                </tr>
              </thead>
              <tbody>
                {#each workflowRows as use (use.key)}
                  <tr class:lib-tr-active={use.isActiveRun} aria-current={use.isActiveRun ? 'true' : undefined}>
                    <td>
                      <b>{use.workflowId ?? 'Unknown workflow'}</b>{#if use.version}<span class="pg-ver">v{use.version}</span>{/if}
                      {#if use.isActiveRun}<span class="pg-muted pg-small lib-block">Last used by active run</span>{/if}
                    </td>
                    <td>
                      {#if use.runId}
                        <span class="pg-mono lib-run" data-run-id={use.runId} title={`${use.runId}. Ctrl+C copies the run id`}>{use.runId}</span>
                      {:else}
                        <span class="pg-muted">Not recorded</span>
                      {/if}
                    </td>
                    <td class="pg-num">{use.runCount}</td>
                    <td class="pg-muted">{use.lastUsed}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
            <div class="pg-muted pg-small lib-table-note">The usage projection keeps the most recent workflow and run for each model.</div>
          {:else if usageError}
            <div class="pg-muted pg-small">Usage could not be read. {usageError}</div>
          {:else}
            <div class="pg-muted pg-small">No workflow run has used this model yet.</div>
          {/if}
        </section>

        <section class="pg-sec" aria-labelledby="lib-source">
          <h3 id="lib-source">Source</h3>
          {#if sourceRows.length > 0}
            <KeyValue rows={sourceRows} />
          {:else}
            <div class="pg-muted pg-small">The Pumas library records no source for this model.</div>
          {/if}
        </section>
      {/if}
    </Panel>
  </div>
</div>

<style>
  .lib-body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .lib-search-row {
    display: flex;
    gap: 8px;
    align-items: flex-end;
  }

  .lib-grow {
    flex: 1;
    min-width: 0;
  }

  .lib-limit {
    flex: 0 0 72px;
  }

  .lib-search-btn {
    flex: none;
  }

  .lib-note {
    margin: 0;
    padding: 10px 12px;
    font-size: 12.5px;
    color: var(--pg-text-muted);
  }

  .lib-host {
    margin: 0;
  }

  .lib-note p {
    margin: 0;
  }

  .lib-note-action {
    margin-top: 8px;
  }

  .lib-item {
    grid-template-columns: 1fr auto;
  }

  .lib-detail {
    font-size: 12px;
  }

  .lib-res {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 4px 14px;
    align-items: center;
    padding: 8px 12px;
    border-top: 1px solid var(--pg-border-soft);
    font-size: 13px;
  }

  .lib-res:hover {
    background: var(--pg-panel-raised);
  }

  .lib-res-id {
    font-size: 12px;
    color: var(--pg-text-bright);
    overflow-wrap: anywhere;
  }

  .lib-res-sub {
    display: block;
  }

  .lib-disclosure {
    border-bottom: 1px solid var(--pg-border-soft);
    background: var(--pg-panel-head);
  }

  .lib-disclosure-btn {
    display: flex;
    gap: 8px;
    align-items: baseline;
    width: 100%;
    padding: 7px 12px;
    border: 0;
    background: transparent;
    color: var(--pg-text-soft);
    font-size: 12px;
    font-weight: 700;
    text-align: left;
    cursor: pointer;
  }

  .lib-disclosure-btn .pg-muted {
    font-weight: 400;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lib-disclosure-body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 0 12px 10px;
  }

  .lib-span {
    grid-column: 1 / -1;
  }

  .lib-foot {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 0;
  }

  .lib-foot-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .lib-fresh {
    margin-left: auto;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .lib-status {
    overflow-wrap: anywhere;
  }

  .lib-title-type {
    margin-left: 8px;
  }

  .lib-head {
    padding: 16px 18px 12px;
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .lib-head-text {
    flex: 1 1 260px;
    min-width: 0;
  }

  .lib-head h2 {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .lib-id {
    margin-top: 2px;
    overflow-wrap: anywhere;
  }

  .lib-chips {
    margin-top: 10px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .lib-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .lib-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(240px, 100%), 1fr));
    border-top: 1px solid var(--pg-border);
  }

  .lib-sec-flush {
    border-top: 0;
  }

  .lib-sec-split {
    border-left: 1px solid var(--pg-border);
  }

  .lib-th {
    position: static;
  }

  .lib-right {
    text-align: right;
  }

  .lib-block {
    display: block;
  }

  .lib-run {
    font-size: 12px;
  }

  .lib-tr-active td {
    background: var(--pg-panel-selected);
  }

  .lib-tr-active td:first-child {
    box-shadow: inset 3px 0 0 var(--pg-text);
  }

  .lib-table-note {
    margin-top: 8px;
  }

  .lib-hf-toggle {
    flex: none;
  }

  .lib-hf-toggle[aria-pressed='true'] {
    background: var(--pg-text);
    color: var(--pg-ink);
    border-color: var(--pg-text);
  }

  .lib-detail-empty {
    padding: 32px 24px;
  }
</style>
