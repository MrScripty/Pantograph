import type { PortDefinition } from '../../../services/workflow/types';

export interface SelectionInputOption {
  label: string;
  value: unknown;
}

export interface SelectionInputState {
  isProviderBacked: boolean;
  selectedString: string;
  displayValue: string;
  hasSelectedOption: boolean;
  placeholderLabel: string | null;
}

export interface SelectionAutoUpdate {
  shouldUpdate: boolean;
  value?: unknown;
}

export function buildSelectionInputState(
  targetPort: PortDefinition | null,
  options: SelectionInputOption[],
  currentValue: unknown,
): SelectionInputState {
  const selectedString = stringifySelectionValue(currentValue);
  const hasSelectedOption = options.some((option) => stringifySelectionValue(option.value) === selectedString);
  const isProviderBacked = Boolean(targetPort?.options_provider);
  const needsPlaceholder = isProviderBacked && !hasSelectedOption;

  return {
    isProviderBacked,
    selectedString,
    displayValue: needsPlaceholder ? '' : selectedString,
    hasSelectedOption,
    placeholderLabel: needsPlaceholder
      ? currentValue === null || currentValue === undefined
        ? 'Unset'
        : 'Stale selection'
      : null,
  };
}

export function resolveSelectionAutoUpdate(
  targetPort: PortDefinition | null,
  options: SelectionInputOption[],
  currentValue: unknown,
  defaultValue: unknown,
): SelectionAutoUpdate {
  if (targetPort?.options_provider || options.length === 0) {
    return { shouldUpdate: false };
  }

  const optionValues = options.map((option) => option.value);
  const hasCurrent = optionValues.some(
    (value) => stringifySelectionValue(value) === stringifySelectionValue(currentValue),
  );
  if (hasCurrent) {
    return { shouldUpdate: false };
  }

  const nextValue = optionValues.some(
    (value) => stringifySelectionValue(value) === stringifySelectionValue(defaultValue),
  )
    ? defaultValue
    : options[0]?.value;

  return {
    shouldUpdate: true,
    value: nextValue ?? null,
  };
}

export function stringifySelectionValue(value: unknown): string {
  if (value === null || value === undefined) {
    return '';
  }
  return JSON.stringify(value);
}
