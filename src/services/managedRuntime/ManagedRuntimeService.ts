import { invoke, Channel } from '@tauri-apps/api/core';
import { Logger } from '../Logger';
import type {
  ManagedRuntimeId,
  ManagedRuntimeManagerRuntimeView,
  ManagedRuntimeProgress,
} from './types';

type ManagedRuntimeProgressListener = (
  progress: ManagedRuntimeProgress
) => void | Promise<void>;

class ManagedRuntimeServiceClass {
  public async listRuntimes(): Promise<ManagedRuntimeManagerRuntimeView[]> {
    return invoke<ManagedRuntimeManagerRuntimeView[]>('list_managed_runtimes');
  }

  public async inspectRuntime(
    runtimeId: ManagedRuntimeId
  ): Promise<ManagedRuntimeManagerRuntimeView> {
    return invoke<ManagedRuntimeManagerRuntimeView>('inspect_managed_runtime', {
      binaryId: runtimeId,
    });
  }

  public async installRuntime(
    runtimeId: ManagedRuntimeId,
    onProgress: ManagedRuntimeProgressListener
  ): Promise<void> {
    const channel = new Channel<ManagedRuntimeProgress>();
    channel.onmessage = (event) => {
      void onProgress(event);
    };

    try {
      await invoke('install_managed_runtime', {
        binaryId: runtimeId,
        channel,
      });
    } catch (error) {
      Logger.log(
        'MANAGED_RUNTIME_INSTALL_ERROR',
        { runtimeId, error: String(error) },
        'error'
      );
      throw error;
    }
  }

  public async removeRuntime(runtimeId: ManagedRuntimeId): Promise<void> {
    await invoke('remove_managed_runtime', { binaryId: runtimeId });
  }

  public async selectRuntimeVersion(
    runtimeId: ManagedRuntimeId,
    version: string | null
  ): Promise<ManagedRuntimeManagerRuntimeView> {
    return invoke<ManagedRuntimeManagerRuntimeView>(
      'select_managed_runtime_version',
      {
        binaryId: runtimeId,
        version,
      }
    );
  }

  public async setDefaultRuntimeVersion(
    runtimeId: ManagedRuntimeId,
    version: string | null
  ): Promise<ManagedRuntimeManagerRuntimeView> {
    return invoke<ManagedRuntimeManagerRuntimeView>(
      'set_default_managed_runtime_version',
      {
        binaryId: runtimeId,
        version,
      }
    );
  }
}

export const managedRuntimeService = new ManagedRuntimeServiceClass();
