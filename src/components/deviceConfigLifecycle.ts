import type { EmbeddingMemoryMode } from '../services/ConfigService';

export interface DeviceConfigLifecycleOptions {
  loadDevices(): Promise<void>;
  startRefresh(): () => void;
  loadEmbeddingMemoryMode(): Promise<EmbeddingMemoryMode>;
  applyEmbeddingMemoryMode(mode: EmbeddingMemoryMode): void;
  onFailure(error: unknown): void;
}

/** One mount owns initialization, late results, and the refresh cleanup. */
export function createDeviceConfigLifecycle(options: DeviceConfigLifecycleOptions) {
  let active = true;
  let initialization: Promise<void> | null = null;
  let stopRefresh: (() => void) | null = null;

  const initialize = async () => {
    if (!active) return;
    try {
      await options.loadDevices();
      if (!active) return;

      const cleanup = options.startRefresh();
      if (!active) {
        cleanup();
        return;
      }
      stopRefresh = cleanup;

      const mode = await options.loadEmbeddingMemoryMode();
      if (active) options.applyEmbeddingMemoryMode(mode);
    } catch (error) {
      if (active) options.onFailure(error);
    }
  };

  return {
    start(): Promise<void> {
      if (!active) return initialization ?? Promise.resolve();
      // Assign ownership before initialization can invoke any callbacks.
      initialization ??= Promise.resolve().then(initialize);
      return initialization;
    },
    stop(): void {
      active = false;
      stopRefresh?.();
      stopRefresh = null;
    },
    isActive(): boolean {
      return active;
    },
  };
}

export type DeviceConfigLifecycle = ReturnType<typeof createDeviceConfigLifecycle>;
