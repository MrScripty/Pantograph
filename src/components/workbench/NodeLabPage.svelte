<script lang="ts">
  // Node Editor (spec "Node Editor"). Authoring has no backend in this build, so
  // the page reads the built-in definitions from the same store the Graph
  // palette lists (filled by the graph session at startup and by Graph on
  // mount) and renders every authoring control disabled with its reason.
  import { nodeDefinitions } from '../../stores/workflowStore';
  import {
    EmptyState,
    GroupHeader,
    ListRow,
    Panel,
    StatusGlyph,
    DimensionLine,
  } from './ui';
  import {
    NODE_LAB_CATEGORIES,
    NODE_LAB_EXECUTION_MODES,
    NODE_LAB_IO_BINDINGS,
    buildNodeLabList,
    nodeLabAuthoringNotice,
    nodeLabCategoryLabel,
    nodeLabDisabledReason,
    nodeLabExecutionModeLabel,
    nodeLabIoBindingLabel,
    nodeLabSummaryLine,
    projectNodeLabDefinition,
    resolveNodeLabSelection,
  } from './nodeLabPresenters';

  const notice = nodeLabAuthoringNotice();
  const fieldReason = nodeLabDisabledReason('edit_field');
  const removeReason = nodeLabDisabledReason('remove_port');
  const addReason = nodeLabDisabledReason('add_port');

  let query = $state('');
  let selectedNodeType = $state<string | null>(null);
  let builtInExpanded = $state(true);

  let list = $derived(buildNodeLabList($nodeDefinitions, query));
  let summary = $derived(nodeLabSummaryLine($nodeDefinitions));
  let selected = $derived(resolveNodeLabSelection($nodeDefinitions, selectedNodeType));
  let view = $derived(selected ? projectNodeLabDefinition(selected) : null);
  let categoryOptions = $derived(
    view && !NODE_LAB_CATEGORIES.includes(view.category as (typeof NODE_LAB_CATEGORIES)[number])
      ? [...NODE_LAB_CATEGORIES, view.category]
      : [...NODE_LAB_CATEGORIES],
  );
  let groupCount = $derived(list.matched === list.total ? list.total : `${list.matched} of ${list.total}`);
</script>

