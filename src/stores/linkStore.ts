/**
 * Link Store - Manages linking between workflow input nodes and UI elements
 *
 * This store enables LinkedInput nodes to bind to UI elements outside the graph,
 * like the FollowUpInput chat input or generated Svelte components.
 */

import { writable, get } from 'svelte/store';

// ============================================================================
// Types
// ============================================================================

export type LinkableElementType = 'input' | 'button' | 'checkbox' | 'generated';

export interface LinkableElement {
  /** Unique identifier for this element */
  id: string;
  /** Human-readable label for display in nodes */
  label: string;
  /** Type of element for icon/behavior selection */
  type: LinkableElementType;
  /** Function to get current value from element */
  getValue: () => string | boolean;
  /** Optional reference to DOM element */
  element?: HTMLElement;
  /** Optional event subscription for direct value updates */
  subscribe?: (onChange: (value: string | boolean) => void) => (() => void) | void;
}

export type LinkStatus = 'linked' | 'unlinked' | 'error';

export interface LinkMapping {
  /** ID of the LinkedInput node */
  nodeId: string;
  /** ID of the linked element */
  elementId: string;
  /** Label of the linked element (for display when element is missing) */
  elementLabel: string;
  /** Current link status */
  status: LinkStatus;
  /** Error message if status is 'error' */
  errorMessage?: string;
  /** Last known value from the linked element */
  lastValue?: string;
}

// ============================================================================
// State
// ============================================================================

/** Whether link mode is currently active (user is selecting an element to link) */
export const linkModeActive = writable<boolean>(false);

/** ID of the node that initiated link mode */
export const linkingNodeId = writable<string | null>(null);

/** Registry of all linkable UI elements (elementId -> element) */
export const linkableElements = writable<Map<string, LinkableElement>>(new Map());

/** Active link mappings (nodeId -> mapping) */
export const linkMappings = writable<Map<string, LinkMapping>>(new Map());

const elementChangeUnsubscribers = new Map<string, () => void>();

// ============================================================================
// Internal Helpers
// ============================================================================

function readElementValue(element: LinkableElement): string | null {
  try {
    return String(element.getValue());
  } catch {
    return null;
  }
}

function updateMappingsForElementValue(elementId: string, value: string): void {
  linkMappings.update((map) => {
    let changed = false;
    const newMap = new Map(map);

    for (const [nodeId, mapping] of map) {
      if (mapping.elementId !== elementId) {
        continue;
      }
      if (mapping.lastValue === value && mapping.errorMessage === undefined) {
        continue;
      }

      newMap.set(nodeId, {
        ...mapping,
        status: 'linked',
        errorMessage: undefined,
        lastValue: value,
      });
      changed = true;
    }

    return changed ? newMap : map;
  });
}

function markMappingsReadError(elementId: string, message: string): void {
  linkMappings.update((map) => {
    let changed = false;
    const newMap = new Map(map);

    for (const [nodeId, mapping] of map) {
      if (mapping.status !== 'linked' || mapping.elementId !== elementId) {
        continue;
      }
      if (mapping.errorMessage === message) {
        continue;
      }

      newMap.set(nodeId, {
        ...mapping,
        status: 'error',
        errorMessage: message,
      });
      changed = true;
    }

    return changed ? newMap : map;
  });
}

function clearElementSubscription(elementId: string): void {
  const unsubscribe = elementChangeUnsubscribers.get(elementId);
  if (unsubscribe) {
    unsubscribe();
    elementChangeUnsubscribers.delete(elementId);
  }
}

function attachElementSubscription(element: LinkableElement): void {
  clearElementSubscription(element.id);

  if (element.subscribe) {
    const maybeUnsubscribe = element.subscribe((value) => {
      notifyLinkableValueChanged(element.id, value);
    });

    if (typeof maybeUnsubscribe === 'function') {
      elementChangeUnsubscribers.set(element.id, maybeUnsubscribe);
    }
    return;
  }

  if (element.element) {
    const listener = () => {
      const nextValue = readElementValue(element);
      if (nextValue === null) {
        markMappingsReadError(element.id, 'Failed to read value');
        return;
      }
      updateMappingsForElementValue(element.id, nextValue);
    };

    element.element.addEventListener('input', listener);
    element.element.addEventListener('change', listener);

    elementChangeUnsubscribers.set(element.id, () => {
      element.element?.removeEventListener('input', listener);
      element.element?.removeEventListener('change', listener);
    });
  }
}

function syncMappingsForElement(element: LinkableElement): void {
  const currentValue = readElementValue(element);
  if (currentValue === null) {
    markMappingsReadError(element.id, 'Failed to read value');
    return;
  }
  updateMappingsForElementValue(element.id, currentValue);
}

// ============================================================================
// Value Sync
// ============================================================================

/**
 * Notify the store that a linkable element's value changed.
 */
export function notifyLinkableValueChanged(elementId: string, value: string | boolean): void {
  updateMappingsForElementValue(elementId, String(value));
}

// ============================================================================
// Element Registration
// ============================================================================

/**
 * Register a UI element as linkable.
 *
 * @param element - The element to register
 * @returns Unregister function (call on unmount)
 *
 * @example
 * ```svelte
 * onMount(() => {
 *   const unregister = registerLinkable({
 *     id: 'follow-up-input',
 *     label: 'Follow-Up Input',
 *     type: 'input',
 *     getValue: () => inputValue
 *   });
 *   return unregister;
 * });
 * ```
 */
