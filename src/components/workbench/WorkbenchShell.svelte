<script lang="ts">
  import {
    Activity,
    Boxes,
    CalendarClock,
    FlaskConical,
    GitBranch,
    Library,
    Network,
    PanelTop,
    X,
  } from 'lucide-svelte';
  import {
    activeWorkflowRun,
    clearActiveWorkflowRun,
    selectedWorkbenchPage,
    setWorkbenchPage,
    WORKBENCH_PAGES,
    type WorkbenchPageId,
  } from '../../stores/workbenchStore';
  import DiagnosticsPage from './DiagnosticsPage.svelte';
  import GraphPage from './GraphPage.svelte';
  import IoInspectorPage from './IoInspectorPage.svelte';
  import LibraryPage from './LibraryPage.svelte';
  import NetworkPage from './NetworkPage.svelte';
  import NodeLabPage from './NodeLabPage.svelte';
  import SchedulerPage from './SchedulerPage.svelte';

  const pageIcons = {
    scheduler: CalendarClock,
    diagnostics: Activity,
    graph: GitBranch,
    io_inspector: PanelTop,
    library: Library,
    network: Network,
    node_lab: FlaskConical,
  } satisfies Record<WorkbenchPageId, typeof CalendarClock>;

  function activeRunLabel(): string {
    if (!$activeWorkflowRun) {
      return 'No run selected';
    }
    return $activeWorkflowRun.workflow_id ?? $activeWorkflowRun.workflow_run_id;
  }
</script>

<div class="flex h-screen w-screen overflow-hidden bg-neutral-950 text-neutral-100 selection:bg-cyan-500/30">
  <aside class="flex w-16 shrink-0 flex-col items-center border-r border-neutral-800 bg-neutral-950">
    <div class="flex h-14 w-full items-center justify-center border-b border-neutral-800">
      <Boxes size={20} aria-hidden="true" class="text-cyan-300" />
    </div>

    <nav class="flex flex-1 flex-col items-center gap-1 py-3" aria-label="Workbench pages">
      {#each WORKBENCH_PAGES as page (page.id)}
        {@const Icon = pageIcons[page.id]}
        <button
          type="button"
          class="group flex h-11 w-11 items-center justify-center rounded border text-neutral-400 transition-colors hover:border-neutral-600 hover:bg-neutral-900 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
          class:border-cyan-700={$selectedWorkbenchPage === page.id}
          class:bg-cyan-950={$selectedWorkbenchPage === page.id}
          class:text-cyan-100={$selectedWorkbenchPage === page.id}
          aria-label={page.label}
          aria-current={$selectedWorkbenchPage === page.id ? 'page' : undefined}
          title={page.label}
          onclick={() => setWorkbenchPage(page.id)}
        >
          <Icon size={17} aria-hidden="true" />
        </button>
      {/each}
    </nav>
  </aside>

  <div class="flex min-w-0 flex-1 flex-col">
    <header class="flex h-14 shrink-0 items-center justify-between border-b border-neutral-800 bg-neutral-950 px-4">
      <div class="min-w-0">
        <div class="text-[11px] uppercase tracking-[0.24em] text-neutral-500">Pantograph</div>
        <div class="truncate text-sm font-medium text-neutral-100">
          {WORKBENCH_PAGES.find((page) => page.id === $selectedWorkbenchPage)?.label ?? 'Scheduler'}
        </div>
      </div>

      <div class="flex min-w-0 items-center gap-3">
        <div class="hidden min-w-0 flex-col items-end md:flex">
          <div class="text-[11px] uppercase tracking-[0.2em] text-neutral-500">Active Run</div>
          <div class="max-w-[28rem] truncate text-sm text-neutral-200" title={activeRunLabel()}>
            {activeRunLabel()}
          </div>
        </div>
        {#if $activeWorkflowRun}
          <span class="hidden rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400 lg:inline-flex">
            {$activeWorkflowRun.status ?? 'unknown'}
          </span>
          <button
            type="button"
            class="flex h-8 w-8 items-center justify-center rounded border border-neutral-700 text-neutral-400 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
            aria-label="Clear active run"
            title="Clear active run"
            onclick={clearActiveWorkflowRun}
          >
            <X size={15} aria-hidden="true" />
          </button>
        {/if}
      </div>
    </header>

    <main class="min-h-0 flex-1 overflow-hidden">
      {#if $selectedWorkbenchPage === 'scheduler'}
        <SchedulerPage />
      {:else if $selectedWorkbenchPage === 'diagnostics'}
        <DiagnosticsPage />
      {:else if $selectedWorkbenchPage === 'graph'}
        <GraphPage />
      {:else if $selectedWorkbenchPage === 'io_inspector'}
        <IoInspectorPage />
      {:else if $selectedWorkbenchPage === 'library'}
        <LibraryPage />
      {:else if $selectedWorkbenchPage === 'network'}
        <NetworkPage />
      {:else if $selectedWorkbenchPage === 'node_lab'}
        <NodeLabPage />
      {/if}
    </main>
  </div>
</div>