<div class="pg-main h-full">
  <DimensionLine label={summary} />

  <aside class="nl-notice pg-small" aria-label="Node authoring availability">
    <StatusGlyph state="queued" />
    <p><strong>{notice.headline}</strong> {notice.sentence}</p>
  </aside>

  <div class="nl-columns">
    <Panel label="Node definitions" width="300px">
      {#snippet top()}
        <div class="nl-search">
          <label for="node-lab-search" class="pg-sr-only">Search built-in nodes</label>
          <input
            id="node-lab-search"
            class="pg-field"
            type="search"
            placeholder="Search built-in nodes"
            bind:value={query}
          />
          <button class="pg-btn" type="button" aria-disabled="true" title={nodeLabDisabledReason('new_node')}>
            New node
          </button>
        </div>
      {/snippet}

      {#each list.groups as group (group.id)}
        <GroupHeader
          label={group.label}
          count={groupCount}
          expanded={builtInExpanded}
          controls="node-lab-built-in"
          ontoggle={() => (builtInExpanded = !builtInExpanded)}
        />
        <div id="node-lab-built-in" hidden={!builtInExpanded}>
          {#each group.rows as row (row.nodeType)}
            <ListRow
              glyph="idle"
              name={row.label}
              version={row.nodeType}
              statusLabel={row.categoryLabel}
              context={row.context}
              selected={row.nodeType === view?.nodeType}
              onselect={() => (selectedNodeType = row.nodeType)}
            />
          {/each}
        </div>
      {/each}
      {#if list.emptyText}
        <p class="pg-empty pg-small nl-empty">{list.emptyText}</p>
      {/if}

      {#snippet footer()}
        <span class="pg-small pg-muted">Built-in nodes are read-only. This is the list the Graph palette offers.</span>
      {/snippet}
    </Panel>

    {#if view}
      <Panel label="Definition">
        <div class="nl-head">
          <div class="nl-head-text">
            <h2 class="pg-title">{view.label}</h2>
            <div class="pg-mono pg-muted">{view.nodeType}</div>
          </div>
          <div class="nl-actions">
            <button class="pg-btn" type="button" aria-disabled="true" title={nodeLabDisabledReason('save_draft')}>
              Save draft
            </button>
            <button
              class="pg-btn pg-btn-primary"
              type="button"
              aria-disabled="true"
              title={nodeLabDisabledReason('publish')}
            >
              Publish to runtime
            </button>
          </div>
        </div>

        <div class="pg-sec nl-form">
          <label class="pg-fl">
            Label
            <input class="pg-field" value={view.label} disabled title={fieldReason} />
          </label>
          <label class="pg-fl">
            Category
            <select class="pg-field" disabled title={fieldReason}>
              {#each categoryOptions as category (category)}
                <option value={category} selected={category === view.category}>
                  {nodeLabCategoryLabel(category)}
                </option>
              {/each}
            </select>
          </label>
          <label class="pg-fl">
            Execution
            <select class="pg-field" disabled title={fieldReason}>
              {#each NODE_LAB_EXECUTION_MODES as mode (mode)}
                <option value={mode} selected={mode === view.executionMode}>{nodeLabExecutionModeLabel(mode)}</option>
              {/each}
              {#if !(NODE_LAB_EXECUTION_MODES as string[]).includes(view.executionMode)}
                <option value={view.executionMode} selected>{view.executionModeLabel}</option>
              {/if}
            </select>
          </label>
          <label class="pg-fl">
            I/O binding
            <select class="pg-field" disabled title={fieldReason}>
              {#each NODE_LAB_IO_BINDINGS as origin (origin)}
                <option value={origin} selected={origin === view.ioBindingOrigin}>{nodeLabIoBindingLabel(origin)}</option>
              {/each}
              {#if !(NODE_LAB_IO_BINDINGS as string[]).includes(view.ioBindingOrigin)}
                <option value={view.ioBindingOrigin} selected>{view.ioBindingLabel}</option>
              {/if}
            </select>
          </label>
          <label class="pg-fl nl-wide">
            Description
            <textarea class="pg-field" rows="2" value={view.description} disabled title={fieldReason}></textarea>
          </label>
        </div>

        <section class="pg-sec" aria-labelledby="node-lab-inputs">
          <h3 id="node-lab-inputs">Inputs</h3>
          {#if view.inputs.length > 0}
            <table class="nl-ports">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">Type</th>
                  <th scope="col">Default</th>
                  <th scope="col">Required</th>
                  <th scope="col"><span class="pg-sr-only">Remove</span></th>
                </tr>
              </thead>
              <tbody>
                {#each view.inputs as port (port.id)}
                  <tr>
                    <td>
                      <input class="pg-field" value={port.label} aria-label="Input {port.label} name" disabled title={fieldReason} />
                      {#if port.idNote || port.description}
                        <span class="nl-portnote">
                          {#if port.idNote}<span class="pg-mono">{port.idNote}</span>{/if}{#if port.idNote && port.description}, {/if}{port.description ?? ''}
                        </span>
                      {/if}
                    </td>
                    <td><input class="pg-field" value={port.dataType} aria-label="Input {port.label} type" disabled title={fieldReason} /></td>
                    <td><input class="pg-field" value={port.defaultText} aria-label="Input {port.label} default" disabled title={fieldReason} /></td>
                    <td class="nl-req">{port.requirement}</td>
                    <td>
                      <button class="nl-rm" type="button" aria-label="Remove input {port.label}" disabled title={removeReason}>×</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="pg-small pg-muted nl-none">This node has no inputs.</p>
          {/if}
          <button class="pg-tgl nl-add" type="button" aria-disabled="true" title={addReason}>Add input</button>
        </section>

        <section class="pg-sec" aria-labelledby="node-lab-outputs">
          <h3 id="node-lab-outputs">Outputs</h3>
          {#if view.outputs.length > 0}
            <table class="nl-ports">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">Type</th>
                  <th scope="col"><span class="pg-sr-only">Remove</span></th>
                </tr>
              </thead>
              <tbody>
                {#each view.outputs as port (port.id)}
                  <tr>
                    <td>
                      <input class="pg-field" value={port.label} aria-label="Output {port.label} name" disabled title={fieldReason} />
                      {#if port.idNote || port.description}
                        <span class="nl-portnote">
                          {#if port.idNote}<span class="pg-mono">{port.idNote}</span>{/if}{#if port.idNote && port.description}, {/if}{port.description ?? ''}
                        </span>
                      {/if}
                    </td>
                    <td><input class="pg-field" value={port.dataType} aria-label="Output {port.label} type" disabled title={fieldReason} /></td>
                    <td>
                      <button class="nl-rm" type="button" aria-label="Remove output {port.label}" disabled title={removeReason}>×</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="pg-small pg-muted nl-none">This node has no outputs.</p>
          {/if}
          <button class="pg-tgl nl-add" type="button" aria-disabled="true" title={addReason}>Add output</button>
        </section>

        <section class="pg-sec" aria-labelledby="node-lab-implementation">
          <h3 id="node-lab-implementation">Implementation</h3>
          <div class="pg-code">{view.implementationText}</div>
        </section>
      </Panel>

      <Panel label="Preview and checks" width="320px">
        {#snippet top()}
          <div class="nl-preview-head">
            <h2 class="nl-preview-title">On the canvas</h2>
            <span class="pg-muted pg-small">preview</span>
          </div>
        {/snippet}

        <div class="nl-stage">
          <div class="nl-gnode">
            <div class="nl-nh">
              <StatusGlyph state="idle" small />
              <span class="nl-nh-label">{view.label}</span>
              <span class="pg-mono nl-nh-type">{view.nodeType}</span>
            </div>
            <div class="nl-pts">
              <div class="nl-pcol">
                {#each view.inputs as port (port.id)}
                  <div class="nl-pt nl-pt-in" title={port.dataType}><span>{port.label}</span></div>
                {/each}
              </div>
              <div class="nl-pcol">
                {#each view.outputs as port (port.id)}
                  <div class="nl-pt nl-pt-out" title={port.dataType}><span>{port.label}</span></div>
                {/each}
              </div>
            </div>
          </div>
        </div>

        <section class="pg-sec nl-side-sec" aria-labelledby="node-lab-checks">
          <h3 id="node-lab-checks">Checks</h3>
          <ul class="nl-checks">
            {#each view.checks as check (check.text)}
              <li><StatusGlyph state={check.glyph} small /><span>{check.text}</span></li>
            {/each}
          </ul>
        </section>

        <section class="pg-sec nl-side-sec" aria-labelledby="node-lab-try">
          <h3 id="node-lab-try">Try it</h3>
          <p class="pg-small pg-muted nl-try-text">
            Runs the node once with sample inputs on the local engine, outside any workflow.
          </p>
          <button class="pg-btn pg-btn-sm" type="button" aria-disabled="true" title={nodeLabDisabledReason('try')}>
            Try with sample inputs
          </button>
        </section>
      </Panel>
    {:else}
      <EmptyState
        label="Definition"
        question="No node definitions are loaded yet."
        sentence="Pantograph reads the built-in nodes from the engine when the app starts and again when Graph opens. They appear here once the engine answers."
      />
    {/if}
  </div>
</div>

<style>
  .nl-notice {
    flex: none;
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 8px 12px;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background: var(--pg-panel-head);
  }

  .nl-notice p {
    margin: 0;
  }

  .nl-notice strong {
    color: var(--pg-text-bright);
  }

  .nl-columns {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .nl-search {
    display: flex;
    gap: 8px;
  }

  .nl-search input {
    flex: 1;
    min-width: 0;
  }

  .nl-empty {
    margin: 0;
  }

  .nl-head {
    padding: 16px 18px 12px;
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .nl-head-text {
    flex: 1 1 220px;
    min-width: 0;
  }

  .nl-head-text h2 {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .nl-head-text .pg-mono {
    margin-top: 2px;
  }

  .nl-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .nl-form {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(200px, 100%), 1fr));
    gap: 10px 14px;
  }

  .nl-form .nl-wide {
    grid-column: 1 / -1;
  }

  .nl-form textarea.pg-field {
    height: auto;
    padding: 6px 10px;
    resize: none;
    font-family: var(--pg-font-mono);
    font-size: 12px;
    line-height: 1.5;
  }

  /* Read-only content stays legible; the disabled state is carried by the
     cursor, the title and the notice above, not by fading the text. */
  .pg-field:disabled {
    color: var(--pg-text-soft);
    background: var(--pg-panel-head);
    border-color: var(--pg-border-soft);
    cursor: default;
    opacity: 1;
  }

  .nl-ports {
    width: 100%;
    border-collapse: collapse;
    font-size: 12.5px;
  }

  .nl-ports th {
    text-align: left;
    color: var(--pg-text-muted);
    font-size: 11.5px;
    font-weight: 700;
    padding: 4px 8px;
    border-bottom: 1px solid var(--pg-border);
    white-space: nowrap;
  }

  .nl-ports td {
    padding: 4px 8px;
    border-top: 1px solid var(--pg-border-soft);
    vertical-align: top;
  }

  .nl-ports .pg-field {
    height: 26px;
    width: 100%;
    padding: 0 8px;
    font-family: var(--pg-font-mono);
    font-size: 12px;
  }

  .nl-ports td:last-child {
    width: 32px;
  }

  .nl-req {
    color: var(--pg-text-muted);
    white-space: nowrap;
    line-height: 26px;
  }

  .nl-portnote {
    display: block;
    margin-top: 3px;
    font-size: 12px;
    color: var(--pg-text-muted);
    overflow-wrap: anywhere;
  }

  .nl-portnote .pg-mono {
    font-size: 11px;
  }

  .nl-rm {
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--pg-text-muted);
  }

  .nl-rm:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .nl-none {
    margin: 0;
  }

  .nl-add {
    margin-top: 8px;
  }

  .nl-preview-head {
    display: flex;
    align-items: center;
  }

  .nl-preview-title {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .nl-preview-head .pg-muted {
    margin-left: auto;
  }

  .nl-stage {
    margin: 12px;
    min-height: 220px;
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
    align-items: center;
    gap: 7px;
    padding: 6px 9px;
    border-bottom: 1px solid var(--pg-border);
    font-size: 12.5px;
    font-weight: 700;
    color: var(--pg-text-bright);
    min-width: 0;
  }

  .nl-nh :global(.pg-g) {
    margin-top: 0;
  }

  .nl-nh-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nl-nh-type {
    margin-left: auto;
    font-weight: 400;
    font-size: 11px;
    color: var(--pg-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    flex: 0 1 auto;
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

  .nl-side-sec {
    padding: 12px 14px;
  }

  .nl-checks {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
    font-size: 12.5px;
  }

  .nl-checks li {
    display: grid;
    grid-template-columns: 14px 1fr;
    gap: 10px;
    align-items: start;
  }

  .nl-checks li :global(.pg-g) {
    margin-top: 5px;
  }

  .nl-try-text {
    margin: 0 0 8px;
  }
</style>
