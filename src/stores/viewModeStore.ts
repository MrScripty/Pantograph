import { writable } from 'svelte/store';

export type ViewMode = 'canvas' | 'node-graph' | 'workflow';

export const viewMode = writable<ViewMode>('canvas');

export function toggleViewMode() {
  viewMode.update((current) => {
    switch (current) {
      case 'canvas':
        return 'node-graph';
      case 'node-graph':
        return 'workflow';
      case 'workflow':
        return 'canvas';
      default:
        return 'canvas';
    }
  });
}

export function setViewMode(mode: ViewMode) {
  viewMode.set(mode);
}
