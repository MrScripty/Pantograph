import { writable } from 'svelte/store';

export interface PanOffset {
  x: number;
  y: number;
}

export const canvasPan = writable<PanOffset>({ x: 0, y: 0 });

export function setPan(x: number, y: number): void {
  canvasPan.set({ x, y });
}

export function adjustPan(dx: number, dy: number): void {
  canvasPan.update((pan) => ({ x: pan.x + dx, y: pan.y + dy }));
}

export function resetPan(): void {
  canvasPan.set({ x: 0, y: 0 });
}
