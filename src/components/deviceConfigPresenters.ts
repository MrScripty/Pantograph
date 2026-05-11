import type { DeviceInfo } from '../services/ConfigService.ts';

export function buildBackendConfirmedDeviceOptions(devices: DeviceInfo[]): DeviceInfo[] {
  return devices.slice();
}

export function formatDeviceDisplayName(device: DeviceInfo): string {
  if (device.id === 'auto') return 'Auto';
  if (device.id === 'none') return device.name;
  const vram = formatVram(device.total_vram_mb);
  return vram ? `${device.name} (${vram})` : device.name;
}

export function resolveSelectedDeviceName(selectedDevice: string, devices: DeviceInfo[]): string {
  if (selectedDevice === 'auto') return 'Auto';
  const device = devices.find((candidate) => candidate.id === selectedDevice);
  return device?.name || selectedDevice;
}

function formatVram(mb: number): string {
  if (mb === 0) return '';
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} GB`;
  }
  return `${mb} MB`;
}
