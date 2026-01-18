import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Logger } from '../services/Logger';
import { undoStore } from './undoStore';

// Types matching the Rust TimelineCommit struct
export interface TimelineCommit {
  hash: string;
  short_hash: string;
  message: string;
  timestamp: string | null;
  is_current: boolean;
}

interface TimelineState {
  currentCommit: TimelineCommit | null;
  commits: TimelineCommit[];
  hiddenCommits: Set<string>; // Soft-deleted commit hashes (localStorage)
  isLoaded: boolean; // Has full history been fetched?
  isLoading: boolean;
  error: string | null;
}

const HIDDEN_COMMITS_KEY = 'pantograph-hidden-commits';

// Load hidden commits from localStorage
function loadHiddenCommits(): Set<string> {
  try {
    const stored = localStorage.getItem(HIDDEN_COMMITS_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (Array.isArray(parsed)) {
        return new Set(parsed);
      }
    }
  } catch (e) {
    Logger.log('hidden_commits_load_failed', { error: String(e) }, 'warn');
  }
  return new Set();
}

function saveHiddenCommits(hidden: Set<string>): void {
  try {
    localStorage.setItem(HIDDEN_COMMITS_KEY, JSON.stringify([...hidden]));
  } catch (e) {
    Logger.log('hidden_commits_save_failed', { error: String(e) }, 'warn');
  }
}

function createTimelineStore() {
  const initial: TimelineState = {
    currentCommit: null,
    commits: [],
    hiddenCommits: loadHiddenCommits(),
    isLoaded: false,
    isLoading: false,
    error: null,
  };

  const { subscribe, set, update } = writable<TimelineState>(initial);

  const store = {
    subscribe,

    // Called on app startup - only loads current commit (lazy loading)
    async loadCurrentCommit(): Promise<void> {
      update((s) => ({ ...s, isLoading: true, error: null }));
      try {
        const current = await invoke<TimelineCommit | null>('get_current_commit_info');
        update((s) => ({
          ...s,
          currentCommit: current,
          commits: current ? [current] : [],
          isLoading: false,
        }));
      } catch (e) {
        update((s) => ({
          ...s,
          isLoading: false,
          error: e instanceof Error ? e.message : String(e),
        }));
      }
    },

    // Called on first timeline interaction - loads full history
    async loadFullHistory(limit = 50): Promise<void> {
      const state = get({ subscribe });
      if (state.isLoaded || state.isLoading) return;

      update((s) => ({ ...s, isLoading: true }));
      try {
        const commits = await invoke<TimelineCommit[]>('get_timeline_commits', { limit });
        update((s) => ({
          ...s,
          commits,
          currentCommit: commits.find((c) => c.is_current) ?? s.currentCommit,
          isLoaded: true,
          isLoading: false,
        }));
      } catch (e) {
        update((s) => ({
          ...s,
          isLoading: false,
          error: e instanceof Error ? e.message : String(e),
        }));
      }
    },

    // Soft delete - just hide from view (stored in localStorage)
    // Also pushes to unified undo history
    softDelete(hash: string): void {
      update((s) => {
        const newHidden = new Set(s.hiddenCommits);
        newHidden.add(hash);
        saveHiddenCommits(newHidden);
        return { ...s, hiddenCommits: newHidden };
      });

      // Register with unified undo system
      undoStore.push({ type: 'COMMIT_SOFT_DELETE', hash });
    },

    // Undo soft delete (unhide a commit)
    unhide(hash: string): void {
      update((s) => {
        const newHidden = new Set(s.hiddenCommits);
        newHidden.delete(hash);
        saveHiddenCommits(newHidden);
        return { ...s, hiddenCommits: newHidden };
      });
    },

    // Internal: hide without pushing to undo (used by redo)
    _hideWithoutUndo(hash: string): void {
      update((s) => {
        const newHidden = new Set(s.hiddenCommits);
        newHidden.add(hash);
        saveHiddenCommits(newHidden);
        return { ...s, hiddenCommits: newHidden };
      });
    },

    // Hard delete - permanent removal from git history
    async hardDelete(hash: string): Promise<boolean> {
      try {
        const result = await invoke<{ success: boolean; message: string }>(
          'hard_delete_commit',
          { hash }
        );
        if (result.success) {
          // Remove from local state
          update((s) => {
            const newHidden = new Set(s.hiddenCommits);
            newHidden.delete(hash); // Remove from hidden if it was there
            saveHiddenCommits(newHidden);
            return {
              ...s,
              commits: s.commits.filter((c) => c.hash !== hash),
              hiddenCommits: newHidden,
            };
          });
          // Reload to get accurate state
          await store.forceReload();
          return true;
        }
        Logger.log('hard_delete_failed', { hash, message: result.message }, 'warn');
        return false;
      } catch (e) {
        Logger.log('hard_delete_error', { hash, error: String(e) }, 'error');
        return false;
      }
    },

    // Navigate to a specific commit
    async navigateToCommit(hash: string): Promise<boolean> {
      try {
        const result = await invoke<{ success: boolean; message: string }>(
          'checkout_commit',
          { hash }
        );
        if (result.success) {
          // Update current commit marker
          update((s) => ({
            ...s,
            commits: s.commits.map((c) => ({
              ...c,
              is_current: c.hash === hash,
            })),
            currentCommit: s.commits.find((c) => c.hash === hash) ?? s.currentCommit,
          }));
          return true;
        }
        return false;
      } catch (e) {
        Logger.log('checkout_commit_error', { hash, error: String(e) }, 'error');
        return false;
      }
    },

    // Force reload full history (used after hard delete or undo/redo)
    async forceReload(): Promise<void> {
      update((s) => ({ ...s, isLoaded: false }));
      await store.loadFullHistory();
    },

    // Refresh after undo/redo operations
    async refresh(): Promise<void> {
      const state = get({ subscribe });
      if (state.isLoaded) {
        await store.forceReload();
      } else {
        await store.loadCurrentCommit();
      }
    },
  };

  return store;
}

export const timelineStore = createTimelineStore();

// Register undo/redo handlers for soft delete
undoStore.onUndo('COMMIT_SOFT_DELETE', (action) => {
  if (action.type === 'COMMIT_SOFT_DELETE') {
    timelineStore.unhide(action.hash);
  }
});

undoStore.onRedo('COMMIT_SOFT_DELETE', (action) => {
  if (action.type === 'COMMIT_SOFT_DELETE') {
    // Re-hide the commit without pushing to undo again
    timelineStore._hideWithoutUndo(action.hash);
  }
});

// Register callback for when soft deletes age past 32 steps
undoStore.onAgedAction(async (action) => {
  if (action.type === 'COMMIT_SOFT_DELETE') {
    // Permanently delete from git
    await timelineStore.hardDelete(action.hash);
  }
});

// Derived store for visible commits (filters out soft-deleted)
export const visibleCommits = derived(timelineStore, ($state) =>
  $state.commits.filter((c) => !$state.hiddenCommits.has(c.hash))
);
