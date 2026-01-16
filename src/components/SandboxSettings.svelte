<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { expandedSection, toggleSection } from '../stores/accordionStore';

  // Types matching the Rust backend
  type ImportValidationMode = 'none' | 'import_resolve' | 'vite_integration' | 'esbuild_bundle';

  interface SandboxConfig {
    import_validation_mode: ImportValidationMode;
    validation_timeout_ms: number;
    allowed_packages: string[];
  }

  let isLoading = true;
  let isSaving = false;
  let loadError: string | null = null;

  // Form state
  let validationMode: ImportValidationMode = 'none';
  let timeoutMs = 5000;
  let allowedPackages = '';

  // Original values for change detection
  let originalMode: ImportValidationMode = 'none';
  let originalTimeout = 5000;
  let originalPackages = '';

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

  $: hasChanges =
    validationMode !== originalMode ||
    timeoutMs !== originalTimeout ||
    allowedPackages !== originalPackages;
</script>

<div class="space-y-3">
  <!-- Header with toggle -->
  <button
    class="w-full flex items-center justify-between text-xs uppercase tracking-wider text-neutral-500 hover:text-neutral-400 transition-colors"
    onclick={() => toggleSection('sandbox')}
  >
    <div class="flex items-center gap-2">
      <span>Sandbox Settings</span>
      {#if isLoading}
        <span class="w-1.5 h-1.5 rounded-full bg-yellow-500 animate-pulse"></span>
      {:else if loadError}
        <span class="w-1.5 h-1.5 rounded-full bg-red-500"></span>
      {:else}
        <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
      {/if}
    </div>
    <svg
      class="w-3 h-3 transform transition-transform {$expandedSection === 'sandbox' ? 'rotate-180' : ''}"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if $expandedSection === 'sandbox'}
    <div class="space-y-4 p-3 bg-neutral-800/30 rounded-lg">
      {#if loadError}
        <div class="text-[10px] text-red-400">
          Failed to load config: {loadError}
        </div>
      {/if}

      <!-- Import Validation Mode -->
      <div class="space-y-2">
        <label class="text-xs text-neutral-400">Import Validation Mode</label>
        <select
          bind:value={validationMode}
          disabled={isLoading}
          class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500 disabled:opacity-50"
          style="color-scheme: dark;"
        >
          <option value="none" class="bg-neutral-900 text-neutral-200">
            None (Fastest)
          </option>
          <option value="import_resolve" class="bg-neutral-900 text-neutral-200">
            Import Resolution (Recommended)
          </option>
          <option value="vite_integration" class="bg-neutral-900 text-neutral-200">
            Vite Integration (Most Accurate)
          </option>
          <option value="esbuild_bundle" class="bg-neutral-900 text-neutral-200">
            esbuild Bundle (Thorough)
          </option>
        </select>
        <div class="text-[10px] text-neutral-600">
          {getModeDescription(validationMode)}
        </div>
      </div>

      <!-- Validation Timeout -->
      <div class="space-y-1">
        <label class="text-xs text-neutral-400">Validation Timeout (ms)</label>
        <div class="flex items-center gap-3">
          <input
            type="range"
            bind:value={timeoutMs}
            min="1000"
            max="30000"
            step="1000"
            disabled={validationMode === 'none'}
            class="flex-1 h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer disabled:opacity-50"
          />
          <span class="text-xs text-neutral-300 w-16 text-right">{timeoutMs}ms</span>
        </div>
        <div class="text-[10px] text-neutral-600">
          Maximum time to wait for validation script.
        </div>
      </div>

      <!-- Additional Allowed Packages -->
      <div class="space-y-1">
        <label class="text-xs text-neutral-400">Additional Allowed Packages</label>
        <textarea
          bind:value={allowedPackages}
          disabled={validationMode === 'none'}
          placeholder="package-name
@scope/package"
          rows="3"
          class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-neutral-500 disabled:opacity-50 font-mono"
          style="color-scheme: dark; resize: none;"
        ></textarea>
        <div class="text-[10px] text-neutral-600">
          One package per line. Added to package.json dependencies for validation.
        </div>
      </div>

      <!-- Save Button -->
      {#if hasChanges}
        <button
          onclick={saveConfig}
          disabled={isSaving}
          class="w-full py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-xs transition-colors"
        >
          {isSaving ? 'Saving...' : 'Save Sandbox Settings'}
        </button>
      {/if}

      <!-- Help text -->
      <div class="text-[10px] text-neutral-600 leading-relaxed">
        Import validation catches errors before they reach the main app.
        Settings apply to the next generated component.
      </div>
    </div>
  {:else}
    <!-- Collapsed summary -->
    <div class="text-xs text-neutral-500">
      <span class="text-neutral-400">{getModeLabel(validationMode)}</span>
      {#if validationMode !== 'none'}
        <span class="mx-1">|</span>
        <span class="text-neutral-400">{timeoutMs}ms timeout</span>
      {/if}
    </div>
  {/if}
</div>
