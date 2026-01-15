import { writable } from 'svelte/store';

export interface ChunkPreviewItem {
  chunk_index: number;
  title: string;
  header_context: string;
  content_preview: string;
  full_content: string;
  char_count: number;
  has_code: boolean;
  start_line: number;
  end_line: number;
}

export interface ChunkPreview {
  doc_id: string;
  doc_title: string;
  total_chunks: number;
  chunks: ChunkPreviewItem[];
}

export interface DocInfo {
  id: string;
  title: string;
  section: string;
  char_count: number;
}

export interface ChunkPreviewState {
  open: boolean;
  loading: boolean;
  docId: string | null;
  preview: ChunkPreview | null;
  docs: DocInfo[];
  error: string | null;
}

const initialState: ChunkPreviewState = {
  open: false,
  loading: false,
  docId: null,
  preview: null,
  docs: [],
  error: null,
};

export const chunkPreviewState = writable<ChunkPreviewState>(initialState);

export function openChunkPreview() {
  chunkPreviewState.update((state) => ({
    ...state,
    open: true,
    error: null,
  }));
}

export function closeChunkPreview() {
  chunkPreviewState.update((state) => ({
    ...state,
    open: false,
    loading: false,
    docId: null,
    preview: null,
    error: null,
  }));
}

export function setDocs(docs: DocInfo[]) {
  chunkPreviewState.update((state) => ({
    ...state,
    docs,
  }));
}

export function setLoading(loading: boolean) {
  chunkPreviewState.update((state) => ({
    ...state,
    loading,
  }));
}

export function setPreview(docId: string, preview: ChunkPreview) {
  chunkPreviewState.update((state) => ({
    ...state,
    docId,
    preview,
    loading: false,
    error: null,
  }));
}

export function setError(error: string) {
  chunkPreviewState.update((state) => ({
    ...state,
    loading: false,
    error,
  }));
}
