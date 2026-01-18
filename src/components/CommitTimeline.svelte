<script lang="ts">
  import { onMount } from 'svelte';
  import { panelWidth } from '../stores/panelStore';
  import { timelineStore, visibleCommits, type TimelineCommit } from '../stores/timelineStore';
  import { componentRegistry, importManager } from '../services/HotLoadRegistry';
  import { refreshGlobModules } from '$lib/hotload-sandbox/services/GlobRegistry';
  import { Logger } from '../services/Logger';

  let isExpanded = $state(false);
  let hoveredCommit: string | null = $state(null);
  let timelineState = $state($timelineStore);
  let commits = $state($visibleCommits);

  // Subscribe to stores
  onMount(() => {
    const unsubState = timelineStore.subscribe((s) => {
      timelineState = s;
    });
    const unsubCommits = visibleCommits.subscribe((c) => {
      commits = c;
    });

    // Load current commit on mount (lazy - just one commit)
    timelineStore.loadCurrentCommit();

    return () => {
      unsubState();
      unsubCommits();
    };
  });

  // Load full history on first interaction
  async function handleTimelineEnter() {
    if (!timelineState.isLoaded && !timelineState.isLoading) {
      await timelineStore.loadFullHistory();
    }
    isExpanded = true;
  }

  function handleTimelineLeave() {
    isExpanded = false;
    hoveredCommit = null;
  }

  // Refresh components after navigating to a different commit
  async function refreshAllComponents() {
    await new Promise((resolve) => setTimeout(resolve, 100));
    refreshGlobModules();
    importManager.clearAllCache();

    const components = componentRegistry.getAll();
    const paths = components.map((c) => c.path).filter((p) => p);
    if (paths.length > 0) {
      await componentRegistry.refreshByPaths(paths);
    }
  }

  async function handleCommitClick(commit: TimelineCommit, event: MouseEvent) {
    if (event.ctrlKey || event.metaKey) {
      // Hard delete with confirmation
      const confirmed = confirm(
        `Permanently delete commit "${commit.message}"?\n\nThis cannot be undone.`
      );
      if (confirmed) {
        const success = await timelineStore.hardDelete(commit.hash);
        if (success) {
          await refreshAllComponents();
          Logger.log('COMMIT_HARD_DELETED', { hash: commit.short_hash });
        }
      }
    } else {
      // Soft delete
      timelineStore.softDelete(commit.hash);
      Logger.log('COMMIT_SOFT_DELETED', { hash: commit.short_hash });
    }
  }

  async function handleCommitDoubleClick(commit: TimelineCommit) {
    // Navigate to this commit
    const success = await timelineStore.navigateToCommit(commit.hash);
    if (success) {
      await refreshAllComponents();
      Logger.log('COMMIT_CHECKOUT', { hash: commit.short_hash });
    }
  }

  // Get tooltip for hovered commit
  function getHoveredCommitData(): TimelineCommit | undefined {
    return commits.find((c) => c.hash === hoveredCommit);
  }
</script>

<!-- Only show if there are commits or we're loading -->
{#if timelineState.currentCommit || timelineState.isLoading || commits.length > 0}
  <div
    class="fixed left-1/2 z-40 transition-all duration-300 ease-out"
    style="
      bottom: 80px;
      transform: translateX(calc(-50% - {$panelWidth / 2}px));
    "
    onmouseenter={handleTimelineEnter}
    onmouseleave={handleTimelineLeave}
    role="navigation"
    aria-label="Commit timeline"
  >
    <!-- Timeline Container -->
    <div
      class="relative px-4 py-2 bg-neutral-900/70 backdrop-blur-sm border border-neutral-700/50 rounded-full transition-all duration-200"
      class:px-6={isExpanded}
      class:py-3={isExpanded}
    >
      <!-- Minimal collapsed view: just a line with current position indicator -->
      {#if !isExpanded}
        <div class="flex items-center gap-1 h-2">
          <div class="w-20 h-[2px] bg-neutral-600 rounded-full relative">
            {#if timelineState.currentCommit}
              <div
                class="absolute top-1/2 -translate-y-1/2 right-0 w-2 h-2 bg-blue-400 rounded-full shadow-sm shadow-blue-400/50"
                title={timelineState.currentCommit.message}
              ></div>
            {/if}
          </div>
          <span class="text-[10px] text-neutral-500 ml-1">
            {#if timelineState.currentCommit}
              {timelineState.currentCommit.short_hash}
            {:else}
              No commits
            {/if}
          </span>
        </div>
      {:else}
        <!-- Expanded view: show commit nodes -->
        <div class="flex items-center gap-3 min-w-[120px] max-w-[500px] relative">
          {#if timelineState.isLoading}
            <div class="text-xs text-neutral-400 px-2">Loading...</div>
          {:else if commits.length === 0}
            <div class="text-xs text-neutral-500 px-2">No commits</div>
          {:else}
            <!-- Timeline line (background) -->
            <div
              class="absolute left-3 right-3 top-1/2 h-[2px] bg-neutral-600 -translate-y-1/2 pointer-events-none"
            ></div>

            <!-- Commit nodes (reversed so oldest is on left, newest on right) -->
            {#each [...commits].reverse() as commit, index (commit.hash)}
              <button
                class="relative z-10 w-3 h-3 rounded-full transition-all duration-150 hover:scale-150 focus:outline-none focus:ring-2 focus:ring-blue-400/50 flex-shrink-0
                  {commit.is_current ? 'bg-blue-400 shadow-sm shadow-blue-400/50' : 'bg-neutral-400'}
                  {hoveredCommit === commit.hash ? 'scale-125' : ''}"
                onclick={(e) => handleCommitClick(commit, e)}
                ondblclick={() => handleCommitDoubleClick(commit)}
                onmouseenter={() => (hoveredCommit = commit.hash)}
                onmouseleave={() => (hoveredCommit = null)}
                title="{commit.short_hash}: {commit.message}"
              ></button>
            {/each}
          {/if}
        </div>

        <!-- Tooltip for hovered commit -->
        {#if hoveredCommit}
          {@const commitData = getHoveredCommitData()}
          {#if commitData}
            <div
              class="absolute bottom-full left-1/2 -translate-x-1/2 mb-3 px-3 py-2 bg-neutral-800 border border-neutral-600 rounded-lg shadow-lg whitespace-nowrap text-xs pointer-events-none"
            >
              <div class="font-mono text-neutral-400">{commitData.short_hash}</div>
              <div class="text-neutral-200 max-w-[200px] truncate">
                {commitData.message}
              </div>
              {#if commitData.timestamp}
                <div class="text-neutral-500 text-[10px]">{commitData.timestamp}</div>
              {/if}
              <div class="text-neutral-600 text-[10px] mt-1 border-t border-neutral-700 pt-1">
                Click: hide (Ctrl+Shift+Z to undo) | Ctrl+Click: delete | DblClick: checkout
              </div>
            </div>
          {/if}
        {/if}
      {/if}
    </div>
  </div>
{/if}
