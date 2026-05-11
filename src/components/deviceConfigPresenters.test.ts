import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildBackendConfirmedDeviceOptions,
  formatDeviceDisplayName,
  isBackendConfirmedDeviceSelection,
  resolveSelectedDeviceName,
} from './deviceConfigPresenters.ts';
import type { DeviceInfo } from '../services/ConfigService.ts';

test('device options preserve only backend-confirmed devices', () => {
  const backendDevices: DeviceInfo[] = [
    { id: 'CUDA0', name: 'NVIDIA GPU', total_vram_mb: 12_288, free_vram_mb: 8192 },
  ];

  assert.deepEqual(buildBackendConfirmedDeviceOptions(backendDevices), backendDevices);
  assert.deepEqual(buildBackendConfirmedDeviceOptions([]), []);
});

test('device selection validation requires backend-confirmed devices', () => {
  const backendDevices: DeviceInfo[] = [
    { id: 'CUDA0', name: 'NVIDIA GPU', total_vram_mb: 12_288, free_vram_mb: 8192 },
  ];

  assert.equal(isBackendConfirmedDeviceSelection('CUDA0', backendDevices), true);
  assert.equal(isBackendConfirmedDeviceSelection('auto', backendDevices), false);
  assert.equal(isBackendConfirmedDeviceSelection('CUDA0', []), false);
});

test('device labels do not imply frontend-owned auto selection', () => {
  assert.equal(
    formatDeviceDisplayName({
      id: 'auto',
      name: 'Auto (backend policy)',
      total_vram_mb: 0,
      free_vram_mb: 0,
    }),
    'Auto',
  );
  assert.equal(
    formatDeviceDisplayName({
      id: 'CUDA0',
      name: 'NVIDIA GPU',
      total_vram_mb: 12_288,
      free_vram_mb: 8192,
    }),
    'NVIDIA GPU (12.0 GB)',
  );
});

test('selected device label uses backend facts when available', () => {
  const devices: DeviceInfo[] = [
    { id: 'CUDA0', name: 'NVIDIA GPU', total_vram_mb: 12_288, free_vram_mb: 8192 },
  ];

  assert.equal(resolveSelectedDeviceName('CUDA0', devices), 'NVIDIA GPU');
  assert.equal(resolveSelectedDeviceName('auto', devices), 'Auto');
  assert.equal(resolveSelectedDeviceName('Metal0', devices), 'Metal0');
});
