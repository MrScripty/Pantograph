<script lang="ts">
  import { MessageSquare } from 'lucide-svelte';
  import { panelWidth, sidePanelOpen, toggleSidePanel } from '../../stores/panelStore';
  import {
    selectedWorkbenchPage,
    setWorkbenchPage,
    WORKBENCH_TAB_GROUPS,
    workbenchPagesInGroup,
  } from '../../stores/workbenchStore';
  import DiagnosticsPage from './DiagnosticsPage.svelte';
  import GraphPage from './GraphPage.svelte';
  import IoInspectorPage from './IoInspectorPage.svelte';
  import LibraryPage from './LibraryPage.svelte';
  import NetworkPage from './NetworkPage.svelte';
  import NodeLabPage from './NodeLabPage.svelte';
  import SchedulerPage from './SchedulerPage.svelte';
  import SessionsPage from './SessionsPage.svelte';
  import SettingsPage from './SettingsPage.svelte';
  import RunContextPicker from './shell/RunContextPicker.svelte';
  import { engineLineParts, engineLineText } from './shell/runContextPicker';
  import { refreshRunContextRunList, runContextRunList } from './shell/runContextStore';

  const settingsPage = workbenchPagesInGroup('settings')[0];

  let availability = $derived($runContextRunList.availability);
  let engineLine = $derived(engineLineParts(availability));
  let engineUnreachable = $derived(availability.kind === 'unreachable');
  let retrying = $derived(engineUnreachable && $runContextRunList.status === 'loading');
</script>

<!-- Right padding follows the Assistant panel so it never covers the header or a page. -->
<div class="root" style:padding-right="{$panelWidth}px">
  <header class="hdr">
    <div class="brand">Pantograph</div>
    <nav class="tabs" aria-label="Workspaces">
      {#each WORKBENCH_TAB_GROUPS as group, index (group)}
        {#if index > 0}<span class="vdiv" aria-hidden="true"></span>{/if}
        {#each workbenchPagesInGroup(group) as page (page.id)}
          <button
            type="button"
            class="tab"
            data-testid={`workbench-nav-${page.id}`}
            aria-current={$selectedWorkbenchPage === page.id ? 'page' : undefined}
            onclick={() => setWorkbenchPage(page.id)}
          >
            {page.label}
          </button>
        {/each}
      {/each}
    </nav>
    <div class="spacer"></div>

    <RunContextPicker />

    <div class="engine" class:pg-muted={!engineUnreachable} role="status">
      <span class="eng" class:off={engineUnreachable} aria-hidden="true"></span>
      <span class="engine-text" title={engineLineText(availability)}>
        <span class="engine-headline">{engineLine.headline}</span>{#if engineLine.detail}<span class="engine-detail">, {engineLine.detail}</span>{/if}
      </span>
      {#if engineUnreachable}
        <button
          type="button"
          class="pg-tgl retry"
          aria-disabled={retrying ? 'true' : undefined}
          onclick={() => {
            if (!retrying) void refreshRunContextRunList();
          }}
        >
          {retrying ? 'Retrying' : 'Retry'}
        </button>
      {/if}
    </div>

    <button
      type="button"
      class="tab"
      data-testid={`workbench-nav-${settingsPage.id}`}
      aria-current={$selectedWorkbenchPage === settingsPage.id ? 'page' : undefined}
      onclick={() => setWorkbenchPage(settingsPage.id)}
    >
      {settingsPage.label}
    </button>
    <button
      type="button"
      class="pg-tgl assistant"
      aria-pressed={$sidePanelOpen}
      title={$sidePanelOpen ? 'Close the Assistant panel' : 'Open the Assistant panel'}
      onclick={toggleSidePanel}
    >
      <MessageSquare size={13} aria-hidden="true" />Assistant
    </button>
  </header>

  <!-- Pages keep showing their last projection while the engine is unreachable; dimming says so. -->
  <div class="page" class:stale={engineUnreachable}>
    {#if $selectedWorkbenchPage === 'sessions'}
      <SessionsPage />
    {:else if $selectedWorkbenchPage === 'scheduler'}
      <SchedulerPage />
    {:else if $selectedWorkbenchPage === 'diagnostics'}
      <DiagnosticsPage />
    {:else if $selectedWorkbenchPage === 'io_inspector'}
      <IoInspectorPage />
    {:else if $selectedWorkbenchPage === 'network'}
      <NetworkPage />
    {:else if $selectedWorkbenchPage === 'graph'}
      <GraphPage />
    {:else if $selectedWorkbenchPage === 'node_lab'}
      <NodeLabPage />
    {:else if $selectedWorkbenchPage === 'library'}
      <LibraryPage />
    {:else if $selectedWorkbenchPage === 'settings'}
      <SettingsPage />
    {/if}
  </div>
</div>

<style>
  .root {
    height: 100vh;
    width: 100vw;
    display: flex;
    flex-direction: column;
    background: var(--pg-ground);
    color: var(--pg-text);
    overflow: hidden;
  }

  .hdr {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 40px;
    padding: 0 14px;
    border-bottom: 1px solid var(--pg-ink);
    background: var(--pg-panel-sunken);
    flex: none;
    min-width: 0;
  }

  .brand {
    font-family: var(--pg-font-serif);
    font-size: 18px;
    font-weight: 500;
    padding-right: 14px;
    letter-spacing: -0.01em;
    color: var(--pg-text-bright);
    flex: none;
  }

  .tabs {
    display: flex;
    align-items: center;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tab {
    display: flex;
    align-items: center;
    height: 40px;
    padding: 0 11px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--pg-text-quiet);
    font-size: 13.5px;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .tab:hover {
    color: var(--pg-text-bright);
    background: rgba(255, 255, 255, 0.04);
  }

  .tab[aria-current='page'] {
    color: var(--pg-text-bright);
    border-bottom-color: var(--pg-text);
  }

  .tab:focus-visible {
    outline-offset: -2px;
  }

  .vdiv {
    width: 1px;
    height: 18px;
    background: var(--pg-border);
    margin: 0 10px;
    flex: none;
  }

  .spacer {
    flex: 1;
    min-width: 8px;
  }

  /* The engine sentence gives way first when the header is narrow. */
  .engine {
    font-size: 12.5px;
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 10px 0 12px;
    white-space: nowrap;
    min-width: 0;
    flex: 0 100 auto;
  }

  .engine-text {
    display: flex;
    min-width: 0;
  }

  .engine-headline {
    flex: none;
  }

  .engine-detail {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .eng {
    display: inline-block;
    width: 7px;
    height: 7px;
    background: var(--pg-ok);
    border-radius: 1px;
    flex: none;
  }

  .eng.off {
    background: var(--pg-danger);
  }

  .retry {
    height: 22px;
  }

  .assistant {
    margin-left: 6px;
    flex: none;
  }

  .page {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .page.stale {
    opacity: 0.72;
  }
</style>
