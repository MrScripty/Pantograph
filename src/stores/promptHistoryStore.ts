import { writable } from 'svelte/store';
import { Logger } from '../services/Logger';

const STORAGE_KEY = 'pantograph-prompt-history';
const MAX_HISTORY_SIZE = 100;

function loadFromStorage(): string[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (Array.isArray(parsed) && parsed.every(p => typeof p === 'string')) {
        return parsed.slice(-MAX_HISTORY_SIZE);
      }
    }
  } catch (e) {
    Logger.log('prompt_history_load_failed', { error: String(e) }, 'warn');
  }
  return [];
}

function saveToStorage(history: string[]) {
  try {
    const trimmed = history.slice(-MAX_HISTORY_SIZE);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch (e) {
    Logger.log('prompt_history_save_failed', { error: String(e) }, 'warn');
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
