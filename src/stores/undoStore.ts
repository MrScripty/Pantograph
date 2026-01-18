import { writable, get } from 'svelte/store';
import { Logger } from '../services/Logger';

// Action types that can be undone
export type UndoableAction =
  | { type: 'COMMIT_SOFT_DELETE'; hash: string }
  // Add more action types as needed

export interface UndoEntry {
  id: string;
  action: UndoableAction;
  timestamp: number;
}

interface UndoState {
  history: UndoEntry[];
  position: number; // Index of next action to undo (-1 means nothing to undo)
}

const MAX_HISTORY = 32;
const STORAGE_KEY = 'pantograph-unified-undo';

// Generate simple unique ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

// Load state from localStorage
function loadFromStorage(): UndoState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (parsed && Array.isArray(parsed.history)) {
        return {
          history: parsed.history,
          position: typeof parsed.position === 'number' ? parsed.position : parsed.history.length - 1,
        };
      }
    }
  } catch (e) {
    Logger.log('UNDO_STORE_LOAD_FAILED', { error: String(e) }, 'warn');
  }
  return { history: [], position: -1 };
}

// Save state to localStorage
function saveToStorage(state: UndoState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch (e) {
    Logger.log('UNDO_STORE_SAVE_FAILED', { error: String(e) }, 'warn');
  }
}

// Callback registry for handling aged-out actions and undo/redo
type AgedActionCallback = (action: UndoableAction) => Promise<void>;
type UndoCallback = (action: UndoableAction) => void;
type RedoCallback = (action: UndoableAction) => void;

let agedActionCallback: AgedActionCallback | null = null;
let undoCallbacks: Map<UndoableAction['type'], UndoCallback> = new Map();
let redoCallbacks: Map<UndoableAction['type'], RedoCallback> = new Map();

function createUndoStore() {
  const initial = loadFromStorage();
  const { subscribe, set, update } = writable<UndoState>(initial);

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
     * If history exceeds MAX_HISTORY, oldest action is permanently executed.
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
      if (newHistory.length > MAX_HISTORY) {
        const aged = newHistory.shift();
        if (aged && agedActionCallback) {
          try {
            await agedActionCallback(aged.action);
            Logger.log('UNDO_ACTION_AGED', { type: aged.action.type });
          } catch (e) {
            Logger.log('UNDO_AGED_ACTION_FAILED', { error: String(e) }, 'error');
          }
        }
      }

      const newState: UndoState = {
        history: newHistory,
        position: newHistory.length - 1,
      };

      set(newState);
      saveToStorage(newState);
      Logger.log('UNDO_PUSH', { type: action.type, historySize: newHistory.length });
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
      Logger.log('UNDO_EXECUTED', { type: entry.action.type, position: newState.position });
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
      Logger.log('REDO_EXECUTED', { type: entry.action.type, position: newState.position });
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
      Logger.log('UNDO_HISTORY_CLEARED', {});
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

export const undoStore = createUndoStore();
