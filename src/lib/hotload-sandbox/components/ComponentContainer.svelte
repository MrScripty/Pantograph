<script lang="ts">
  import { onMount } from 'svelte';
  import type { GeneratedComponent, LoggerInterface, HotloadConfig } from '../types';
  import { defaultLogger } from '../types';
  import { ComponentRegistry } from '../services/ComponentRegistry';
  import SafeComponent from './SafeComponent.svelte';
  import ErrorPlaceholder from './ErrorPlaceholder.svelte';
  import { interactionMode } from '../../../stores/interactionModeStore';
  import { canvasPan, type PanOffset } from '../../../stores/canvasStore';

  interface Props {
    /** The component registry to subscribe to */
    registry: ComponentRegistry;
    /** Optional right offset (e.g., for side panels) */
    rightOffset?: number;
    /** Logger for debugging */
    logger?: LoggerInterface;
    /** Full config (alternative to individual props) */
    config?: HotloadConfig;
  }

  let {
    registry,
    rightOffset = 0,
    logger = defaultLogger,
    config,
  }: Props = $props();

  // Use config logger if provided
  const activeLogger = $derived(config?.logger ?? logger);

  // Component state from registry
  let components: GeneratedComponent[] = $state([]);

  // Track render errors locally (in addition to registry)
  let renderErrors: Map<string, Error> = $state(new Map());

  // Track interaction mode for pointer-events
  let currentMode: 'draw' | 'interact' = $state('draw');

  // Track canvas pan offset
  let currentPan: PanOffset = $state({ x: 0, y: 0 });

  onMount(() => {
    const unsubscribe = registry.subscribe((next) => {
      components = next;
      activeLogger.log('container_components_updated', { count: next.length });
    });

    const unsubscribeMode = interactionMode.subscribe((mode) => {
      currentMode = mode;
    });

    const unsubscribePan = canvasPan.subscribe((pan) => {
      currentPan = pan;
    });

    return () => {
      unsubscribe();
      unsubscribeMode();
      unsubscribePan();
    };
  });

  /**
   * Handle render error from SafeComponent
   */
  function handleRenderError(error: Error, componentId: string) {
    activeLogger.log('container_render_error', { componentId, error: error.message }, 'error');

    // Update local state - create new Map to trigger Svelte 5 reactivity
    const newErrors = new Map(renderErrors);
    newErrors.set(componentId, error);
    renderErrors = newErrors;

    // Update registry
    registry.setRenderError(componentId, error.message);
  }

  /**
   * Handle retry request
   */
  async function handleRetry(componentId: string) {
    activeLogger.log('container_retry', { componentId });

    // Clear local error - create new Map to trigger Svelte 5 reactivity
    const newErrors = new Map(renderErrors);
    newErrors.delete(componentId);
    renderErrors = newErrors;

    // Retry via registry
    await registry.retry(componentId);
  }

  /**
   * Determine if a component should show error placeholder
   */
  function shouldShowError(comp: GeneratedComponent): boolean {
    return (
      comp.status === 'error' ||
      !comp.component ||
      renderErrors.has(comp.id) ||
      !!comp.error ||
      !!comp.renderError
    );
  }

  /**
   * Get error message for a component
   */
  function getErrorMessage(comp: GeneratedComponent): string {
    return (
      renderErrors.get(comp.id)?.message ??
      comp.renderError ??
      comp.error ??
      'Unknown error'
    );
  }
</script>

<!-- Generated components container - sits below the canvas but above the background -->
<div
  class="fixed inset-0 pointer-events-none z-10 overflow-hidden"
  style="right: {rightOffset}px;"
>
  <!-- Inner container that moves with canvas pan -->
  <div style="transform: translate({currentPan.x}px, {currentPan.y}px);">
  {#each components as comp (comp.id)}
    <div
      class="absolute {currentMode === 'interact' ? 'pointer-events-auto' : 'pointer-events-none'}"
      style="left: {comp.position.x}px; top: {comp.position.y}px; width: {comp.size.width}px; height: {comp.size.height}px;"
    >
      {#if shouldShowError(comp)}
        <!-- Show error placeholder -->
        <ErrorPlaceholder
          error={registry.getErrorReporter().getLatestError(comp.id)}
          errorMessage={getErrorMessage(comp)}
          componentId={comp.id}
          onRetry={() => handleRetry(comp.id)}
          showRetry={true}
        />
      {:else if comp.status === 'loading'}
        <!-- Loading state -->
        <div
          class="flex items-center justify-center h-full w-full bg-neutral-800/50 border border-dashed border-neutral-600 rounded-lg text-neutral-500 text-sm"
        >
          <span class="text-center p-2">
            <span class="animate-pulse">Loading...</span>
            <br />
            <span class="text-xs text-neutral-600">{comp.id}</span>
          </span>
        </div>
      {:else}
        <!-- Render component safely -->
        <SafeComponent
          component={comp.component}
          props={comp.props ?? {}}
          componentId={comp.id}
          onRenderError={handleRenderError}
          logger={activeLogger}
        />
      {/if}
    </div>
  {/each}
  </div>
</div>
