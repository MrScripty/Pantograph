<script lang="ts">
  // Sessions tab (plan M8). A projection over existing authority only:
  // saved workflows (`list_workflows`), orchestrations (`list_orchestrations`)
  // and the shell's run-list projection. There is no session service, so
  // session-level actions render disabled with a reason; nothing is persisted.
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import type { WorkflowMetadata } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import {
    deleteWorkflowByName,
    graphSessionError,
    loadWorkflowByName,
  } from '../../stores/graphSessionStore';
  import {
    loadOrchestrationList,
    orchestrationList,
    type OrchestrationGraphMetadata,
  } from '../../stores/orchestrationStore';
  import { setSchedulerRunFilters } from '../../stores/schedulerRunListStore';
  import { activeWorkflowRun, setWorkbenchPage } from '../../stores/workbenchStore';
  import { runContextRunList, selectRunContext } from './shell/runContextStore';
  import {
    buildSavedSessions,
    buildSessionDetail,
    createLatestOnlyGate,
    groupSessions,
    savedSourceFailure,
    schedulerSearchForSession,
    sessionDetailEmptyText,
    sessionFilterOptions,
    sessionSourceFromResult,
    sessionsDimensionLine,
    NO_SESSION_SERVICE_REASON,
    type InvocationOutcome,
    type RunOutcomeKind,
    type SessionEntry,
    type SessionSourceState,
    type SessionsFilter,
  } from './sessionsPresenters';
  import {
    Chip,
    DimensionLine,
    ErrorCard,
    GroupHeader,
    KeyValue,
    ListRow,
    Panel,
    SegmentedControl,
    StatusGlyph,
    TwoStepButton,
  } from './ui';

  type ActionState =
    | { key: string; status: 'busy' }
    | { key: string; status: 'failed'; message: string }
    | null;

  let workflows = $state<SessionSourceState<WorkflowMetadata>>({ status: 'loading' });
  let orchestrations = $state<SessionSourceState<OrchestrationGraphMetadata>>({ status: 'loading' });
  let filter = $state<SessionsFilter>('all');
  let query = $state('');
  let selectedKey = $state<string | null>(null);
  let collapsed = $state<Record<string, boolean>>({});
  let openState = $state<ActionState>(null);
  let deleteState = $state<ActionState>(null);

  // Current-invocation authority for this page instance, one gate per
  // operation. Gates open on mount and close on destroy, so a call that
  // finishes after the page is left, or after a newer call, never applies.
  const loadGate = createLatestOnlyGate();
  const openGate = createLatestOnlyGate();
  const deleteGate = createLatestOnlyGate();

  // Wording such as "now" and "Today" is recomputed whenever the sources or
  // the run list change; no clock timer is needed for a projection this coarse.
  let saved = $derived(
    buildSavedSessions(workflows, orchestrations, $runContextRunList.runs, Date.now()),
  );
  let groups = $derived(groupSessions(saved, filter, query));
  let filterOptions = $derived(sessionFilterOptions(saved));
  let visible = $derived(groups.flatMap((group) => group.entries));
  let selected = $derived<SessionEntry | null>(
    visible.find((entry) => entry.key === selectedKey) ?? visible[0] ?? null,
  );
  let detail = $derived(
    selected
      ? buildSessionDetail(selected, $activeWorkflowRun?.workflow_run_id ?? null, Date.now())
      : null,
  );
  let dimLine = $derived(
    sessionsDimensionLine({
      workflows,
      orchestrations,
      saved,
      runs: $runContextRunList.runs,
      runsLoaded: $runContextRunList.loadedAtMs !== null,
    }),
  );
  let sourceFailure = $derived(savedSourceFailure(workflows, orchestrations));
  let loadingSaved = $derived(workflows.status === 'loading' || orchestrations.status === 'loading');

  const STRIP_CLASS: Record<RunOutcomeKind, string> = {
    running: 'ss-seg-running',
    queued: 'ss-seg-queued',
    succeeded: 'ss-seg-succeeded',
    failed: 'ss-seg-failed',
    cancelled: 'ss-seg-cancelled',
  };

  async function listOrchestrations(): Promise<OrchestrationGraphMetadata[]> {
    // The orchestration store owns `list_orchestrations` and its list store.
    await loadOrchestrationList();
    return get(orchestrationList);
  }

  /** Load both saved sources; only the newest load on a mounted page applies. */
  async function loadSavedSessions(): Promise<InvocationOutcome> {
    const token = loadGate.begin();
    // allSettled observes both calls' failures; neither rejects this function.
    const [workflowResult, orchestrationResult] = await Promise.allSettled([
      workflowService.listWorkflows(),
      listOrchestrations(),
    ]);
    const outcome = loadGate.classify(token);
    if (outcome !== 'current') return outcome;
    workflows = sessionSourceFromResult(workflowResult);
    orchestrations = sessionSourceFromResult(orchestrationResult);
    return outcome;
  }

  onMount(() => {
    loadGate.open();
    openGate.open();
    deleteGate.open();
    void loadSavedSessions();
    return () => {
      loadGate.close();
      openGate.close();
      deleteGate.close();
    };
  });

  function errorMessage(error: unknown, fallback: string): string {
    if (error instanceof Error && error.message.trim()) return error.message;
    if (typeof error === 'string' && error.trim()) return error;
    return fallback;
  }

  async function openGraph(entry: SessionEntry): Promise<void> {
    if (entry.kind !== 'workflow' || openState?.status === 'busy') return;
    const token = openGate.begin();
    openState = { key: entry.key, status: 'busy' };
    let loaded = false;
    let failure: string | null = null;
    try {
      // Same loader the Graph selector uses; it owns the edit-session swap.
      loaded = await loadWorkflowByName(entry.id);
      if (!loaded) {
        failure = get(graphSessionError) ?? 'Another graph started loading before this one finished.';
      }
    } catch (error) {
      failure = errorMessage(error, 'The graph loader gave no reason.');
    }
    // Leaving the page or starting another open supersedes this one: no tab switch.
    if (openGate.classify(token) !== 'current') return;
    if (loaded) {
      openState = null;
      setWorkbenchPage('graph');
    } else {
      openState = { key: entry.key, status: 'failed', message: failure ?? 'The graph did not load.' };
    }
  }

  async function deleteWorkflow(entry: SessionEntry): Promise<void> {
    if (entry.kind !== 'workflow' || deleteState?.status === 'busy') return;
    const token = deleteGate.begin();
    deleteState = { key: entry.key, status: 'busy' };
    // The graph session store owns the call, its error text and the Graph reset.
    const deleted = await deleteWorkflowByName(entry.id);
    if (deleteGate.classify(token) !== 'current') return;
    if (!deleted) {
      deleteState = {
        key: entry.key,
        status: 'failed',
        message: get(graphSessionError) ?? `The workflow "${entry.name}" was not deleted.`,
      };
      return;
    }
    deleteState = null;
    if (selectedKey === entry.key) selectedKey = null;
    await loadSavedSessions();
  }

  function showInScheduler(entry: SessionEntry): void {
    setSchedulerRunFilters({ search: schedulerSearchForSession(entry) });
    setWorkbenchPage('scheduler');
  }

  function toggleGroup(id: string): void {
    collapsed = { ...collapsed, [id]: !collapsed[id] };
  }
