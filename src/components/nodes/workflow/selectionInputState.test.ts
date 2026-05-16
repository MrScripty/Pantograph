import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildSelectionInputState,
  resolveSelectionAutoUpdate,
  type SelectionInputOption,
} from './selectionInputState.ts';
import type { PortDefinition } from '../../../services/workflow/types.ts';

function port(overrides: Partial<PortDefinition> = {}): PortDefinition {
  return {
    id: 'denoising_scheduler',
    label: 'Denoising Scheduler',
    data_type: 'string',
    required: false,
    multiple: false,
    ...overrides,
  };
}

const options: SelectionInputOption[] = [
  { label: 'Euler', value: 'euler' },
  { label: 'DPM++', value: 'dpmpp' },
];

test('resolveSelectionAutoUpdate keeps static allowed-values behavior', () => {
  assert.deepEqual(resolveSelectionAutoUpdate(port({ default_value: 'dpmpp' }), options, null, 'dpmpp'), {
    shouldUpdate: true,
    value: 'dpmpp',
  });
  assert.deepEqual(resolveSelectionAutoUpdate(port(), options, 'euler', null), {
    shouldUpdate: false,
  });
});

test('resolveSelectionAutoUpdate does not write defaults for provider-backed ports', () => {
  assert.deepEqual(
    resolveSelectionAutoUpdate(
      port({
        options_provider: {
          node_type: 'llm-inference',
          port_id: 'denoising_scheduler',
        },
      }),
      options,
      null,
      'euler',
    ),
    { shouldUpdate: false },
  );
});

test('buildSelectionInputState renders provider-backed unset and stale states explicitly', () => {
  const providerPort = port({
    options_provider: {
      node_type: 'llm-inference',
      port_id: 'denoising_scheduler',
    },
  });

  assert.deepEqual(buildSelectionInputState(providerPort, options, null), {
    isProviderBacked: true,
    selectedString: '',
    displayValue: '',
    hasSelectedOption: false,
    placeholderLabel: 'Unset',
  });
  assert.deepEqual(buildSelectionInputState(providerPort, options, 'legacy-scheduler'), {
    isProviderBacked: true,
    selectedString: '"legacy-scheduler"',
    displayValue: '',
    hasSelectedOption: false,
    placeholderLabel: 'Stale selection',
  });
});

test('buildSelectionInputState keeps disabled provider rows visible when selected', () => {
  const providerPort = port({
    options_provider: {
      node_type: 'llm-inference',
      port_id: 'denoising_scheduler',
    },
  });

  assert.deepEqual(
    buildSelectionInputState(
      providerPort,
      [
        {
          label: 'DPM++',
          value: 'dpmpp',
          disabled: true,
          unavailableState: 'requires_runtime_capability',
          unavailableReason: 'Selected runtime does not expose this scheduler',
        },
      ],
      'dpmpp',
    ),
    {
      isProviderBacked: true,
      selectedString: '"dpmpp"',
      displayValue: '"dpmpp"',
      hasSelectedOption: true,
      placeholderLabel: null,
    },
  );
});
