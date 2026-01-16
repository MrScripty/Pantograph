<script lang="ts">
  import { onMount } from 'svelte';
  import type { SvelteComponent, ComponentType } from 'svelte';
  import type { LoggerInterface } from '../types';
  import { defaultLogger } from '../types';

  interface Props {
    /** The Svelte component to render */
    component: ComponentType<SvelteComponent> | null;
    /** Props to pass to the component */
    props?: Record<string, unknown>;
    /** Unique identifier for this component instance */
    componentId: string;
    /** Callback when a render error occurs */
    onRenderError?: (error: Error, componentId: string) => void;
    /** Logger for debugging */
    logger?: LoggerInterface;
  }

  let {
    component,
    props = {},
    componentId,
    onRenderError,
    logger = defaultLogger,
  }: Props = $props();

  // Track render state
  let hasError = $state(false);
  let errorMessage = $state('');
  let validated = $state(false);

  // Validate component before rendering
  $effect(() => {
    hasError = false;
    errorMessage = '';
    validated = false;

    if (!component) {
      hasError = true;
      errorMessage = 'Component is null or undefined';
      logger.log('SAFE_COMPONENT_NULL', { componentId }, 'warn');
      return;
    }

    // Type validation
    if (typeof component === 'string') {
      hasError = true;
      errorMessage = `Cannot render string as component: "${(component as string).slice(0, 50)}..."`;
      logger.log('SAFE_COMPONENT_STRING', { componentId, value: component }, 'error');
      onRenderError?.(new Error(errorMessage), componentId);
      return;
    }

    if (typeof component === 'number') {
      hasError = true;
      errorMessage = `Cannot render number as component: ${component}`;
      logger.log('SAFE_COMPONENT_NUMBER', { componentId, value: component }, 'error');
      onRenderError?.(new Error(errorMessage), componentId);
      return;
    }

    if (typeof component !== 'function' && typeof component !== 'object') {
      hasError = true;
      errorMessage = `Invalid component type: ${typeof component}`;
      logger.log('SAFE_COMPONENT_INVALID_TYPE', { componentId, type: typeof component }, 'error');
      onRenderError?.(new Error(errorMessage), componentId);
      return;
    }

    // Component appears valid
    validated = true;
    logger.log('SAFE_COMPONENT_VALIDATED', { componentId });
  });

  // Capture runtime errors during render
  onMount(() => {
    // Set up error handler for this component's boundary
    const handleError = (event: ErrorEvent) => {
      // Only handle errors that seem related to component rendering
      // This is imperfect but helps catch some errors
      if (event.message && !hasError) {
        logger.log(
          'SAFE_COMPONENT_RUNTIME_ERROR',
          { componentId, error: event.message },
          'error'
        );
        hasError = true;
        errorMessage = event.message;
        onRenderError?.(event.error || new Error(event.message), componentId);
      }
    };

    // Note: This is a global handler which isn't ideal for isolation
    // A more robust solution would use an iframe or web worker
    window.addEventListener('error', handleError);

    return () => {
      window.removeEventListener('error', handleError);
    };
  });
</script>

{#if hasError}
  <!-- Error state - render nothing, let parent show placeholder -->
  <div
    class="hidden"
    data-error={errorMessage}
    data-component-id={componentId}
  />
{:else if validated && component}
  <!-- Render the component -->
  {#key componentId}
    <svelte:component this={component} {...props} />
  {/key}
{:else}
  <!-- Loading/validating state -->
  <div class="flex items-center justify-center h-full w-full text-neutral-500 text-sm">
    <span>Validating...</span>
  </div>
{/if}
