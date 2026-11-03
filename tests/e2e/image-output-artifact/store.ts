import { writable } from 'svelte/store';

export const nodeData = writable<{ image?: unknown; label?: string }>({ label: 'Image artifact fixture' });
export const nodeExecutionStates = writable(new Map());
export async function updateNodeData(_id: string, patch: Record<string, unknown>): Promise<void> {
  nodeData.update((data) => ({ ...data, ...patch }));
}
