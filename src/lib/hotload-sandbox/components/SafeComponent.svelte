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
      logger.log('safe_component_null', { componentId }, 'warn');
      return;
    }

    // Type validation
    if (typeof component === 'string') {
      hasError = true;
      errorMessage = `Cannot render string as component: "${(component as string).slice(0, 50)}..."`;
      logger.log('safe_component_string', { componentId, value: component }, 'error');
      onRenderError?.(new Error(errorMessage), componentId);
      return;
    }

    if (typeof component === 'number') {
      hasError = true;
      errorMessage = `Cannot render number as component: ${component}`;
      logger.log('safe_component_number', { componentId, value: component }, 'error');
      onRenderError?.(new Error(errorMessage), componentId);
      return;
    }

    if (typeof component !== 'function' && typeof component !== 'object') {
      hasError = true;
      errorMessage = `Invalid component type: ${typeof component}`;
      logger.log('safe_component_invalid_type', { componentId, type: typeof component }, 'error');
      onRenderError?.(new Error(errorMessage), componentId);
      return;
    }

    // Component appears valid
    validated = true;
    logger.log('safe_component_validated', { componentId });
  });

  // Capture runtime errors during render
  onMount(() => {
    // Track when this component was mounted to filter stale errors
    const mountTime = Date.now();

    // Set up error handler for this component's boundary
    const handleError = (event: ErrorEvent) => {
      // Filter out errors that occurred before this component mounted
      // or that have already been handled
      if (hasError) return;

      // Try to determine if this error is related to our component
      // by checking if the error occurred shortly after mount or
      // if the stack trace references our component path
      const isRecentError = Date.now() - mountTime < 1000;
      const errorStack = event.error?.stack || '';
      const isRelatedError = isRecentError || errorStack.includes('generated/');

      if (event.message && isRelatedError) {
        logger.log(
          'safe_component_runtime_error',
          { componentId, error: event.message, isRecentError },
          'error'
        );
        hasError = true;
        errorMessage = event.message;
        onRenderError?.(event.error || new Error(event.message), componentId);
      }
    };

    // LIMITATION: This is a global error handler which isn't ideal for isolation.
    // Errors from unrelated code may be caught if they occur during the window
    // after mount. A more robust solution would use:
    // - An iframe sandbox (strongest isolation, but complex communication)
    // - A try/catch wrapper around render (not possible with Svelte's reactive model)
    // - Error boundaries (not yet available in Svelte 5)
    // TODO: Consider iframe isolation for production use cases
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
