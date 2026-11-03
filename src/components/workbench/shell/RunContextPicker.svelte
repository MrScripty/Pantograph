<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { activeWorkflowRun } from '../../../stores/workbenchStore';
  import StatusGlyph from '../ui/StatusGlyph.svelte';
  import Toast from '../ui/Toast.svelte';
  import {
    createRunContextClipboard,
    type RunContextClipboard,
    type RunCopyOutcome,
    type RunPasteSource,
  } from './runContextClipboard';
  import {
    groupRunsForContextPicker,
    resolveRunContextInput,
    runContextChip,
    runContextEmptyText,
    shortRunId,
  } from './runContextPicker';
  import {
    clearRunContext,
    closeRunContextPicker,
    openRunContextPicker,
    refreshRunContextRunList,
    runContextLiveUpdates,
    runContextPicker,
    runContextRunList,
    selectRunContext,
    setRunContextPickerQuery,
    startRunContextSync,
  } from './runContextStore';

  const COPY_TOAST_MS = 1600;
  const POPOVER_ID = 'run-context-popover';
  const SEARCH_ID = 'run-context-search';

  let wrapElement: HTMLDivElement | null = $state(null);
  let chipButton: HTMLButtonElement | null = $state(null);
  let searchInput: HTMLInputElement | null = $state(null);
  let toastMessage = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  let clipboard: RunContextClipboard | null = null;
  let restoreFocusOnClose = true;
  let wasOpen = false;

  let chip = $derived(runContextChip($activeWorkflowRun, $runContextRunList.runs));
  let groups = $derived(
    $runContextPicker.open
      ? groupRunsForContextPicker($runContextRunList.runs, {
          query: $runContextPicker.query,
          selectedRunId: $activeWorkflowRun?.workflow_run_id ?? null,
          nowMs: $runContextRunList.loadedAtMs ?? 0,
        })
      : [],
  );
  let emptyText = $derived(
    runContextEmptyText({
      query: $runContextPicker.query,
      runCount: $runContextRunList.runs.length,
      availability: $runContextRunList.availability,
      loading: $runContextRunList.status === 'loading' && $runContextRunList.loadedAtMs === null,
    }),
  );

  function showToast(message: string): void {
    if (toastTimer) clearTimeout(toastTimer);
    toastMessage = message;
    toastTimer = setTimeout(() => {
      toastMessage = null;
      toastTimer = null;
    }, COPY_TOAST_MS);
  }

  function handleCopied(runId: string, outcome: RunCopyOutcome): void {
    showToast(
      outcome === 'clipboard'
        ? `Copied ${runId}`
        : `Copied ${runId} (system clipboard unavailable, pastes in this window only)`,
    );
  }

  function handlePaste(text: string, _source: RunPasteSource): void {
    const resolution = resolveRunContextInput(text, $runContextRunList.runs);
    if (resolution.kind === 'select') {
      selectRunContext(resolution.run);
      return;
    }
    openRunContextPicker(resolution.kind === 'empty' ? '' : text);
  }

  function togglePicker(): void {
    if ($runContextPicker.open) {
      restoreFocusOnClose = true;
      closeRunContextPicker();
    } else {
      openRunContextPicker();
    }
  }

  function handleSearchInput(event: Event): void {
    const value = (event.currentTarget as HTMLInputElement).value;
    const resolution = resolveRunContextInput(value, $runContextRunList.runs);
    if (resolution.kind === 'select') {
      restoreFocusOnClose = true;
      selectRunContext(resolution.run);
      return;
    }
    setRunContextPickerQuery(value);
  }

  function clearFromChip(): void {
    clearRunContext();
    // The clear button unmounts with the run chip; keep focus on the chip.
    void tick().then(() => chipButton?.focus());
  }

  function pickRow(run: Parameters<typeof selectRunContext>[0]): void {
    restoreFocusOnClose = true;
    selectRunContext(run);
  }

  function handleDocumentKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && $runContextPicker.open) {
      event.preventDefault();
      restoreFocusOnClose = true;
      closeRunContextPicker();
    }
  }

  function handleDocumentPointerDown(event: PointerEvent): void {
    if (!$runContextPicker.open || !wrapElement) return;
    if (event.target instanceof Node && wrapElement.contains(event.target)) return;
    // The pointer chose a new focus target; do not pull focus back to the chip.
    restoreFocusOnClose = false;
    closeRunContextPicker();
  }

  // Open/close transitions: refresh and focus the search field on open;
  // invalidate pending clipboard reads and return focus to the chip on close.
  $effect(() => {
    const open = $runContextPicker.open;
    if (open && !wasOpen) {
      void refreshRunContextRunList();
      void tick().then(() => searchInput?.focus());
    } else if (!open && wasOpen) {
      clipboard?.cancelPendingPaste();
      if (restoreFocusOnClose) void tick().then(() => chipButton?.focus());
      restoreFocusOnClose = true;
    }
    wasOpen = open;
  });

  // One owner for the run-list sync and the clipboard listeners: attached when
  // the shell mounts this picker, removed when it is destroyed.
  onMount(() => {
    const stopSync = startRunContextSync();
    const systemClipboard = typeof navigator !== 'undefined' ? navigator.clipboard : undefined;
    const controller = createRunContextClipboard({
      target: document,
      writeText: systemClipboard?.writeText ? (text) => systemClipboard.writeText(text) : undefined,
      readText: systemClipboard?.readText ? () => systemClipboard.readText() : undefined,
      hasTextSelection: () => (window.getSelection()?.toString().length ?? 0) > 0,
      onCopied: handleCopied,
      onPaste: handlePaste,
    });
    controller.attach();
    clipboard = controller;

    return () => {
      controller.detach();
      clipboard = null;
      stopSync();
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = null;
    };
  });
