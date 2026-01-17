import { writable } from 'svelte/store';

const STORAGE_KEY = 'pantograph-prompt-history';
const MAX_HISTORY_SIZE = 100;

function loadFromStorage(): string[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (Array.isArray(parsed)) {
        return parsed.slice(-MAX_HISTORY_SIZE);
      }
    }
  } catch (e) {
    console.warn('[promptHistoryStore] Failed to load history from localStorage:', e);
  }
  return [];
}

function saveToStorage(history: string[]) {
  try {
    const trimmed = history.slice(-MAX_HISTORY_SIZE);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch (e) {
    console.warn('[promptHistoryStore] Failed to save history to localStorage:', e);
  }
}

function createPromptHistoryStore() {
  const initialHistory = loadFromStorage();
  const { subscribe, set, update } = writable<string[]>(initialHistory);

  return {
    subscribe,

    /**
     * Add a prompt to the history
     */
    addPrompt: (prompt: string) => {
      update(history => {
        // Avoid duplicate consecutive prompts
        if (history.length > 0 && history[history.length - 1] === prompt) {
          return history;
        }
        const newHistory = [...history, prompt].slice(-MAX_HISTORY_SIZE);
        saveToStorage(newHistory);
        return newHistory;
      });
    },

    /**
     * Clear all history
     */
    clear: () => {
      set([]);
      saveToStorage([]);
    },

    /**
     * Get the current history synchronously
     */
    getHistory: (): string[] => {
      return loadFromStorage();
    }
  };
}

export const promptHistoryStore = createPromptHistoryStore();
