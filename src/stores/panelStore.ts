import { writable } from 'svelte/store';

// Panel width: 20px handle + 320px content = 340px when open, 20px when closed (just the handle)
const HANDLE_WIDTH = 20;
const PANEL_CONTENT_WIDTH = 320;

export const sidePanelOpen = writable(false);

export const panelWidth = writable(HANDLE_WIDTH);

export function toggleSidePanel() {
  sidePanelOpen.update((open) => {
    const newOpen = !open;
    panelWidth.set(newOpen ? HANDLE_WIDTH + PANEL_CONTENT_WIDTH : HANDLE_WIDTH);
    return newOpen;
  });
}

export function openSidePanel() {
  sidePanelOpen.set(true);
  panelWidth.set(HANDLE_WIDTH + PANEL_CONTENT_WIDTH);
}

export function closeSidePanel() {
  sidePanelOpen.set(false);
  panelWidth.set(HANDLE_WIDTH);
}
