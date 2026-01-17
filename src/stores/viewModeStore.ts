import { writable } from 'svelte/store';

export type ViewMode = 'canvas' | 'node-graph';

export const viewMode = writable<ViewMode>('canvas');

export function toggleViewMode() {
  viewMode.update((current) => (current === 'canvas' ? 'node-graph' : 'canvas'));
}
