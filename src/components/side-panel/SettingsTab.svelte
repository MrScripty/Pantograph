<script lang="ts">
  import ServerStatus from '../ServerStatus.svelte';
  import ModelConfig from '../ModelConfig.svelte';
  import DeviceConfig from '../DeviceConfig.svelte';
  import RagStatus from '../RagStatus.svelte';
  import SandboxSettings from '../SandboxSettings.svelte';
</script>

<div class="flex-1 overflow-y-auto">
  <!-- Server Status (includes Backend Selector) -->
  <div class="px-4 py-3 border-b border-neutral-700">
    <ServerStatus />
  </div>

  <!-- Model Configuration -->
  <div class="px-4 py-3 border-b border-neutral-700">
    <ModelConfig />
  </div>

  <!-- Device Configuration -->
  <div class="px-4 py-3 border-b border-neutral-700">
    <DeviceConfig />
  </div>

  <!-- RAG Status Panel -->
  <div class="px-4 py-3 border-b border-neutral-700">
    <RagStatus />
  </div>

  <!-- Sandbox Settings Panel -->
  <div class="px-4 py-3 border-b border-neutral-700">
    <SandboxSettings />
  </div>
</div>
