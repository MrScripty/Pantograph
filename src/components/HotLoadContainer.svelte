<script lang="ts">
  /**
   * HotLoadContainer - Renders dynamically generated components with error isolation
   *
   * This component now uses the hotload-sandbox module for safe component rendering.
   * It maintains the same external interface while adding:
   * - Error boundaries around component rendering
   * - Timeout protection for imports
   * - Better error display with retry functionality
   */
  import { ComponentContainer } from '$lib/hotload-sandbox';
  import { componentRegistry } from '../services/HotLoadRegistry';
  import { Logger } from '../services/Logger';
  import { panelWidth } from '../stores/panelStore';

  // Logger adapter for the sandbox
  const sandboxLogger = {
    log: (event: string, data?: unknown, level?: 'info' | 'warn' | 'error') => {
      Logger.log(event, data, level);
    },
  };
</script>

<!-- Use the new ComponentContainer from hotload-sandbox -->
<ComponentContainer
  registry={componentRegistry}
  rightOffset={$panelWidth}
  logger={sandboxLogger}
/>