export function registerLinkable(element: LinkableElement): () => void {
  linkableElements.update((map) => {
    const newMap = new Map(map);
    newMap.set(element.id, element);
    return newMap;
  });

  attachElementSubscription(element);
  syncMappingsForElement(element);

  // Return unregister function
  return () => {
    clearElementSubscription(element.id);

    linkableElements.update((map) => {
      const newMap = new Map(map);
      newMap.delete(element.id);
      return newMap;
    });

    // NOTE: Don't mark mappings as error here - the element may just be temporarily
    // unmounted (e.g., view switch between canvas and workflow). Links should persist
    // and continue working when the element re-mounts.
  };
}

// ============================================================================
// Link Mode Actions
// ============================================================================

/**
 * Enter link mode for a specific node.
 * The UI should display an overlay highlighting linkable elements.
 */
export function startLinkMode(nodeId: string): void {
  linkModeActive.set(true);
  linkingNodeId.set(nodeId);
}

/**
 * Cancel link mode without creating a link.
 */
export function cancelLinkMode(): void {
  linkModeActive.set(false);
  linkingNodeId.set(null);
}

/**
 * Create a link between the current linking node and the specified element.
 */
export function createLink(elementId: string): void {
  const nodeId = get(linkingNodeId);

  if (!nodeId) {
    return;
  }

  const elements = get(linkableElements);
  const element = elements.get(elementId);

  if (!element) {
    cancelLinkMode();
    return;
  }

  const currentValue = readElementValue(element);

  linkMappings.update((map) => {
    const newMap = new Map(map);
    newMap.set(nodeId, {
      nodeId,
      elementId,
      elementLabel: element.label,
      status: currentValue === null ? 'error' : 'linked',
      errorMessage: currentValue === null ? 'Failed to read value' : undefined,
      lastValue: currentValue ?? undefined,
    });
    return newMap;
  });

  cancelLinkMode();
}

/**
 * Remove the link for a specific node.
 */
export function unlinkNode(nodeId: string): void {
  linkMappings.update((map) => {
    const newMap = new Map(map);
    newMap.delete(nodeId);
    return newMap;
  });
}

/**
 * Clear error state and remove mapping for a node.
 */
export function clearNodeLink(nodeId: string): void {
  unlinkNode(nodeId);
}

// ============================================================================
// Value Access
// ============================================================================

/**
 * Get the current linked value for a node.
 *
 * @returns The current value, or undefined if not linked or error
 */
export function getLinkedValue(nodeId: string): string | undefined {
  const mappings = get(linkMappings);
  const mapping = mappings.get(nodeId);

  if (!mapping || mapping.status !== 'linked') {
    return undefined;
  }

  const elements = get(linkableElements);
  const element = elements.get(mapping.elementId);

  if (!element) {
    return undefined;
  }

  const currentValue = readElementValue(element);
  if (currentValue === null) {
    markMappingsReadError(mapping.elementId, 'Failed to read value');
    return undefined;
  }

  if (currentValue !== mapping.lastValue) {
    updateMappingsForElementValue(mapping.elementId, currentValue);
  }

  return currentValue;
}

/**
 * Get all current linked values for workflow execution.
 *
 * @returns Map of nodeId -> currentValue for all valid links
 */
export function getAllLinkedValues(): Map<string, string> {
  const result = new Map<string, string>();
  const mappings = get(linkMappings);
  const elements = get(linkableElements);

  const latestByElementId = new Map<string, string>();
  const failedElementIds = new Set<string>();

  for (const [nodeId, mapping] of mappings) {
    if (mapping.status !== 'linked') continue;

    const element = elements.get(mapping.elementId);
    if (!element) continue;

    const currentValue = readElementValue(element);
    if (currentValue === null) {
      failedElementIds.add(mapping.elementId);
      continue;
    }

    latestByElementId.set(mapping.elementId, currentValue);
    result.set(nodeId, currentValue);
  }

  for (const [elementId, value] of latestByElementId) {
    updateMappingsForElementValue(elementId, value);
  }

  for (const elementId of failedElementIds) {
    markMappingsReadError(elementId, 'Failed to read value');
  }

  return result;
}

// ============================================================================
// Persistence Helpers
// ============================================================================

/**
 * Export link mappings for saving with workflow.
 */
export function exportLinkMappings(): LinkMapping[] {
  const mappings = get(linkMappings);
  return Array.from(mappings.values());
}

/**
 * Import link mappings when loading a workflow.
 * Validates against currently registered elements.
 */
export function importLinkMappings(mappings: LinkMapping[]): void {
  const elements = get(linkableElements);
  const newMappings = new Map<string, LinkMapping>();

  for (const mapping of mappings) {
    const element = elements.get(mapping.elementId);

    if (element) {
      // Element exists - restore as linked
      const currentValue = readElementValue(element);
      newMappings.set(mapping.nodeId, {
        ...mapping,
        status: currentValue === null ? 'error' : 'linked',
        errorMessage: currentValue === null ? 'Failed to read value' : undefined,
        lastValue: currentValue ?? mapping.lastValue,
      });
    } else {
      // Element doesn't exist - restore as error
      newMappings.set(mapping.nodeId, {
        ...mapping,
        status: 'error',
        errorMessage: 'Element not found',
      });
    }
  }

  linkMappings.set(newMappings);
}
