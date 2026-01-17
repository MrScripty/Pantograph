import { writable } from 'svelte/store';

export type SidePanelTab = 'settings' | 'history';

export const activeSidePanelTab = writable<SidePanelTab>('history');
