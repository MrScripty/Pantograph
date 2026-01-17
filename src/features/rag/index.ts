/**
 * RAG Feature Module
 *
 * Retrieval-Augmented Generation for Svelte documentation.
 */

// Services
export { RagService } from '../../services/RagService';

// Components
export { default as RagStatus } from '../../components/RagStatus.svelte';
export { default as ChunkPreview } from '../../components/ChunkPreview.svelte';

// Stores
export {
  chunkPreviewState,
  openChunkPreview,
  closeChunkPreview,
  setDocs,
  setLoading,
  setPreview,
  setError,
} from '../../stores/chunkPreviewStore';
export type {
  ChunkPreviewItem,
  ChunkPreview as ChunkPreviewType,
  DocInfo,
  ChunkPreviewState,
} from '../../stores/chunkPreviewStore';
