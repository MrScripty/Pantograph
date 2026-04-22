import type {
  ModelDependencyBinding,
  ModelDependencyRequirements,
} from './dependencyEnvironmentTypes.ts';

export function filterDependencyEnvironmentBindings(
  requirements: ModelDependencyRequirements,
  selectedBindingIds: string[],
): ModelDependencyBinding[] {
  if (selectedBindingIds.length === 0) return requirements.bindings;
  return requirements.bindings.filter((binding) => selectedBindingIds.includes(binding.binding_id));
}

export function isDependencyEnvironmentBindingSelected(
  selectedBindingIds: string[],
  bindingId: string,
): boolean {
  if (selectedBindingIds.length === 0) return true;
  return selectedBindingIds.includes(bindingId);
}

export function toggleDependencyEnvironmentBindingSelection(
  selectedBindingIds: string[],
  bindingId: string,
): string[] {
  if (selectedBindingIds.includes(bindingId)) {
    return selectedBindingIds.filter((id) => id !== bindingId);
  }

  return [...selectedBindingIds, bindingId];
}

export function toggleDependencyEnvironmentAllBindings(
  requirements: ModelDependencyRequirements,
  selectedBindingIds: string[],
): string[] {
  if (selectedBindingIds.length === requirements.bindings.length) {
    return [];
  }

  return requirements.bindings.map((binding) => binding.binding_id);
}
