<script lang="ts">
  import {
    dependencyTokenLabel,
    type DependencyOverridePatchV1,
    type ModelDependencyBinding,
    type ModelDependencyRequirements,
    type StringOverrideField,
  } from './dependencyEnvironmentState';

  interface Props {
    requirements: ModelDependencyRequirements;
    bindings: ModelDependencyBinding[];
    mode: 'auto' | 'manual';
    isBusy: boolean;
    effectiveManualOverrides: DependencyOverridePatchV1[];
    upstreamManualOverrides: DependencyOverridePatchV1[];
    manualOverrides: DependencyOverridePatchV1[];
    bindingHasSelection: (bindingId: string) => boolean;
    bindingPatchCount: (bindingId: string) => number;
    bindingLocalPatchCount: (bindingId: string) => number;
    requirementPatchCount: (bindingId: string, requirementName: string) => number;
    hasRequirementLocalOverrides: (bindingId: string, requirementName: string) => boolean;
    hasBindingLocalOverride: (bindingId: string) => boolean;
    getStringOverrideField: (
      bindingId: string,
      scope: 'binding' | 'requirement',
      requirementName: string | undefined,
      field: StringOverrideField
    ) => string;
    getExtraIndexUrls: (bindingId: string, requirementName: string) => string;
    onClearAllOverrides: () => void;
    onToggleAllBindings: () => void;
    onToggleBinding: (bindingId: string) => void;
    onBindingPythonExecutableChange: (bindingId: string, event: Event) => void;
    onClearBindingPythonExecutable: (bindingId: string) => void;
    onClearBindingOverrides: (bindingId: string) => void;
    onRequirementFieldChange: (
      bindingId: string,
      requirementName: string,
      field: Exclude<StringOverrideField, 'python_executable'>,
      event: Event
    ) => void;
    onRequirementExtraUrlsChange: (bindingId: string, requirementName: string, event: Event) => void;
    onClearRequirementOverrides: (bindingId: string, requirementName: string) => void;
  }

  let {
    requirements,
    bindings,
    mode,
    isBusy,
    effectiveManualOverrides,
    upstreamManualOverrides,
    manualOverrides,
    bindingHasSelection,
    bindingPatchCount,
    bindingLocalPatchCount,
    requirementPatchCount,
    hasRequirementLocalOverrides,
    hasBindingLocalOverride,
    getStringOverrideField,
    getExtraIndexUrls,
    onClearAllOverrides,
    onToggleAllBindings,
    onToggleBinding,
    onBindingPythonExecutableChange,
    onClearBindingPythonExecutable,
    onClearBindingOverrides,
    onRequirementFieldChange,
    onRequirementExtraUrlsChange,
    onClearRequirementOverrides,
  }: Props = $props();
</script>