</script>

<svelte:document onkeydown={handleDocumentKeydown} onpointerdown={handleDocumentPointerDown} />

<div class="ctxwrap" bind:this={wrapElement} data-run-context-chip>
  {#if chip.kind === 'run'}
    <div class="ctx" data-run-id={chip.runId}>
      <button
        bind:this={chipButton}
        class="ctxbtn"
        type="button"
        aria-haspopup="dialog"
        aria-expanded={$runContextPicker.open}
        aria-controls={POPOVER_ID}
        aria-label={`Run ${chip.workflowLabel}${chip.versionLabel ? ` ${chip.versionLabel}` : ''}, ${chip.statusLabel}. Change run`}
        title={`${chip.runId}, click to change run. Ctrl+C copies the id, Ctrl+V views a copied run`}
        onclick={togglePicker}
      >
        <StatusGlyph state={chip.glyph} />
        <span class="ctxname">{chip.workflowLabel}{#if chip.versionLabel}<span class="pg-muted ctxver">{chip.versionLabel}</span>{/if}</span>
        <span class="pg-muted caret" aria-hidden="true">▾</span>
      </button>
      <button class="x" type="button" aria-label="Clear run context" title="Clear run context" onclick={clearFromChip}>×</button>
    </div>
  {:else}
    <button
      bind:this={chipButton}
      class="ctx ctxbtn ctxempty"
      type="button"
      aria-haspopup="dialog"
      aria-expanded={$runContextPicker.open}
      aria-controls={POPOVER_ID}
      title="Pick the run every tab shows. Ctrl+V over this chip views a copied run"
      onclick={togglePicker}
    >
      Pick a run <span class="caret" aria-hidden="true">▾</span>
    </button>
  {/if}

  {#if $runContextPicker.open}
    <div class="pop" id={POPOVER_ID} role="dialog" aria-label="Pick a run">
      <div class="popsearch">
        <label for={SEARCH_ID} class="pg-sr-only">Search runs or paste a run id</label>
        <input
          id={SEARCH_ID}
          bind:this={searchInput}
          class="pg-field popfield"
          type="search"
          autocomplete="off"
          spellcheck="false"
          placeholder="Search runs, or paste a run id"
          value={$runContextPicker.query}
          oninput={handleSearchInput}
        />
      </div>
      <div class="poplist">
        {#each groups as group (group.id)}
          <div class="pophead">{group.title} <span class="pg-n">{group.rows.length}</span></div>
          {#each group.rows as row (row.runId)}
            <button
              class="poprow"
              type="button"
              aria-current={row.selected ? 'true' : undefined}
              data-run-id={row.runId}
              title={`${row.runId}. Ctrl+C copies the run id`}
              onclick={() => pickRow(row.record)}
            >
              <StatusGlyph state={row.glyph} />
              <span class="min-w-0">
                <b>{row.workflowId}</b>{#if row.versionLabel}<span class="pg-ver">{row.versionLabel}</span>{/if}
                <span class="pg-sub popsub"><span class:pg-st-failed={row.statusIsFailure}>{row.statusLabel}</span>{row.contextSuffix}</span>
              </span>
              <span class="poptime">{shortRunId(row.runId)}</span>
            </button>
          {/each}
        {/each}
        {#if groups.length === 0}
          <div class="pg-muted pg-small popempty">{emptyText}</div>
        {/if}
      </div>
      <div class="popfoot">
        Hover a run and press Ctrl+C to copy its id. Hover this chip and press Ctrl+V to view a copied run.
        {#if $runContextLiveUpdates === 'unavailable'}
          Live updates are unavailable, so this list refreshes each time you open it.
        {/if}
      </div>
    </div>
  {/if}
</div>

<Toast message={toastMessage} />

<style>
  .ctxwrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: none;
  }

  .ctx {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    padding: 0 4px 0 10px;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background: var(--pg-panel-head);
    font-size: 12.5px;
    max-width: 320px;
    min-width: 0;
    white-space: nowrap;
  }

  .ctx .x {
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--pg-text-muted);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }

  .ctx .x:hover {
    background: var(--pg-border);
    color: var(--pg-text-bright);
  }

  .ctxbtn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 24px;
    border: 0;
    background: transparent;
    color: var(--pg-text);
    cursor: pointer;
    padding: 0;
    max-width: 280px;
    min-width: 0;
    font-size: 12.5px;
  }

  .ctxname {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ctxver {
    margin-left: 5px;
  }

  .ctxbtn:hover .ctxname {
    color: var(--pg-text-bright);
  }

  .caret {
    font-size: 10px;
  }

  .ctx.ctxempty {
    height: 26px;
    padding: 0 10px;
    color: var(--pg-text-quiet);
    border: 1px dashed var(--pg-border);
    background: transparent;
  }

  .ctx.ctxempty:hover {
    color: var(--pg-text-bright);
    border-color: var(--pg-border-strong);
  }

  .pop {
    position: absolute;
    right: 0;
    top: 32px;
    width: 400px;
    max-width: calc(100vw - 24px);
    z-index: 60;
    background: var(--pg-panel-sunken);
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    max-height: 70vh;
  }

  .popsearch {
    padding: 10px 10px 6px;
  }

  .popfield {
    width: 100%;
  }

  .poplist {
    overflow: auto;
    min-height: 0;
    flex: 1;
  }

  .pophead {
    padding: 6px 12px;
    border-top: 1px solid var(--pg-border-soft);
    background: var(--pg-panel-head);
    color: var(--pg-text-soft);
    font-size: 12px;
    font-weight: 700;
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .poprow {
    display: grid;
    grid-template-columns: 14px 1fr auto;
    gap: 10px;
    align-items: start;
    width: 100%;
    padding: 7px 12px;
    border: 0;
    border-top: 1px solid var(--pg-border-soft);
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: var(--pg-text);
    font-size: 13px;
  }

  .poprow :global(.pg-g) {
    margin-top: 4px;
  }

  .poprow:hover {
    background: var(--pg-panel-raised);
  }

  .poprow[aria-current='true'] {
    background: var(--pg-panel-selected);
    box-shadow: inset 3px 0 0 var(--pg-text);
  }

  .poprow b {
    color: var(--pg-text-bright);
  }

  .popsub {
    font-size: 12px;
  }

  .poptime {
    font-family: var(--pg-font-mono);
    font-size: 11px;
    color: var(--pg-text-muted);
    padding-top: 3px;
    white-space: nowrap;
  }

  .popempty {
    padding: 10px 12px;
  }

  .popfoot {
    padding: 8px 12px;
    border-top: 1px solid var(--pg-border-soft);
    font-size: 11.5px;
    color: var(--pg-border-strong);
    line-height: 1.4;
  }
</style>
