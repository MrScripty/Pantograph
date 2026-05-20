export interface DeviceRefreshTimerApi {
  setInterval(callback: () => void, delayMs: number): unknown;
  clearInterval(handle: unknown): void;
}

export interface DeviceRefreshScope {
  update(active: boolean): void;
  stop(): void;
  isRunning(): boolean;
}

const DEFAULT_REFRESH_INTERVAL_MS = 3000;

export function createScopedDeviceRefresh(
  refresh: () => void,
  timerApi: DeviceRefreshTimerApi,
  intervalMs = DEFAULT_REFRESH_INTERVAL_MS,
): DeviceRefreshScope {
  let timerHandle: unknown = null;

  const stop = () => {
    if (timerHandle === null) return;
    timerApi.clearInterval(timerHandle);
    timerHandle = null;
  };

  const start = () => {
    if (timerHandle !== null) return;
    refresh();
    timerHandle = timerApi.setInterval(refresh, intervalMs);
  };

  return {
    update(active: boolean) {
      if (active) {
        start();
      } else {
        stop();
      }
    },
    stop,
    isRunning() {
      return timerHandle !== null;
    },
  };
}
