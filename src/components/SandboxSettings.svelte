<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { expandedSection, toggleSection } from '../stores/accordionStore';
  import { StatusGlyph } from './workbench/ui';

  // Types matching the Rust backend
  type ImportValidationMode = 'none' | 'import_resolve' | 'vite_integration' | 'esbuild_bundle';

  interface SandboxConfig {
    import_validation_mode: ImportValidationMode;
    validation_timeout_ms: number;
    allowed_packages: string[];
  }

  let isLoading = $state(true);
  let isSaving = $state(false);
  let loadError: string | null = $state(null);

  // Form state
  let validationMode: ImportValidationMode = $state('none');
  let timeoutMs = $state(5000);
  let allowedPackages = $state('');

  // Original values for change detection
  let originalMode: ImportValidationMode = $state('none');
  let originalTimeout = $state(5000);
  let originalPackages = $state('');

  onMount(async () => {
    await loadConfig();
  });

  const loadConfig = async () => {
    isLoading = true;
    loadError = null;
    try {
      const config: SandboxConfig = await invoke('get_sandbox_config');
      validationMode = config.import_validation_mode;
      timeoutMs = config.validation_timeout_ms;
      allowedPackages = config.allowed_packages.join('\n');

      // Store original values
      originalMode = validationMode;
      originalTimeout = timeoutMs;
      originalPackages = allowedPackages;
    } catch (error) {
      loadError = String(error);
      console.error('Failed to load sandbox config:', error);
    } finally {
      isLoading = false;
    }
  };

  const saveConfig = async () => {
    isSaving = true;
    try {
      const config: SandboxConfig = {
        import_validation_mode: validationMode,
        validation_timeout_ms: timeoutMs,
        allowed_packages: allowedPackages
          .split('\n')
          .map(p => p.trim())
          .filter(p => p.length > 0),
      };
      await invoke('set_sandbox_config', { sandbox: config });

      // Update original values
      originalMode = validationMode;
      originalTimeout = timeoutMs;
      originalPackages = allowedPackages;
    } catch (error) {
      console.error('Failed to save sandbox config:', error);
    } finally {
      isSaving = false;
    }
  };

  const getModeLabel = (mode: ImportValidationMode): string => {
    switch (mode) {
      case 'none': return 'None';
      case 'import_resolve': return 'Import Resolution';
      case 'vite_integration': return 'Vite';
      case 'esbuild_bundle': return 'esbuild';
    }
  };

  const getModeDescription = (mode: ImportValidationMode): string => {
    switch (mode) {
      case 'none':
        return 'No import validation. Errors only appear at runtime.';
      case 'import_resolve':
        return 'Fast check against package.json dependencies. Good for catching typos.';
      case 'vite_integration':
        return 'Uses Vite\'s resolver. Most accurate but adds latency.';
      case 'esbuild_bundle':
        return 'Full bundle check with esbuild. Catches all errors but slowest.';
    }
  };

  let hasChanges = $derived(
    validationMode !== originalMode ||
    timeoutMs !== originalTimeout ||
    allowedPackages !== originalPackages
  );
</script>

<section class="pg-card" aria-labelledby="settings-sandbox-title">
  <div class="flex items-center gap-2">
    <h3 class="m-0 min-w-0 flex-1">
      <button
        type="button"
        id="settings-sandbox-title"
        class="flex w-full items-center gap-2 text-left text-[13px] font-bold text-pg-text-bright"
        aria-expanded={$expandedSection === 'sandbox'}
        onclick={() => toggleSection('sandbox')}
      >
        Sandbox settings
        <span class="pg-muted text-[11px] font-normal" aria-hidden="true">
          {$expandedSection === 'sandbox' ? '▴' : '▾'}
        </span>
      </button>
    </h3>
    <span class="flex items-center gap-1.5 pg-small text-pg-text-soft">
      <StatusGlyph state={isLoading ? 'running' : loadError ? 'failed' : 'succeeded'} small />
      {isLoading ? 'Loading' : loadError ? 'Unavailable' : 'Loaded'}
    </span>
  </div>

  {#if $expandedSection === 'sandbox'}
    <div class="mt-3 flex flex-col gap-4 border-t border-pg-border-soft pt-3">
      {#if loadError}
        <p class="m-0 pg-muted pg-small" role="alert">Failed to load config: {loadError}</p>
      {/if}

      <div class="flex flex-col gap-1">
        <label for="validation-mode-select" class="pg-fl">Import validation mode</label>
        <select
          id="validation-mode-select"
          bind:value={validationMode}
          disabled={isLoading}
          class="pg-field w-full"
        >
          <option value="none">None (Fastest)</option>
          <option value="import_resolve">Import Resolution (Recommended)</option>
          <option value="vite_integration">Vite Integration (Most Accurate)</option>
          <option value="esbuild_bundle">esbuild Bundle (Thorough)</option>
        </select>
        <p class="m-0 pg-muted pg-small">{getModeDescription(validationMode)}</p>
      </div>

      <div class="flex flex-col gap-1">
        <label for="validation-timeout-range" class="pg-fl">Validation timeout (ms)</label>
        <div class="flex items-center gap-3">
          <input
            id="validation-timeout-range"
            type="range"
            bind:value={timeoutMs}
            min="1000"
            max="30000"
            step="1000"
            disabled={validationMode === 'none'}
            class="min-w-0 flex-1 cursor-pointer accent-pg-text disabled:cursor-default disabled:opacity-50"
          />
          <span class="pg-mono w-20 text-right">{timeoutMs}ms</span>
        </div>
        <p class="m-0 pg-muted pg-small">Maximum time to wait for validation script.</p>
      </div>

      <div class="flex flex-col gap-1">
        <label for="allowed-packages-textarea" class="pg-fl">Additional allowed packages</label>
        <textarea
          id="allowed-packages-textarea"
          bind:value={allowedPackages}
          disabled={validationMode === 'none'}
          placeholder="package-name
@scope/package"
          rows="3"
          class="pg-field pg-mono h-auto resize-none py-1.5"
        ></textarea>
        <p class="m-0 pg-muted pg-small">
          One package per line. Added to package.json dependencies for validation.
        </p>
      </div>

      {#if hasChanges}
        <div>
          <button type="button" onclick={saveConfig} disabled={isSaving} class="pg-btn pg-btn-primary">
            {isSaving ? 'Saving...' : 'Save sandbox settings'}
          </button>
        </div>
      {/if}

      <p class="m-0 pg-muted pg-small">
        Import validation catches errors before they reach the main app. Settings apply to the next
        generated component.
      </p>
    </div>
  {:else}
    <p class="m-0 mt-1 pg-muted pg-small">
      {getModeLabel(validationMode)}{#if validationMode !== 'none'}, {timeoutMs}ms timeout{/if}
    </p>
  {/if}
</section>
