/**
 * View Store - Manages zoom transitions and navigation between graph levels
 *
 * This store handles the two-level system:
 * 1. Orchestration Graphs - Control flow (sequence, conditions, loops)
 * 2. Data Graphs - Computation (LLM inference, validation, etc.)
 *
 * Plus navigation into node groups within data graphs.
 */
import { writable, derived, get } from 'svelte/store';

// --- Types ---

export type ViewLevel = 'orchestration' | 'data-graph' | 'group';

export interface BreadcrumbItem {
  id: string;
  name: string;
  level: ViewLevel;
}

export interface ViewportState {
  x: number;
  y: number;
  zoom: number;
}

export interface ZoomTarget {
  nodeId: string;
  position: { x: number; y: number };
  bounds?: { width: number; height: number };
}

// Animation configuration
export interface AnimationConfig {
  duration: number;
  easing: string;
}

// --- Constants ---

const DEFAULT_ANIMATION: AnimationConfig = {
  duration: 300,
  easing: 'cubic-bezier(0.4, 0, 0.2, 1)', // ease-out
};

const STORAGE_KEY = 'pantograph.viewState';

// --- State ---

/** Current view level (orchestration, data-graph, or group) */
export const viewLevel = writable<ViewLevel>('data-graph');

/** Current orchestration graph ID (if viewing orchestration or zoomed into a data graph) */
export const currentOrchestrationId = writable<string | null>(null);

/** Current data graph ID (if viewing a data graph or group within it) */
export const currentDataGraphId = writable<string | null>(null);

/** Stack of group IDs for nested group navigation */
export const groupStack = writable<string[]>([]);

/** Whether a zoom animation is in progress */
export const isAnimating = writable<boolean>(false);

/** The target node for zoom animations (node being zoomed into/out of) */
export const zoomTarget = writable<ZoomTarget | null>(null);

/** Animation configuration */
export const animationConfig = writable<AnimationConfig>(DEFAULT_ANIMATION);

/** Saved viewport states for returning to previous views */
const savedViewports = writable<Map<string, ViewportState>>(new Map());

// --- Derived ---

/** Computed breadcrumb trail based on current navigation state */
export const breadcrumb = derived(
  [viewLevel, currentOrchestrationId, currentDataGraphId, groupStack],
  ([$level, $orchestrationId, $dataGraphId, $groupStack]): BreadcrumbItem[] => {
    const items: BreadcrumbItem[] = [];

    // Root: orchestration level (if we have one)
    if ($orchestrationId) {
      items.push({
        id: $orchestrationId,
        name: $orchestrationId, // Will be replaced with actual name from store
        level: 'orchestration',
      });
    }

    // Data graph level
    if ($dataGraphId && $level !== 'orchestration') {
      items.push({
        id: $dataGraphId,
        name: $dataGraphId, // Will be replaced with actual name
        level: 'data-graph',
      });
    }

    // Group levels (nested groups)
    if ($level === 'group' && $groupStack.length > 0) {
      for (const groupId of $groupStack) {
        items.push({
          id: groupId,
          name: groupId, // Will be replaced with actual name
          level: 'group',
        });
      }
    }

    return items;
  }
);

/** Whether navigation back is possible */
export const canNavigateBack = derived(
  [viewLevel, groupStack, currentOrchestrationId],
  ([$level, $groupStack, $orchestrationId]) => {
    // Can go back if in a group
    if ($level === 'group' && $groupStack.length > 0) return true;
    // Can go back if in data-graph and came from orchestration
    if ($level === 'data-graph' && $orchestrationId) return true;
    return false;
  }
);

/** Current depth in the navigation hierarchy */
export const navigationDepth = derived(
  [viewLevel, groupStack],
  ([$level, $groupStack]) => {
    let depth = 0;
    if ($level === 'orchestration') depth = 0;
    else if ($level === 'data-graph') depth = 1;
    else if ($level === 'group') depth = 1 + $groupStack.length;
    return depth;
  }
);

// --- Actions ---

/**
 * Zoom out from current data graph to orchestration view
 * The data graph shrinks into a single node in the orchestration
 */
export async function zoomToOrchestration(
  targetNodeId?: string
): Promise<void> {
  const currentLevel = get(viewLevel);
  if (currentLevel === 'orchestration') return;

  isAnimating.set(true);

  // Save current data graph ID and set zoom target
  const dataGraphId = get(currentDataGraphId);
  if (targetNodeId) {
    zoomTarget.set({
      nodeId: targetNodeId,
      position: { x: 0, y: 0 }, // Will be computed by component
    });
  }

  // Transition to orchestration view
  viewLevel.set('orchestration');
  groupStack.set([]);

  // Wait for animation to complete
  const config = get(animationConfig);
  await sleep(config.duration);

  zoomTarget.set(null);
  isAnimating.set(false);
}

/**
 * Zoom into an orchestration node to view its data graph
 * The node expands to fill the view
 */
export async function zoomToDataGraph(
  orchestrationNodeId: string,
  dataGraphId: string
): Promise<void> {
  const currentLevel = get(viewLevel);
  if (currentLevel === 'data-graph' || currentLevel === 'group') return;

  isAnimating.set(true);

  // Set zoom target for animation
  zoomTarget.set({
    nodeId: orchestrationNodeId,
    position: { x: 0, y: 0 }, // Will be computed by component
  });

  // Transition to data graph view
  currentDataGraphId.set(dataGraphId);
  viewLevel.set('data-graph');

  // Wait for animation to complete
  const config = get(animationConfig);
  await sleep(config.duration);

  zoomTarget.set(null);
  isAnimating.set(false);
}

