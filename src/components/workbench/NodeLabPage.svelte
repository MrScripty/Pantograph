<script lang="ts">
  // Node Editor (spec "Node Editor", board NodeEditor). The page projects the
  // NodeLab controller (PR #95) into the redesign's three panels: admitted
  // nodes on the left, the workflow draft in the centre, the preview, checks,
  // implementation identity and session cleanup on the right. All authority
  // stays in NodeLabController and NodeLabService; the page only adds view
  // state (search, selection, which control raised the last error).
  import { onMount } from 'svelte';
  import { nodeLabService } from '../../services/workflow/customNodeAuthoring';
  import { NodeLabController, type NodeLabView } from './nodeLabController';
  import { DimensionLine, EmptyState, GroupHeader, KeyValue, ListRow, Panel, StatusGlyph } from './ui';
  import {
    buildNodeLabCatalogRows,
    buildNodeLabChecks,
    buildNodeLabIdentityRows,
    buildNodeLabStatusRows,
    nodeLabCleanupMessage,
    nodeLabDraftPlaceholder,
    nodeLabErrorRegion,
    nodeLabShowsSearch,
    nodeLabSummaryLine,
    nodeLabTrustMessage,
    projectNodeLabDraft,
    resolveNodeLabSelection,
    type NodeLabAction,
  } from './nodeLabPresenters';

  let view = $state<NodeLabView>({ catalog: null, draft: null, savedPath: '', busy: false, error: null, notice: '', pendingCleanup: 0 });
  let name = $state('custom-node-workflow');
  let path = $state('');
  let query = $state('');
  let selectedNodeType = $state<string | null>(null);
  let listExpanded = $state(true);
  let lastAction = $state<NodeLabAction>('loadCatalog');
  let controller: NodeLabController;
  onMount(() => {
    controller = new NodeLabController(nodeLabService, state => { view = state; });
    void controller.loadCatalog();
    return () => { void controller.dispose(); };
  });

  /** Remembers which control acted, so its error shows beside it. */
  function act(action: NodeLabAction, operation: () => Promise<void>): void {
    lastAction = action;
    void operation();
  }

  const statusRows = buildNodeLabStatusRows().map(row => ({ key: row.label, value: row.value }));
  let summary = $derived(nodeLabSummaryLine(view));
  let showSearch = $derived(nodeLabShowsSearch(view.catalog));
  let list = $derived(buildNodeLabCatalogRows(view.catalog, view.draft, showSearch ? query : ''));
  let groupCount = $derived(list.matched === list.total ? list.total : `${list.matched} of ${list.total}`);
  let selected = $derived(resolveNodeLabSelection(view.catalog, selectedNodeType, view.draft));
  let identityRows = $derived(selected ? buildNodeLabIdentityRows(selected) : []);
  let draftView = $derived(view.draft ? projectNodeLabDraft(view.draft) : null);
  let checks = $derived(view.draft ? buildNodeLabChecks(view.draft) : []);
  let placeholder = $derived(nodeLabDraftPlaceholder(view.catalog));
  let errorRegion = $derived(view.error ? nodeLabErrorRegion(lastAction) : null);

  function createSelectedDraft(): void {
    const entry = selected;
    if (entry) act('create', () => controller.create(entry));
  }
</script>

