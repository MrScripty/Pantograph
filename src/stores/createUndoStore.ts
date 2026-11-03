import { writable, get } from 'svelte/store';
import { decodeUndoState, MAX_UNDO_HISTORY, UNDO_STORAGE_KEY } from './undoPersistence.ts';
import type { UndoableAction, UndoEntry, UndoState } from './undoPersistence.ts';

interface UndoStoreOptions {
  storage: () => Pick<Storage, 'getItem' | 'setItem'>;
  log: (type: string, payload: Record<string, unknown>, severity?: 'info' | 'warn' | 'error') => void;
}

// Generate simple unique ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

// Callback registry for handling aged-out actions and undo/redo
type AgedActionCallback = (action: UndoableAction) => Promise<void>;
type UndoCallback = (action: UndoableAction) => void;
type RedoCallback = (action: UndoableAction) => void;

export function createUndoStore({ storage, log }: UndoStoreOptions) {
  // Rejected/unavailable bytes are never replaced by fallback or later edits.
  let canPersist = false;
  let initial: UndoState = { history: [], position: -1 };
  try {
    const stored = storage().getItem(UNDO_STORAGE_KEY);
    if (stored === null) {
      canPersist = true;
    } else {
      const decoded = decodeUndoState(JSON.parse(stored));
      if (decoded.ok) {
        initial = decoded.state;
        canPersist = true;
      } else {
        log('UNDO_STORE_LOAD_FAILED', { error: decoded.reason }, 'warn');
      }
    }
  } catch (error) {
    log('UNDO_STORE_LOAD_FAILED', { error: String(error) }, 'warn');
  }

  function saveToStorage(state: UndoState): void {
    if (!canPersist) {
      log('UNDO_STORE_SAVE_FAILED', { error: 'Undo storage was rejected or unavailable; existing bytes preserved.' }, 'warn');
      return;
    }
    try {
      storage().setItem(UNDO_STORAGE_KEY, JSON.stringify(state));
    } catch (error) {
      log('UNDO_STORE_SAVE_FAILED', { error: String(error) }, 'warn');
    }
  }

  let agedActionCallback: AgedActionCallback | null = null;
  const undoCallbacks = new Map<UndoableAction['type'], UndoCallback>();
  const redoCallbacks = new Map<UndoableAction['type'], RedoCallback>();
  const { subscribe, set } = writable<UndoState>(initial);

  const store = {
    subscribe,

    /**
     * Register callback for when actions age past 32 steps
     */
    onAgedAction(callback: AgedActionCallback): void {
      agedActionCallback = callback;
    },

    /**
     * Register undo handler for specific action type
     */
    onUndo(type: UndoableAction['type'], callback: UndoCallback): void {
      undoCallbacks.set(type, callback);
    },

    /**
     * Register redo handler for specific action type
     */
    onRedo(type: UndoableAction['type'], callback: RedoCallback): void {
      redoCallbacks.set(type, callback);
    },

    /**
     * Push a new action to the undo history.
     * If history exceeds MAX_UNDO_HISTORY, oldest action is permanently executed.
     */
    async push(action: UndoableAction): Promise<void> {
      const state = get({ subscribe });

      // If we're not at the end of history, truncate future actions
      // (user did something new after undoing)
      const newHistory = state.history.slice(0, state.position + 1);

      // Add new action
      const entry: UndoEntry = {
        id: generateId(),
        action,
        timestamp: Date.now(),
      };
      newHistory.push(entry);

      // Check if we need to age out the oldest action
      if (newHistory.length > MAX_UNDO_HISTORY) {
        const aged = newHistory.shift();
        if (aged && agedActionCallback) {
          try {
            await agedActionCallback(aged.action);
            log('UNDO_ACTION_AGED', { type: aged.action.type });
          } catch (e) {
            log('UNDO_AGED_ACTION_FAILED', { error: String(e) }, 'error');
          }
        }
      }

      const newState: UndoState = {
        history: newHistory,
        position: newHistory.length - 1,
      };

      set(newState);
      saveToStorage(newState);
      log('UNDO_PUSH', { type: action.type, historySize: newHistory.length });
    },

    /**
     * Undo the most recent action
     */
    undo(): boolean {
      const state = get({ subscribe });

      if (state.position < 0 || state.history.length === 0) {
        return false;
      }

      const entry = state.history[state.position];
      if (!entry) return false;

      // Call undo handler
      const handler = undoCallbacks.get(entry.action.type);
      if (handler) {
        handler(entry.action);
      }

      const newState: UndoState = {
        ...state,
        position: state.position - 1,
      };

      set(newState);
      saveToStorage(newState);
      log('UNDO_EXECUTED', { type: entry.action.type, position: newState.position });
      return true;
    },

    /**
     * Redo a previously undone action
     */
    redo(): boolean {
      const state = get({ subscribe });

      if (state.position >= state.history.length - 1) {
        return false;
      }

      const nextPosition = state.position + 1;
      const entry = state.history[nextPosition];
      if (!entry) return false;

      // Call redo handler
      const handler = redoCallbacks.get(entry.action.type);
      if (handler) {
        handler(entry.action);
      }

      const newState: UndoState = {
        ...state,
        position: nextPosition,
      };

      set(newState);
      saveToStorage(newState);
      log('REDO_EXECUTED', { type: entry.action.type, position: newState.position });
      return true;
    },

    /**
     * Check if undo is available
     */
    canUndo(): boolean {
      const state = get({ subscribe });
      return state.position >= 0 && state.history.length > 0;
    },

    /**
     * Check if redo is available
     */
    canRedo(): boolean {
      const state = get({ subscribe });
      return state.position < state.history.length - 1;
    },

    /**
     * Clear all history
     */
    clear(): void {
      const newState: UndoState = { history: [], position: -1 };
      set(newState);
      saveToStorage(newState);
      log('UNDO_HISTORY_CLEARED', {});
    },

    /**
     * Get current state for debugging
     */
    getState(): UndoState {
      return get({ subscribe });
    },
  };

  return store;
}
