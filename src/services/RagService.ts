import { invoke, Channel } from '@tauri-apps/api/core';
import { Logger } from './Logger';

export interface RagStatus {
  docs_available: boolean;
  docs_count: number;
  vectorizer_available: boolean;
  vectorizer_url: string | null;
  vectors_indexed: boolean;
  vectors_count: number;
  indexing_progress: IndexingProgress | null;
}

export interface IndexingProgress {
  current: number;
  total: number;
  status: string;
}

export interface IndexingEvent {
  current: number;
  total: number;
  status: string;
  done: boolean;
  error: string | null;
}

export interface SvelteDoc {
  id: string;
  title: string;
  section: string;
  summary: string;
  content: string;
}

export interface RagState {
  status: RagStatus;
  isIndexing: boolean;
  indexingProgress: IndexingProgress | null;
  error: string | null;
}

const defaultStatus: RagStatus = {
  docs_available: false,
  docs_count: 0,
  vectorizer_available: false,
  vectorizer_url: null,
  vectors_indexed: false,
  vectors_count: 0,
  indexing_progress: null,
};

class RagServiceClass {
  private state: RagState = {
    status: { ...defaultStatus },
    isIndexing: false,
    indexingProgress: null,
    error: null,
  };

  private listeners: Array<(state: RagState) => void> = [];

  public subscribe(callback: (state: RagState) => void): () => void {
    this.listeners.push(callback);
    callback({ ...this.state });
    return () => {
      this.listeners = this.listeners.filter((l) => l !== callback);
    };
  }

  private notify(): void {
    const stateCopy = { ...this.state };
    this.listeners.forEach((l) => l(stateCopy));
  }

  public getState(): RagState {
    return { ...this.state };
  }

  /**
   * Fetch the current RAG status from the backend
   */
  public async refreshStatus(): Promise<RagStatus> {
    try {
      const status = await invoke<RagStatus>('get_rag_status');
      this.state.status = status;
      this.state.error = null;
      this.notify();
      return status;
    } catch (error) {
      this.state.error = String(error);
      Logger.log('RAG_STATUS_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * Check if an embedding server is available at the given URL
   */
  public async checkEmbeddingServer(url: string): Promise<boolean> {
    try {
      const available = await invoke<boolean>('check_embedding_server', { url });
      Logger.log('RAG_CHECK_SERVER', { url, available });
      return available;
    } catch (error) {
      Logger.log('RAG_CHECK_SERVER_ERROR', { url, error: String(error) }, 'error');
      return false;
    }
  }

  /**
   * Set the embedding server URL and check if it's available
   */
  public async setEmbeddingServerUrl(url: string): Promise<boolean> {
    try {
      const available = await invoke<boolean>('set_embedding_server_url', { url });
      this.state.status.vectorizer_url = url;
      this.state.status.vectorizer_available = available;
      this.state.error = null;
      Logger.log('RAG_SET_SERVER', { url, available });
      this.notify();
      return available;
    } catch (error) {
      this.state.error = String(error);
      Logger.log('RAG_SET_SERVER_ERROR', { url, error: String(error) }, 'error');
      this.notify();
      return false;
    }
  }

  /**
   * Index all documentation for RAG
   */
  public async indexDocuments(): Promise<void> {
    if (this.state.isIndexing) {
      throw new Error('Already indexing');
    }

    this.state.isIndexing = true;
    this.state.indexingProgress = null;
    this.state.error = null;
    this.notify();

    Logger.log('RAG_INDEX_START', {});

    try {
      const channel = new Channel<IndexingEvent>();

      channel.onmessage = (event: IndexingEvent) => {
        if (event.error) {
          this.state.error = event.error;
          this.state.isIndexing = false;
          Logger.log('RAG_INDEX_ERROR', { error: event.error }, 'error');
          this.notify();
          return;
        }

        this.state.indexingProgress = {
          current: event.current,
          total: event.total,
          status: event.status,
        };
        this.notify();

        if (event.done) {
          this.state.isIndexing = false;
          this.state.status.vectors_indexed = true;
          this.state.status.vectors_count = event.total;
          Logger.log('RAG_INDEX_COMPLETE', { count: event.total });
          this.notify();
        }
      };

      await invoke('index_rag_documents', { channel });
    } catch (error) {
      this.state.isIndexing = false;
      this.state.error = String(error);
      Logger.log('RAG_INDEX_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * Try to load existing RAG index from disk
   */
  public async loadFromDisk(): Promise<boolean> {
    try {
      const loaded = await invoke<boolean>('load_rag_from_disk');
      if (loaded) {
        await this.refreshStatus();
        Logger.log('RAG_LOAD_DISK', { success: true });
      }
      return loaded;
    } catch (error) {
      Logger.log('RAG_LOAD_DISK_ERROR', { error: String(error) }, 'error');
      return false;
    }
  }

  /**
   * Clear the RAG cache
   */
  public async clearCache(): Promise<void> {
    try {
      await invoke('clear_rag_cache');
      this.state.status.vectors_indexed = false;
      this.state.status.vectors_count = 0;
      this.state.error = null;
      Logger.log('RAG_CACHE_CLEARED', {});
      this.notify();
    } catch (error) {
      this.state.error = String(error);
      Logger.log('RAG_CACHE_CLEAR_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * Search the RAG index
   */
  public async search(query: string, limit: number = 3): Promise<SvelteDoc[]> {
    try {
      const results = await invoke<SvelteDoc[]>('search_rag', { query, limit });
      Logger.log('RAG_SEARCH', { query, resultCount: results.length });
      return results;
    } catch (error) {
      Logger.log('RAG_SEARCH_ERROR', { query, error: String(error) }, 'error');
      throw error;
    }
  }

  /**
   * Check if RAG search is available
   */
  public get isSearchAvailable(): boolean {
    return (
      this.state.status.vectorizer_available && this.state.status.vectors_indexed
    );
  }
}

export const RagService = new RagServiceClass();