</script>

<div class="pg-main h-full">
  <DimensionLine label={dimLine} />

  <div class="ss-body">
    <Panel label="Sessions" width="400px">
      {#snippet top()}
        <div class="ss-searchrow">
          <label for="sessions-search" class="pg-sr-only">Search sessions</label>
          <input
            id="sessions-search"
            class="pg-field ss-search"
            type="search"
            placeholder="Search sessions"
            bind:value={query}
          />
          <button class="pg-btn" type="button" aria-disabled="true" title={NO_SESSION_SERVICE_REASON}>
            New session
          </button>
        </div>
        <SegmentedControl label="Show" options={filterOptions} value={filter} onchange={(next) => (filter = next)} />
      {/snippet}

      {#each groups as group (group.id)}
        {@const expanded = !collapsed[group.id]}
        <GroupHeader
          label={group.label}
          count={group.count}
          {expanded}
          ontoggle={() => toggleGroup(group.id)}
          controls={`sessions-group-${group.id}`}
        />
        {#if expanded}
          <div id={`sessions-group-${group.id}`}>
            {#if group.id === 'saved' && sourceFailure}
              <div class="ss-pad">
                <ErrorCard
                  title={sourceFailure.title}
                  code={sourceFailure.code}
                  explanation={sourceFailure.explanation}
                  suggestion="Check that the local engine is running, then retry."
                  actionLabel="Retry"
                  onaction={() => void loadSavedSessions()}
                />
              </div>
            {/if}
            {#if group.id === 'saved' && loadingSaved}
              <p class="ss-note pg-muted pg-small">Loading saved workflows and orchestrations…</p>
            {:else}
              {#each group.entries as entry (entry.key)}
                <button
                  type="button"
                  class="pg-item"
                  aria-current={selected?.key === entry.key ? 'true' : undefined}
                  title={entry.id}
                  onclick={() => (selectedKey = entry.key)}
                >
                  <StatusGlyph state={entry.glyph} />
                  <span class="min-w-0">
                    <span class="pg-name">{entry.name}</span> <span class="pg-ver">{entry.tag}</span>
                    <span class="pg-sub">{entry.sub}</span>
                    <span class="ss-strip" aria-hidden="true">
                      {#each entry.strip as segment (segment.kind)}
                        <i class="ss-seg {STRIP_CLASS[segment.kind]}" style:flex="{segment.count} 1 0"></i>
                      {/each}
                    </span>
                    <span class="pg-sr-only">{entry.stripLabel}</span>
                  </span>
                  <span class="ss-cnt">{entry.runCountLabel} <span class="ss-when">{entry.whenLabel}</span></span>
                </button>
              {/each}
              {#if group.note && !(group.id === 'saved' && sourceFailure && saved.length === 0)}
                <p class="ss-note pg-muted pg-small">{group.note}</p>
              {/if}
            {/if}
          </div>
        {/if}
      {/each}

      {#snippet footer()}
        <span class="pg-small pg-muted">
          Saved sessions are your saved workflows and orchestrations. Runs stay in the Scheduler history.
        </span>
      {/snippet}
    </Panel>

    <Panel label="Session detail">
      {#if detail && selected}
        {@const entry = selected}
        <div class="ss-head">
          <div class="ss-headtext">
            <div class="pg-title">{detail.title}</div>
            {#if detail.id !== detail.title}
              <div class="pg-mono pg-muted ss-id">{detail.id}</div>
            {/if}
            <div class="ss-chips">
              <Chip glyph={detail.glyph}>{detail.stateLabel}</Chip>
              {#each detail.chips as chip (chip)}
                <Chip>{chip}</Chip>
              {/each}
            </div>
          </div>
          <div class="ss-actions">
            {#each detail.actions as action (action.id)}
              {#if action.id === 'delete' && action.disabledReason === null}
                <TwoStepButton
                  label={deleteState?.key === entry.key && deleteState.status === 'busy' ? 'Deleting…' : action.label}
                  confirmLabel="Confirm delete"
                  small={false}
                  disabled={deleteState?.key === entry.key && deleteState.status === 'busy'}
                  disabledReason="Deleting the saved workflow"
                  onconfirm={() => void deleteWorkflow(entry)}
                />
              {:else}
                <button
                  type="button"
                  class="pg-btn"
                  class:pg-btn-danger={action.id === 'delete'}
                  aria-disabled="true"
                  title={action.disabledReason ?? undefined}
                >
                  {action.label}
                </button>
              {/if}
            {/each}
          </div>
        </div>
        {#if deleteState?.key === entry.key && deleteState.status === 'failed'}
          <div class="ss-pad">
            <ErrorCard
              title="The workflow was not deleted"
              code="delete_workflow"
              explanation={deleteState.message}
              suggestion="Check that the local engine is running and the file is not locked, then delete again."
            />
          </div>
        {/if}

        <div class="ss-grid">
          <section class="pg-sec ss-sec-first" aria-labelledby="sessions-activity">
            <h3 id="sessions-activity">Activity</h3>
            <KeyValue rows={detail.activity} />
          </section>
          <section class="pg-sec ss-sec-first ss-sec-side" aria-labelledby="sessions-runs">
            <h3 id="sessions-runs">Runs from this session</h3>
            <KeyValue rows={detail.runSummary} />
            <div class="ss-btnrow">
              <button
                class="pg-btn pg-btn-sm"
                type="button"
                title="Opens Scheduler with its search set to {detail.id}"
                onclick={() => showInScheduler(entry)}
              >
                Show in Scheduler
              </button>
            </div>
          </section>
        </div>

        {#if detail.recentRuns.length > 0}
          <section class="pg-sec ss-runs" aria-labelledby="sessions-recent">
            <h3 id="sessions-recent">Recent runs</h3>
            <div class="ss-runlist">
              {#each detail.recentRuns as row (row.runId)}
                <ListRow
                  glyph={row.glyph}
                  name={row.workflowId}
                  version={row.versionLabel}
                  statusLabel={row.statusLabel}
                  statusDanger={row.statusIsFailure}
                  context={row.contextSuffix}
                  time={row.runId}
                  runId={row.runId}
                  selected={row.selected}
                  onselect={() => selectRunContext(row.record)}
                />
              {/each}
            </div>
            {#if detail.moreRunCount > 0}
              <p class="pg-small pg-muted ss-more">
                {detail.moreRunCount} more in Scheduler. Selecting a run makes it the run every tab shows.
              </p>
            {:else}
              <p class="pg-small pg-muted ss-more">Selecting a run makes it the run every tab shows.</p>
            {/if}
          </section>
        {/if}

        <section class="pg-sec" aria-labelledby="sessions-open">
          <h3 id="sessions-open">Open in this session</h3>
          {#each detail.open as item (item.name)}
            <div class="ss-open">
              <StatusGlyph state={item.glyph} />
              <span class="min-w-0">
                <b>{item.name}</b> <span class="pg-mono ss-meta">{item.meta}</span>
                <span class="pg-muted pg-small ss-opensub">{item.sub}</span>
              </span>
              {#if item.disabledReason}
                <button class="pg-btn pg-btn-sm" type="button" aria-disabled="true" title={item.disabledReason}>
                  {item.actionLabel}
                </button>
              {:else}
                {@const opening = openState?.key === entry.key && openState.status === 'busy'}
                <button
                  class="pg-btn pg-btn-sm"
                  type="button"
                  aria-disabled={opening ? 'true' : undefined}
                  title="Loads the saved workflow into the Graph tab for editing"
                  onclick={() => void openGraph(entry)}
                >
                  {opening ? 'Opening…' : item.actionLabel}
                </button>
              {/if}
            </div>
          {/each}
          {#if openState?.key === entry.key && openState.status === 'failed'}
            <div class="ss-errgap">
              <ErrorCard
                title="The graph did not open"
                code="load_workflow"
                explanation={openState.message}
                suggestion="Check that the saved file still exists, then open it again."
                actionLabel="Open again"
                onaction={() => void openGraph(entry)}
              />
            </div>
          {/if}
        </section>

        <section class="pg-sec" aria-labelledby="sessions-resume">
          <h3 id="sessions-resume">What resumes</h3>
          <p class="pg-small pg-muted ss-para">{detail.resumeSentence}</p>
        </section>
      {:else}
        <div class="pg-empty">
          {sessionDetailEmptyText({
            loading: loadingSaved,
            failed: sourceFailure !== null,
            savedCount: saved.length,
            filter,
            query,
          })}
        </div>
      {/if}
    </Panel>
  </div>
</div>

<style>
  .ss-body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .ss-searchrow {
    display: flex;
    gap: 8px;
  }

  .ss-search {
    flex: 1;
    min-width: 0;
  }

  .ss-pad {
    padding: 10px 12px;
  }

  .ss-note {
    margin: 0;
    padding: 10px 12px 12px 36px;
  }

  .ss-strip {
    display: flex;
    gap: 2px;
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
    background: var(--pg-panel-sunken);
    margin-top: 6px;
  }

  .ss-seg {
    display: block;
    height: 100%;
  }

  .ss-seg-running {
    background: var(--pg-accent);
  }

  .ss-seg-succeeded {
    background: var(--pg-border-strong);
  }

  .ss-seg-failed {
    background: var(--pg-danger);
  }

  .ss-seg-queued {
    background: transparent;
    border: 1px solid var(--pg-border-idle);
    border-radius: 2px;
  }

  .ss-seg-cancelled {
    background: transparent;
    border: 1px dashed var(--pg-border-idle);
    border-radius: 2px;
  }

  .ss-cnt {
    font-family: var(--pg-font-mono);
    font-size: 11.5px;
    color: var(--pg-text-muted);
    padding-top: 2px;
    text-align: right;
    white-space: nowrap;
  }

  .ss-when {
    display: block;
    font-size: 11px;
  }

  .ss-head {
    padding: 16px 18px 12px;
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .ss-headtext {
    flex: 1 1 260px;
    min-width: 0;
  }

  .ss-id {
    margin-top: 2px;
    overflow-wrap: anywhere;
  }

  .ss-chips {
    margin-top: 10px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .ss-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .ss-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(240px, 100%), 1fr));
    border-top: 1px solid var(--pg-border);
  }

  .ss-sec-first {
    border-top: 0;
  }

  .ss-sec-side {
    border-left: 1px solid var(--pg-border);
  }

  .ss-btnrow {
    margin-top: 10px;
  }

  .ss-runs {
    padding-left: 0;
    padding-right: 0;
    padding-bottom: 8px;
  }

  .ss-runs h3,
  .ss-more {
    padding: 0 18px;
  }

  .ss-more {
    margin: 8px 0 0;
  }

  .ss-runlist {
    border-bottom: 1px solid var(--pg-border-soft);
  }

  .ss-open {
    display: grid;
    grid-template-columns: 14px 1fr auto;
    gap: 6px 10px;
    align-items: center;
    padding: 7px 0;
    border-top: 1px solid var(--pg-border-soft);
    font-size: 13px;
  }

  .ss-open:first-of-type {
    border-top: 0;
  }

  .ss-meta {
    font-size: 11.5px;
    color: var(--pg-text-muted);
  }

  .ss-opensub {
    display: block;
    overflow-wrap: anywhere;
  }

  .ss-errgap {
    margin-top: 10px;
  }

  .ss-para {
    margin: 0;
  }
</style>