/**
 * Tab into a node group to view its internal nodes
 */
export async function tabIntoGroup(groupId: string): Promise<void> {
  isAnimating.set(true);

  // Set zoom target
  zoomTarget.set({
    nodeId: groupId,
    position: { x: 0, y: 0 },
  });

  // Push group onto stack and switch to group view
  groupStack.update((stack) => [...stack, groupId]);
  viewLevel.set('group');

  const config = get(animationConfig);
  await sleep(config.duration);

  zoomTarget.set(null);
  isAnimating.set(false);
}

/**
 * Tab out of current group to parent group or data graph
 */
export async function tabOutOfGroup(): Promise<void> {
  const stack = get(groupStack);
  if (stack.length === 0) return;

  isAnimating.set(true);

  // Pop from group stack
  const poppedGroupId = stack[stack.length - 1];
  groupStack.update((s) => s.slice(0, -1));

  // Set zoom target for animation (collapsing back to group node)
  zoomTarget.set({
    nodeId: poppedGroupId,
    position: { x: 0, y: 0 },
  });

  // If no more groups, go back to data-graph level
  if (stack.length === 1) {
    viewLevel.set('data-graph');
  }

  const config = get(animationConfig);
  await sleep(config.duration);

  zoomTarget.set(null);
  isAnimating.set(false);
}

/**
 * Navigate back one level in the hierarchy
 */
export async function navigateBack(): Promise<void> {
  const level = get(viewLevel);
  const stack = get(groupStack);
  const orchestrationId = get(currentOrchestrationId);

  if (level === 'group' && stack.length > 0) {
    await tabOutOfGroup();
  } else if (level === 'data-graph' && orchestrationId) {
    await zoomToOrchestration();
  }
}

/**
 * Navigate to a specific breadcrumb item
 */
export async function navigateToBreadcrumb(item: BreadcrumbItem): Promise<void> {
  const currentLevel = get(viewLevel);
  const stack = get(groupStack);

  if (item.level === 'orchestration') {
    // Go all the way back to orchestration
    await zoomToOrchestration();
  } else if (item.level === 'data-graph') {
    // Go back to data graph (exit all groups)
    if (currentLevel === 'group') {
      groupStack.set([]);
      viewLevel.set('data-graph');
    }
  } else if (item.level === 'group') {
    // Navigate to specific group depth
    const targetIndex = stack.indexOf(item.id);
    if (targetIndex >= 0 && targetIndex < stack.length - 1) {
      groupStack.set(stack.slice(0, targetIndex + 1));
    }
  }
}

/**
 * Save viewport state for a specific view (for restoring when returning)
 */
export function saveViewport(viewId: string, viewport: ViewportState): void {
  savedViewports.update((map) => {
    const newMap = new Map(map);
    newMap.set(viewId, viewport);
    return newMap;
  });
}

/**
 * Get saved viewport state for a view
 */
export function getSavedViewport(viewId: string): ViewportState | undefined {
  return get(savedViewports).get(viewId);
}

/**
 * Set the orchestration context (called when loading an orchestration)
 */
export function setOrchestrationContext(orchestrationId: string | null): void {
  currentOrchestrationId.set(orchestrationId);
}

/**
 * Reset view state to default (data-graph level, no groups)
 */
export function resetViewState(): void {
  viewLevel.set('data-graph');
  currentOrchestrationId.set(null);
  currentDataGraphId.set(null);
  groupStack.set([]);
  zoomTarget.set(null);
  isAnimating.set(false);
}

/**
 * Update animation configuration
 */
export function setAnimationConfig(config: Partial<AnimationConfig>): void {
  animationConfig.update((current) => ({
    ...current,
    ...config,
  }));
}

/**
 * Persist view state to localStorage
 */
export function persistViewState(): void {
  try {
    const state = {
      viewLevel: get(viewLevel),
      orchestrationId: get(currentOrchestrationId),
      dataGraphId: get(currentDataGraphId),
      groupStack: get(groupStack),
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // localStorage might not be available
  }
}

/**
 * Restore view state from localStorage
 */
export function restoreViewState(): boolean {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const state = JSON.parse(stored);
      if (state.viewLevel) viewLevel.set(state.viewLevel);
      if (state.orchestrationId) currentOrchestrationId.set(state.orchestrationId);
      if (state.dataGraphId) currentDataGraphId.set(state.dataGraphId);
      if (state.groupStack) groupStack.set(state.groupStack);
      return true;
    }
  } catch {
    // localStorage might not be available or corrupted
  }
  return false;
}

// --- Utilities ---

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// --- Subscriptions for auto-persistence ---

// Auto-persist on changes
let persistTimeout: ReturnType<typeof setTimeout> | null = null;
viewLevel.subscribe(() => {
  if (persistTimeout) clearTimeout(persistTimeout);
  persistTimeout = setTimeout(persistViewState, 500);
});

groupStack.subscribe(() => {
  if (persistTimeout) clearTimeout(persistTimeout);
  persistTimeout = setTimeout(persistViewState, 500);
});
