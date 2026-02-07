/**
 * View Store Factory — creates per-instance navigation and zoom state
 *
 * Manages the multi-level view system:
 * 1. Orchestration Graphs - Control flow (sequence, conditions, loops)
 * 2. Data Graphs - Computation (LLM inference, validation, etc.)
 * 3. Groups - Nested node groups within data graphs
 */
import { writable, derived, get } from 'svelte/store';
import type { ViewLevel, BreadcrumbItem, ViewportState, ZoomTarget, AnimationConfig } from '../types/view.js';
import { DEFAULT_ANIMATION } from '../types/view.js';

export interface ViewStoreOptions {
  /** localStorage key for auto-persistence (omit to disable) */
  storageKey?: string;
}

export interface ViewStores {
  // Writable stores
  viewLevel: ReturnType<typeof writable<ViewLevel>>;
  currentOrchestrationId: ReturnType<typeof writable<string | null>>;
  currentDataGraphId: ReturnType<typeof writable<string | null>>;
  groupStack: ReturnType<typeof writable<string[]>>;
  isAnimating: ReturnType<typeof writable<boolean>>;
  zoomTarget: ReturnType<typeof writable<ZoomTarget | null>>;
  animationConfig: ReturnType<typeof writable<AnimationConfig>>;

  // Derived stores
  breadcrumb: ReturnType<typeof derived>;
  canNavigateBack: ReturnType<typeof derived>;
  navigationDepth: ReturnType<typeof derived>;

  // Actions
  zoomToOrchestration: (targetNodeId?: string) => Promise<void>;
  zoomToDataGraph: (orchestrationNodeId: string, dataGraphId: string) => Promise<void>;
  tabIntoGroup: (groupId: string) => Promise<void>;
  tabOutOfGroup: () => Promise<void>;
  navigateBack: () => Promise<void>;
  navigateToBreadcrumb: (item: BreadcrumbItem) => Promise<void>;
  saveViewport: (viewId: string, viewport: ViewportState) => void;
  getSavedViewport: (viewId: string) => ViewportState | undefined;
  setOrchestrationContext: (orchestrationId: string | null) => void;
  resetViewState: () => void;
  setAnimationConfig: (config: Partial<AnimationConfig>) => void;
  persistViewState: () => void;
  restoreViewState: () => boolean;
  enablePersistence: () => () => void;
}

