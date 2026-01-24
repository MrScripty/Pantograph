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

// ============================================================================
// Value Sync
// ============================================================================

let syncInterval: ReturnType<typeof setInterval> | null = null;

/**
 * Start polling for value changes in linked elements.
 * Call this when the app mounts.
 */
export function startValueSync(): void {
  if (syncInterval) return;

  syncInterval = setInterval(() => {
    const mappings = get(linkMappings);
    const elements = get(linkableElements);

    let hasChanges = false;
    const newMappings = new Map(mappings);

    for (const [nodeId, mapping] of newMappings) {
      if (mapping.status !== 'linked') continue;

      const element = elements.get(mapping.elementId);
      if (!element) {
        // Element is temporarily unmounted (e.g., view switch) - keep the link
        // and its last value. Don't mark as error since it may re-mount.
        continue;
      }

      try {
        const currentValue = String(element.getValue());
        if (currentValue !== mapping.lastValue) {
          newMappings.set(nodeId, {
            ...mapping,
            lastValue: currentValue,
          });
          hasChanges = true;
        }
      } catch (e) {
        newMappings.set(nodeId, {
          ...mapping,
          status: 'error',
          errorMessage: 'Failed to read value',
        });
        hasChanges = true;
      }
    }

    if (hasChanges) {
      linkMappings.set(newMappings);
    }
  }, 100); // Poll every 100ms
}

/**
 * Stop the value sync polling.
 * Call this when the app unmounts.
 */
export function stopValueSync(): void {
  if (syncInterval) {
    clearInterval(syncInterval);
    syncInterval = null;
  }
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

  // Return unregister function
  return () => {
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
  console.log('[linkStore] startLinkMode called with nodeId:', nodeId);
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
  console.log('[linkStore] createLink called with elementId:', elementId, 'nodeId:', nodeId);

  if (!nodeId) {
    console.error('[linkStore] No nodeId - link mode not active');
    return;
  }

  const elements = get(linkableElements);
  const element = elements.get(elementId);
  console.log('[linkStore] Available elements:', Array.from(elements.keys()));

  if (!element) {
    console.error('[linkStore] Element not found:', elementId);
    cancelLinkMode();
    return;
  }

  const currentValue = String(element.getValue());
  console.log('[linkStore] Creating link:', { nodeId, elementId, elementLabel: element.label, currentValue });

  linkMappings.update((map) => {
    const newMap = new Map(map);
    newMap.set(nodeId, {
      nodeId,
      elementId,
      elementLabel: element.label,
      status: 'linked',
      lastValue: currentValue,
    });
    console.log('[linkStore] Updated linkMappings:', Array.from(newMap.entries()));
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

  try {
    return String(element.getValue());
  } catch {
    return undefined;
  }
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

  for (const [nodeId, mapping] of mappings) {
    if (mapping.status !== 'linked') continue;

    const element = elements.get(mapping.elementId);
    if (!element) continue;

    try {
      result.set(nodeId, String(element.getValue()));
    } catch {
      // Skip elements that fail to provide value
    }
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
      newMappings.set(mapping.nodeId, {
        ...mapping,
        status: 'linked',
        lastValue: String(element.getValue()),
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
