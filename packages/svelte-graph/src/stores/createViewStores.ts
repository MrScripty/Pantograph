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
import { DEFAULT_ANIMATION } from '../types/view.ts';

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
  let activeAnimation: {
    timer: ReturnType<typeof setTimeout>;
    resolve: () => void;
  } | null = null;

  function cancelAnimation(): void {
    if (!activeAnimation) return;
    const previous = activeAnimation;
    activeAnimation = null;
    clearTimeout(previous.timer);
    previous.resolve();
  }

  function beginAnimation(target: ZoomTarget | null): Promise<void> {
    cancelAnimation();
    const completion = new Promise<void>((resolve) => {
      const animation = {
        resolve,
        timer: setTimeout(() => {
          if (activeAnimation !== animation) return;
          activeAnimation = null;
          zoomTarget.set(null);
          isAnimating.set(false);
          resolve();
        }, get(animationConfig).duration),
      };
      activeAnimation = animation;
    });
    isAnimating.set(true);
    zoomTarget.set(target);
    return completion;
  }

  // --- Actions ---

  async function zoomToOrchestration(targetNodeId?: string): Promise<void> {
    if (get(viewLevel) === 'orchestration') return;

    const completion = beginAnimation(targetNodeId
      ? { nodeId: targetNodeId, position: { x: 0, y: 0 } }
      : null);

    viewLevel.set('orchestration');
    groupStack.set([]);

    await completion;
  }

  async function zoomToDataGraph(orchestrationNodeId: string, dataGraphId: string): Promise<void> {
    const currentLevel = get(viewLevel);
    if (currentLevel === 'data-graph' || currentLevel === 'group') return;

    const completion = beginAnimation({ nodeId: orchestrationNodeId, position: { x: 0, y: 0 } });
    currentDataGraphId.set(dataGraphId);
    viewLevel.set('data-graph');

    await completion;
  }

  async function tabIntoGroup(groupId: string): Promise<void> {
    const completion = beginAnimation({ nodeId: groupId, position: { x: 0, y: 0 } });
    groupStack.update((stack) => [...stack, groupId]);
    viewLevel.set('group');

    await completion;
  }

  async function tabOutOfGroup(): Promise<void> {
    const stack = get(groupStack);
    if (stack.length === 0) return;

    const poppedGroupId = stack[stack.length - 1];
    const completion = beginAnimation({ nodeId: poppedGroupId, position: { x: 0, y: 0 } });
    groupStack.update((s) => s.slice(0, -1));

    if (stack.length === 1) {
      viewLevel.set('data-graph');
    }

    await completion;
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
        cancelAnimation();
        zoomTarget.set(null);
        isAnimating.set(false);
        groupStack.set([]);
        viewLevel.set('data-graph');
      }
    } else if (item.level === 'group') {
      const targetIndex = stack.indexOf(item.id);
      if (targetIndex >= 0 && targetIndex < stack.length - 1) {
        cancelAnimation();
        zoomTarget.set(null);
        isAnimating.set(false);
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
    cancelAnimation();
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

  let persistenceOwners = 0;
  let cleanupPersistence: (() => void) | null = null;

  /** Share auto-persistence while any caller owns an enable handle. */
  function enablePersistence(): () => void {
    if (!storageKey) return () => {};

    persistenceOwners += 1;
    if (persistenceOwners === 1) {
      let persistTimeout: ReturnType<typeof setTimeout> | null = null;
      const debouncedPersist = () => {
        if (persistTimeout) clearTimeout(persistTimeout);
        persistTimeout = setTimeout(() => {
          persistTimeout = null;
          persistViewState();
        }, 500);
      };

      const unsub1 = viewLevel.subscribe(debouncedPersist);
      const unsub2 = groupStack.subscribe(debouncedPersist);
      cleanupPersistence = () => {
        unsub1();
        unsub2();
        if (persistTimeout) clearTimeout(persistTimeout);
      };
    }

    let released = false;
    return () => {
      if (released) return;
      released = true;
      persistenceOwners -= 1;
      if (persistenceOwners === 0) {
        cleanupPersistence?.();
        cleanupPersistence = null;
      }
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