{#if mode === 'manual'}
  <div class="rounded border border-neutral-700 px-2 py-1 space-y-2">
    <div class="flex items-center gap-2">
      <span class="text-[10px] text-neutral-400">Structured Override Controls</span>
      <span class="ml-auto text-[10px] text-neutral-500">
        {effectiveManualOverrides.length} effective patch(es)
      </span>
    </div>
    <div class="text-[10px] text-neutral-500">
      Configure per binding and per requirement below. Overrides are applied immediately.
    </div>
    {#if upstreamManualOverrides.length > 0}
      <div class="text-[10px] text-amber-300">
        {upstreamManualOverrides.length} patch(es) are provided by connected input. Local form edits override conflicts.
      </div>
    {/if}
    <div class="flex items-center gap-2">
      <button
        type="button"
        class="text-[10px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
        onclick={onClearAllOverrides}
        disabled={isBusy || manualOverrides.length === 0}
      >
        Clear All Overrides
      </button>
      <button
        type="button"
        class="text-[10px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
        onclick={onToggleAllBindings}
        disabled={isBusy}
      >
        Toggle All Bindings
      </button>
    </div>
  </div>
{/if}

{#if requirements.bindings.length > 0}
  <div class="rounded border border-neutral-700 px-2 py-1 text-[10px] space-y-1">
    {#each bindings as binding (binding.binding_id)}
      <div class="rounded border border-neutral-800 px-2 py-1">
        <div class="flex items-center gap-2">
          <input
            type="checkbox"
            checked={bindingHasSelection(binding.binding_id)}
            disabled={isBusy}
            onchange={() => onToggleBinding(binding.binding_id)}
          />
          <span class="text-neutral-200 truncate" title={binding.binding_id}>{binding.binding_id}</span>
          {#if mode === 'manual' && bindingPatchCount(binding.binding_id) > 0}
            <span class="ml-auto text-[9px] text-cyan-300">
              {bindingPatchCount(binding.binding_id)} override patch(es)
            </span>
          {/if}
        </div>
        <div class="text-[9px] text-neutral-500 truncate" title={binding.profile_id + ' v' + binding.profile_version}>
          {binding.profile_id} v{binding.profile_version}
        </div>
        <div class="text-[9px] text-neutral-400">validation: {dependencyTokenLabel(binding.validation_state)}</div>
        {#if binding.requirements.length > 0}
          <div class="space-y-1 mt-1">
            {#if mode === 'manual'}
              <div class="rounded border border-cyan-900/40 bg-cyan-950/10 px-2 py-1 space-y-1">
                <div class="text-[10px] text-cyan-300">Binding Override</div>
                <label class="text-[9px] text-neutral-400">Python executable</label>
                <input
                  type="text"
                  class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                  value={getStringOverrideField(binding.binding_id, 'binding', undefined, 'python_executable')}
                  placeholder="/path/to/python or python3"
                  onchange={(event) => onBindingPythonExecutableChange(binding.binding_id, event)}
                />
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    class="text-[9px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                    onclick={() => onClearBindingPythonExecutable(binding.binding_id)}
                    disabled={isBusy || !hasBindingLocalOverride(binding.binding_id)}
                  >
                    Clear Local Binding Override
                  </button>
                  <button
                    type="button"
                    class="text-[9px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                    onclick={() => onClearBindingOverrides(binding.binding_id)}
                    disabled={isBusy || bindingLocalPatchCount(binding.binding_id) === 0}
                  >
                    Clear Local Binding Patches
                  </button>
                </div>
              </div>
            {/if}

            {#each binding.requirements as requirement (`${binding.binding_id}:${requirement.name}`)}
              <div class="rounded border border-neutral-800 px-2 py-1 space-y-1">
                <div class="text-[9px] text-neutral-200">
                  {requirement.name}{requirement.exact_pin}
                </div>
                {#if mode === 'manual'}
                  <div class="space-y-1">
                    <label class="text-[9px] text-neutral-400">Index URL</label>
                    <input
                      type="text"
                      class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                      value={getStringOverrideField(binding.binding_id, 'requirement', requirement.name, 'index_url')}
                      placeholder="https://..."
                      onchange={(event) => onRequirementFieldChange(binding.binding_id, requirement.name, 'index_url', event)}
                    />
                    <label class="text-[9px] text-neutral-400">Extra index URLs (comma-separated)</label>
                    <input
                      type="text"
                      class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                      value={getExtraIndexUrls(binding.binding_id, requirement.name)}
                      placeholder="https://a, https://b"
                      onchange={(event) => onRequirementExtraUrlsChange(binding.binding_id, requirement.name, event)}
                    />
                    <label class="text-[9px] text-neutral-400">Wheel source path</label>
                    <input
                      type="text"
                      class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                      value={getStringOverrideField(binding.binding_id, 'requirement', requirement.name, 'wheel_source_path')}
                      placeholder="/path/to/wheels"
                      onchange={(event) => onRequirementFieldChange(binding.binding_id, requirement.name, 'wheel_source_path', event)}
                    />
                    <label class="text-[9px] text-neutral-400">Package source override</label>
                    <input
                      type="text"
                      class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[10px] text-neutral-200 font-mono focus:outline-none focus:border-cyan-500"
                      value={getStringOverrideField(binding.binding_id, 'requirement', requirement.name, 'package_source_override')}
                      placeholder="custom source descriptor"
                      onchange={(event) =>
                        onRequirementFieldChange(binding.binding_id, requirement.name, 'package_source_override', event)}
                    />
                    <div class="flex items-center gap-2">
                      <button
                        type="button"
                        class="text-[9px] px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 hover:bg-neutral-700 disabled:opacity-50"
                        onclick={() => onClearRequirementOverrides(binding.binding_id, requirement.name)}
                        disabled={isBusy || !hasRequirementLocalOverrides(binding.binding_id, requirement.name)}
                      >
                        Clear Local Requirement Overrides
                      </button>
                      {#if requirementPatchCount(binding.binding_id, requirement.name) > 0}
                        <span class="text-[9px] text-cyan-300">
                          {requirementPatchCount(binding.binding_id, requirement.name)} patch(es)
                        </span>
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
{/if}
