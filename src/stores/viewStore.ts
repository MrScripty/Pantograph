/**
 * View Store — thin re-export layer over singleton store instances.
 *
 * Pantograph components (UnifiedGraphView, etc.) import from this file.
 * The underlying stores are the same instances used by the package context.
 */
import { viewStores } from './storeInstances';

// Re-export types from the package
export type { ViewLevel, BreadcrumbItem, ViewportState, ZoomTarget, AnimationConfig } from '@pantograph/svelte-graph';

// --- Writable stores ---
export const viewLevel = viewStores.viewLevel;
export const currentOrchestrationId = viewStores.currentOrchestrationId;
export const currentDataGraphId = viewStores.currentDataGraphId;
export const groupStack = viewStores.groupStack;
export const isAnimating = viewStores.isAnimating;
export const zoomTarget = viewStores.zoomTarget;
export const animationConfig = viewStores.animationConfig;

// --- Derived stores ---
export const breadcrumb = viewStores.breadcrumb;
export const canNavigateBack = viewStores.canNavigateBack;
export const navigationDepth = viewStores.navigationDepth;

// --- Actions ---
export const zoomToOrchestration = viewStores.zoomToOrchestration;
export const zoomToDataGraph = viewStores.zoomToDataGraph;
export const tabIntoGroup = viewStores.tabIntoGroup;
export const tabOutOfGroup = viewStores.tabOutOfGroup;
export const navigateBack = viewStores.navigateBack;
export const navigateToBreadcrumb = viewStores.navigateToBreadcrumb;
export const saveViewport = viewStores.saveViewport;
export const getSavedViewport = viewStores.getSavedViewport;
export const setOrchestrationContext = viewStores.setOrchestrationContext;
export const resetViewState = viewStores.resetViewState;
export const setAnimationConfig = viewStores.setAnimationConfig;
export const persistViewState = viewStores.persistViewState;
export const restoreViewState = viewStores.restoreViewState;
