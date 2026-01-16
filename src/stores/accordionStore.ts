import { writable } from 'svelte/store';

// Track which section is currently expanded in the side panel
// Only one section can be expanded at a time
export type AccordionSection = 'server' | 'model' | 'device' | 'rag' | 'sandbox' | null;

export const expandedSection = writable<AccordionSection>(null);

export const toggleSection = (section: AccordionSection) => {
  expandedSection.update((current) => (current === section ? null : section));
};
