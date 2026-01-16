import { writable } from 'svelte/store';

export type InteractionMode = 'draw' | 'interact';

export const interactionMode = writable<InteractionMode>('draw');

export function toggleInteractionMode() {
  interactionMode.update((mode) => (mode === 'draw' ? 'interact' : 'draw'));
}

export function setDrawMode() {
  interactionMode.set('draw');
}

export function setInteractMode() {
  interactionMode.set('interact');
}