export function createViewStores(options?: ViewStoreOptions): ViewStores {
  const storageKey = options?.storageKey;

  // --- State ---
  const viewLevel = writable<ViewLevel>('data-graph');
  const currentOrchestrationId = writable<string | null>(null);
  const currentDataGraphId = writable<string | null>(null);
  const groupStack = writable<string[]>([]);
  const isAnimating = writable<boolean>(false);
  const zoomTarget = writable<ZoomTarget | null>(null);
  const animationConfig = writable<AnimationConfig>(DEFAULT_ANIMATION);
  const savedViewports = writable<Map<string, ViewportState>>(new Map());

  // --- Derived ---
  const breadcrumb = derived(
    [viewLevel, currentOrchestrationId, currentDataGraphId, groupStack],
    ([$level, $orchestrationId, $dataGraphId, $groupStack]): BreadcrumbItem[] => {
      const items: BreadcrumbItem[] = [];

      if ($orchestrationId) {
        items.push({ id: $orchestrationId, name: $orchestrationId, level: 'orchestration' });
      }
      if ($dataGraphId && $level !== 'orchestration') {
        items.push({ id: $dataGraphId, name: $dataGraphId, level: 'data-graph' });
      }
      if ($level === 'group' && $groupStack.length > 0) {
        for (const groupId of $groupStack) {
          items.push({ id: groupId, name: groupId, level: 'group' });
        }
      }
      return items;
    }
  );

  const canNavigateBack = derived(
    [viewLevel, groupStack, currentOrchestrationId],
    ([$level, $groupStack, $orchestrationId]) => {
      if ($level === 'group' && $groupStack.length > 0) return true;
      if ($level === 'data-graph' && $orchestrationId) return true;
      return false;
    }
  );

  const navigationDepth = derived(
    [viewLevel, groupStack],
    ([$level, $groupStack]) => {
      if ($level === 'orchestration') return 0;
      if ($level === 'data-graph') return 1;
      return 1 + $groupStack.length;
    }
  );

  // --- Utility ---
  function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  // --- Actions ---

  async function zoomToOrchestration(targetNodeId?: string): Promise<void> {
    if (get(viewLevel) === 'orchestration') return;

    isAnimating.set(true);
    if (targetNodeId) {
      zoomTarget.set({ nodeId: targetNodeId, position: { x: 0, y: 0 } });
    }

    viewLevel.set('orchestration');
    groupStack.set([]);

    const config = get(animationConfig);
    await sleep(config.duration);
    zoomTarget.set(null);
    isAnimating.set(false);
  }

  async function zoomToDataGraph(orchestrationNodeId: string, dataGraphId: string): Promise<void> {
    const currentLevel = get(viewLevel);
    if (currentLevel === 'data-graph' || currentLevel === 'group') return;

    isAnimating.set(true);
    zoomTarget.set({ nodeId: orchestrationNodeId, position: { x: 0, y: 0 } });
    currentDataGraphId.set(dataGraphId);
    viewLevel.set('data-graph');

    const config = get(animationConfig);
    await sleep(config.duration);
    zoomTarget.set(null);
    isAnimating.set(false);
  }

  async function tabIntoGroup(groupId: string): Promise<void> {
    isAnimating.set(true);
    zoomTarget.set({ nodeId: groupId, position: { x: 0, y: 0 } });
    groupStack.update((stack) => [...stack, groupId]);
    viewLevel.set('group');

    const config = get(animationConfig);
    await sleep(config.duration);
    zoomTarget.set(null);
    isAnimating.set(false);
  }

  async function tabOutOfGroup(): Promise<void> {
    const stack = get(groupStack);
    if (stack.length === 0) return;

    isAnimating.set(true);
    const poppedGroupId = stack[stack.length - 1];
    groupStack.update((s) => s.slice(0, -1));
    zoomTarget.set({ nodeId: poppedGroupId, position: { x: 0, y: 0 } });

    if (stack.length === 1) {
      viewLevel.set('data-graph');
    }

    const config = get(animationConfig);
    await sleep(config.duration);
    zoomTarget.set(null);
    isAnimating.set(false);
  }

  async function navigateBack(): Promise<void> {
    const level = get(viewLevel);
    const stack = get(groupStack);
    const orchestrationId = get(currentOrchestrationId);

    if (level === 'group' && stack.length > 0) {
      await tabOutOfGroup();
    } else if (level === 'data-graph' && orchestrationId) {
      await zoomToOrchestration();
    }
  }

  async function navigateToBreadcrumb(item: BreadcrumbItem): Promise<void> {
    const currentLevel = get(viewLevel);
    const stack = get(groupStack);

    if (item.level === 'orchestration') {
      await zoomToOrchestration();
    } else if (item.level === 'data-graph') {
      if (currentLevel === 'group') {
        groupStack.set([]);
        viewLevel.set('data-graph');
      }
    } else if (item.level === 'group') {
      const targetIndex = stack.indexOf(item.id);
      if (targetIndex >= 0 && targetIndex < stack.length - 1) {
        groupStack.set(stack.slice(0, targetIndex + 1));
      }
    }
  }

  function saveViewport(viewId: string, viewport: ViewportState): void {
    savedViewports.update((map) => {
      const newMap = new Map(map);
      newMap.set(viewId, viewport);
      return newMap;
    });
  }

  function getSavedViewport(viewId: string): ViewportState | undefined {
    return get(savedViewports).get(viewId);
  }

  function setOrchestrationContext(orchestrationId: string | null): void {
    currentOrchestrationId.set(orchestrationId);
  }

  function resetViewState(): void {
    viewLevel.set('data-graph');
    currentOrchestrationId.set(null);
    currentDataGraphId.set(null);
    groupStack.set([]);
    zoomTarget.set(null);
    isAnimating.set(false);
  }

  function setAnimationConfigFn(config: Partial<AnimationConfig>): void {
    animationConfig.update((current) => ({ ...current, ...config }));
  }

  function persistViewState(): void {
    if (!storageKey) return;
    try {
      const state = {
        viewLevel: get(viewLevel),
        orchestrationId: get(currentOrchestrationId),
        dataGraphId: get(currentDataGraphId),
        groupStack: get(groupStack),
      };
      localStorage.setItem(storageKey, JSON.stringify(state));
    } catch {
      // localStorage might not be available
    }
  }

  function restoreViewState(): boolean {
    if (!storageKey) return false;
    try {
      const stored = localStorage.getItem(storageKey);
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

  /** Enable auto-persistence on state changes. Returns a cleanup function. */
  function enablePersistence(): () => void {
    if (!storageKey) return () => {};

    let persistTimeout: ReturnType<typeof setTimeout> | null = null;
    const debouncedPersist = () => {
      if (persistTimeout) clearTimeout(persistTimeout);
      persistTimeout = setTimeout(persistViewState, 500);
    };

    const unsub1 = viewLevel.subscribe(debouncedPersist);
    const unsub2 = groupStack.subscribe(debouncedPersist);

    return () => {
      unsub1();
      unsub2();
      if (persistTimeout) clearTimeout(persistTimeout);
    };
  }

  return {
    // Stores
    viewLevel, currentOrchestrationId, currentDataGraphId, groupStack,
    isAnimating, zoomTarget, animationConfig,
    breadcrumb, canNavigateBack, navigationDepth,
    // Actions
    zoomToOrchestration, zoomToDataGraph, tabIntoGroup, tabOutOfGroup,
    navigateBack, navigateToBreadcrumb, saveViewport, getSavedViewport,
    setOrchestrationContext, resetViewState,
    setAnimationConfig: setAnimationConfigFn,
    persistViewState, restoreViewState, enablePersistence,
  };
}