{#snippet errorLine()}
  <div class="nl-error">
    <StatusGlyph state="failed" small />
    <p role="alert">{view.error}</p>
  </div>
{/snippet}

<section class="pg-main h-full" aria-label="NodeLab authoring" aria-busy={view.busy}>
  <DimensionLine label={summary} />

  <aside class="nl-notice pg-small" aria-label="Custom node trust">
    <StatusGlyph state="idle" />
    <p>{nodeLabTrustMessage()}</p>
  </aside>

  <div class="nl-columns">
    <div class="nl-col nl-col-list">
      <Panel label="Admitted nodes">
        {#snippet top()}
          <div class="nl-list-head">
            <h2 class="nl-h">Admitted nodes</h2>
            <button class="pg-btn pg-btn-sm" type="button" disabled={view.busy} onclick={() => act('loadCatalog', () => controller.loadCatalog())}>Refresh catalog</button>
          </div>
          {#if showSearch}
            <label for="node-lab-search" class="pg-sr-only">Search admitted nodes</label>
            <input id="node-lab-search" class="pg-field" type="search" placeholder="Search admitted nodes" bind:value={query} />
          {/if}
        {/snippet}

        {#if errorRegion === 'catalog'}
          <div class="nl-pad">{@render errorLine()}</div>
        {/if}
        {#if view.catalog === null}
          <p class="nl-note nl-pad">
            {view.busy || !view.error
              ? 'Loading admitted nodes…'
              : 'Admitted nodes are not listed until the host answers. Refresh catalog to try again.'}
          </p>
        {:else if view.catalog.length === 0}
          <p class="nl-note nl-pad">None admitted on this host.</p>
        {:else}
          <GroupHeader
            label="Admitted on this host"
            count={groupCount}
            expanded={listExpanded}
            controls="node-lab-admitted"
            ontoggle={() => (listExpanded = !listExpanded)}
          />
          <div id="node-lab-admitted" hidden={!listExpanded}>
            {#each list.rows as row (row.nodeType)}
              <ListRow
                glyph={row.glyph}
                name={row.label}
                version={row.nodeType}
                statusLabel={row.statusLabel}
                detail={row.detail}
                time={row.time}
                title={row.title}
                selected={row.nodeType === selected?.identity.node_type}
                onselect={() => (selectedNodeType = row.nodeType)}
              />
            {/each}
          </div>
          {#if list.matched === 0}
            <p class="nl-note nl-pad">No admitted node matches “{query.trim()}”.</p>
          {/if}
        {/if}

        {#snippet footer()}
          {#if selected}
            <button class="pg-btn pg-btn-primary nl-create" type="button" disabled={view.busy} onclick={createSelectedDraft}>Create {selected.label} draft</button>
          {:else}
            <span class="pg-small pg-muted">Drafts start from an admitted node.</span>
          {/if}
        {/snippet}
      </Panel>
    </div>

    <div class="nl-col nl-col-draft">
      <Panel label="Workflow draft">
        {#snippet top()}
          <form class="nl-reopen" onsubmit={event => { event.preventDefault(); act('reopen', () => controller.reopen(path)); }}>
            <label class="pg-fl nl-grow">
              Saved workflow path
              <input class="pg-field" bind:value={path} required disabled={view.busy} />
            </label>
            <button class="pg-btn" type="submit" disabled={view.busy || !path.trim()}>Reopen saved workflow</button>
          </form>
          {#if errorRegion === 'draft'}
            {@render errorLine()}
          {/if}
        {/snippet}

        {#if view.draft && draftView}
          <div class="nl-head">
            <h2 class="pg-title">Workflow draft</h2>
            <div class="pg-mono pg-muted nl-counts">
              {draftView.countsText}{#each draftView.customNodes as node (node.id)}<span class="nl-uses">{` · ${node.label} `}<span class="nl-type">{node.nodeType}</span></span>{/each}
            </div>
            <p class="pg-small pg-muted nl-lede">Saving preserves the selected implementation identity. Run saved workflows from Scheduler.</p>
            {#if view.savedPath}
              <p class="pg-small nl-saved">Saved: <code class="pg-mono">{view.savedPath}</code></p>
            {/if}
          </div>

          <section class="pg-sec" aria-labelledby="node-lab-inputs">
            <h3 id="node-lab-inputs">Inputs</h3>
            {#each draftView.sources as source (source.node.id)}
              <form
                class="nl-source"
                onsubmit={event => {
                  event.preventDefault();
                  const input = new FormData(event.currentTarget).get('text');
                  if (typeof input === 'string') act('update', () => controller.update(source.node, input));
                }}
              >
                <label class="pg-fl">
                  {source.label}
                  <textarea name="text" class="pg-field" rows="3" disabled={view.busy} value={source.text}></textarea>
                </label>
                <button class="pg-btn pg-btn-sm" type="submit" disabled={view.busy}>Apply input</button>
              </form>
            {:else}
              <p class="nl-note">This draft has no text inputs.</p>
            {/each}
          </section>

          {#if draftView.outputs.length}
            <section class="pg-sec" aria-labelledby="node-lab-outputs">
              <h3 id="node-lab-outputs">Outputs</h3>
              <ul class="nl-list">
                {#each draftView.outputs as output (output.id)}
                  <li><StatusGlyph state="queued" small /><span>{output.label}</span></li>
                {/each}
              </ul>
              <p class="nl-note">Filled when the saved workflow runs from Scheduler.</p>
            </section>
          {/if}
        {:else}
          <div class="nl-placeholder">
            <EmptyState question={placeholder.question} sentence={placeholder.sentence} />
          </div>
        {/if}

        {#snippet footer()}
          <div class="nl-foot">
            {#if view.draft}
              <form class="nl-save" onsubmit={event => { event.preventDefault(); act('save', () => controller.save(name)); }}>
                <label class="nl-save-label pg-small pg-muted">
                  Workflow name
                  <input class="pg-field" bind:value={name} required disabled={view.busy} />
                </label>
                <button class="pg-btn pg-btn-primary" type="submit" disabled={view.busy}>Save workflow</button>
              </form>
            {:else}
              <span class="pg-small pg-muted">Saved workflows run from Scheduler.</span>
            {/if}
            <p role="status" class="nl-status pg-small pg-muted">{view.busy ? 'Working…' : view.notice}</p>
          </div>
        {/snippet}
      </Panel>
    </div>

    <div class="nl-col nl-col-side">
      <Panel label="Preview and checks">
        {#snippet top()}
          <div class="nl-preview-head">
            <h2 class="nl-h">On the canvas</h2>
            <span class="pg-muted pg-small">preview</span>
          </div>
        {/snippet}

        {#if selected}
          <div class="nl-stage">
            <div class="nl-gnode">
              <div class="nl-nh">
                <StatusGlyph state="idle" small />
                <span class="nl-nh-text">
                  <span class="nl-nh-label">{selected.label}</span>
                  <span class="pg-mono nl-nh-type">{selected.identity.node_type}</span>
                </span>
              </div>
              <div class="nl-pts">
                <div class="nl-pcol">
                  {#each selected.inputs as port (port.id)}
                    <div class="nl-pt nl-pt-in" title={port.id}><span>{port.label}</span></div>
                  {/each}
                </div>
                <div class="nl-pcol">
                  {#each selected.outputs as port (port.id)}
                    <div class="nl-pt nl-pt-out" title={port.id}><span>{port.label}</span></div>
                  {/each}
                </div>
              </div>
            </div>
          </div>
          <p class="pg-small pg-muted nl-desc">{selected.description}</p>
        {:else}
          <p class="nl-note nl-pad">No admitted node to preview.</p>
        {/if}

        <section class="pg-sec nl-side-sec" aria-labelledby="node-lab-checks">
          <h3 id="node-lab-checks">Checks</h3>
          {#if view.draft}
            <ul class="nl-list" aria-label={view.draft.graph_diagnostics.length ? 'Graph diagnostics' : 'Graph contract'}>
              {#each checks as check, index (index)}
                <li><StatusGlyph state={check.glyph} small /><span>{check.text}</span></li>
              {/each}
            </ul>
          {:else}
            <p class="nl-note">Checks run on the open workflow draft.</p>
          {/if}
        </section>

        {#if selected}
          <details class="pg-sec nl-side-sec nl-identity" open>
            <summary>Implementation identity</summary>
            <KeyValue rows={identityRows} label="Implementation identity of {selected.label}" />
          </details>
        {/if}

        <section class="pg-sec nl-side-sec" aria-labelledby="node-lab-sessions">
          <h3 id="node-lab-sessions">Sessions</h3>
          {#if errorRegion === 'sessions'}
            {@render errorLine()}
          {/if}
          {#if view.pendingCleanup}
            <p role="alert" class="pg-small nl-cleanup">{nodeLabCleanupMessage(view.pendingCleanup)}</p>
            <button class="pg-btn pg-btn-sm" type="button" disabled={view.busy} onclick={() => act('retryCleanup', () => controller.retryCleanup())}>Retry session cleanup</button>
          {:else}
            <p class="nl-note">No earlier NodeLab sessions are waiting to close.</p>
          {/if}
        </section>

        <section class="pg-sec nl-side-sec" aria-labelledby="node-lab-host">
          <h3 id="node-lab-host">Authoring on this host</h3>
          <KeyValue rows={statusRows} />
        </section>
      </Panel>
    </div>
  </div>
</section>

<style>
  .nl-notice {
    flex: none;
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 12px;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background: var(--pg-panel-head);
  }

  .nl-notice p {
    margin: 0;
  }

  .nl-columns {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  /* Fixed side widths at desktop size; below that the three columns shrink
     together instead of clipping the centre (the page never scrolls). */
  .nl-col {
    min-width: 0;
    min-height: 0;
    display: flex;
  }

  .nl-col-list {
    flex: 0 1 300px;
    min-width: 200px;
  }

  .nl-col-draft {
    flex: 1 1 360px;
    min-width: 240px;
  }

  .nl-col-side {
    flex: 0 1 320px;
    min-width: 220px;
  }

  .nl-h {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .nl-list-head,
  .nl-preview-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .nl-list-head .pg-btn,
  .nl-preview-head .pg-muted {
    margin-left: auto;
  }

  .nl-pad {
    padding: 10px 12px;
  }

  .nl-note {
    margin: 0;
    font-size: 12.5px;
    color: var(--pg-text-muted);
  }

  .nl-error {
    display: grid;
    grid-template-columns: 14px 1fr;
    gap: 8px;
    align-items: start;
    font-size: 12.5px;
    color: var(--pg-text);
  }

  .nl-error p {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .nl-error :global(.pg-g) {
    margin-top: 4px;
  }

  .nl-create {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nl-reopen {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: flex-end;
  }

  .nl-grow {
    flex: 1 1 200px;
    min-width: 0;
  }

  .nl-head {
    padding: 16px 18px 12px;
  }

  .nl-head h2 {
    margin: 0;
  }

  .nl-counts {
    margin-top: 2px;
    overflow-wrap: anywhere;
  }

  .nl-type {
    color: var(--pg-text-muted);
  }

  .nl-uses {
    color: var(--pg-text-soft);
  }

  .nl-lede {
    margin: 8px 0 0;
  }

  .nl-saved {
    margin: 6px 0 0;
    overflow-wrap: anywhere;
  }

  .nl-saved code {
    font-size: 12px;
    color: var(--pg-text-soft);
  }

  .nl-source + .nl-source {
    margin-top: 12px;
  }

  .nl-source textarea.pg-field {
    height: auto;
    padding: 6px 10px;
    resize: vertical;
    font-family: var(--pg-font-mono);
    font-size: 12px;
    line-height: 1.5;
  }

  .nl-source .pg-btn {
    margin-top: 6px;
  }

  .nl-placeholder {
    height: 100%;
    display: flex;
  }

  /* The centre panel already draws the border; the empty state sits inside it. */
  .nl-placeholder :global(.pg-emptystate) {
    border: 0;
    background: transparent;
  }

  .nl-foot {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .nl-save {
    flex: 1 1 260px;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .nl-save-label {
    flex: 1 1 180px;
    min-width: 0;
    display: flex;
    gap: 8px;
    align-items: center;
    white-space: nowrap;
  }

  .nl-save-label .pg-field {
    flex: 1;
    min-width: 0;
  }

  .nl-status {
    margin: 0 0 0 auto;
    white-space: nowrap;
  }

  .nl-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
    font-size: 12.5px;
  }

  .nl-list li {
    display: grid;
    grid-template-columns: 14px 1fr;
    gap: 10px;
    align-items: start;
    overflow-wrap: anywhere;
  }

  .nl-list li :global(.pg-g) {
    margin-top: 5px;
  }

  .nl-list + .nl-note {
    margin-top: 8px;
  }

  .nl-stage {
    margin: 12px;
    min-height: 180px;
    padding: 24px 16px;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background-color: var(--pg-panel-head);
    background-image: radial-gradient(var(--pg-border) 1px, transparent 1px);
    background-size: 22px 22px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Same visual language as the graph node card (board .gnode, BaseNode). */
  .nl-gnode {
    width: 220px;
    max-width: 100%;
    background: var(--pg-panel);
    border: 1px solid var(--pg-scroll-thumb);
    border-radius: 4px;
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.25);
  }

  .nl-nh {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    padding: 6px 9px;
    border-bottom: 1px solid var(--pg-border);
    font-size: 12.5px;
    font-weight: 700;
    color: var(--pg-text-bright);
    min-width: 0;
  }

  .nl-nh-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .nl-nh-label,
  .nl-nh-type {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nl-nh-type {
    font-weight: 400;
    font-size: 11px;
    color: var(--pg-text-muted);
  }

  .nl-pts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 8px;
    padding: 4px 0;
  }

  .nl-pcol {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .nl-pt {
    position: relative;
    padding: 3px 9px;
    font-size: 11.5px;
    color: var(--pg-text-soft);
  }

  .nl-pt span {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nl-pt::before {
    content: '';
    position: absolute;
    top: 50%;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--pg-panel-head);
    border: 1.5px solid var(--pg-border-strong);
    transform: translateY(-50%);
  }

  .nl-pt-in::before {
    left: -4px;
  }

  .nl-pt-out {
    text-align: right;
  }

  .nl-pt-out::before {
    right: -4px;
  }

  .nl-desc {
    margin: 0 14px 12px;
  }

  .nl-side-sec {
    padding: 12px 14px;
  }

  .nl-identity summary {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 700;
    color: var(--pg-text-bright);
    cursor: pointer;
  }

  .nl-identity :global(.pg-kv) {
    font-size: 12px;
  }

  .nl-identity :global(.pg-kv .pg-mono) {
    font-size: 11.5px;
  }

  .nl-cleanup {
    margin: 0 0 8px;
  }
</style>
