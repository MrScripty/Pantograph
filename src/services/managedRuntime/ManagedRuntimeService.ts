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

type ManagedRuntimeStateListener = (
  runtimes: ManagedRuntimeManagerRuntimeView[]
) => void;

class ManagedRuntimeServiceClass {
  private runtimes: ManagedRuntimeManagerRuntimeView[] = [];
  private listeners: ManagedRuntimeStateListener[] = [];

  public subscribe(callback: ManagedRuntimeStateListener): () => void {
    this.listeners.push(callback);
    callback(this.snapshotRuntimes());
    return () => {
      this.listeners = this.listeners.filter((listener) => listener !== callback);
    };
  }

  public getState(): ManagedRuntimeManagerRuntimeView[] {
    return this.snapshotRuntimes();
  }

  private notify(): void {
    const runtimes = this.snapshotRuntimes();
    this.listeners.forEach((listener) => listener(runtimes));
  }

  private snapshotRuntimes(): ManagedRuntimeManagerRuntimeView[] {
    return this.runtimes.map((runtime) => ({
      ...runtime,
      missing_files: [...runtime.missing_files],
      versions: runtime.versions.map((version) => ({ ...version })),
      selection: { ...runtime.selection },
      active_job: runtime.active_job ? { ...runtime.active_job } : null,
      job_artifact: runtime.job_artifact ? { ...runtime.job_artifact } : null,
      install_history: runtime.install_history.map((entry) => ({ ...entry })),
    }));
  }

  private setRuntimes(runtimes: ManagedRuntimeManagerRuntimeView[]): void {
    this.runtimes = runtimes.map((runtime) => ({
      ...runtime,
      missing_files: [...runtime.missing_files],
      versions: runtime.versions.map((version) => ({ ...version })),
      selection: { ...runtime.selection },
      active_job: runtime.active_job ? { ...runtime.active_job } : null,
      job_artifact: runtime.job_artifact ? { ...runtime.job_artifact } : null,
      install_history: runtime.install_history.map((entry) => ({ ...entry })),
    }));
    this.notify();
  }

  private upsertRuntime(runtime: ManagedRuntimeManagerRuntimeView): void {
    const nextRuntimes = this.runtimes.filter(
      (candidate) => candidate.id !== runtime.id
    );
    nextRuntimes.push(runtime);
    nextRuntimes.sort((left, right) =>
      left.display_name.localeCompare(right.display_name)
    );
    this.setRuntimes(nextRuntimes);
  }

  public async listRuntimes(): Promise<ManagedRuntimeManagerRuntimeView[]> {
    const runtimes = await invoke<ManagedRuntimeManagerRuntimeView[]>(
      'list_managed_runtimes'
    );
    this.setRuntimes(runtimes);
    return this.snapshotRuntimes();
  }

  public async inspectRuntime(
    runtimeId: ManagedRuntimeId
  ): Promise<ManagedRuntimeManagerRuntimeView> {
    const runtime = await invoke<ManagedRuntimeManagerRuntimeView>(
      'inspect_managed_runtime',
      {
        binaryId: runtimeId,
      }
    );
    this.upsertRuntime(runtime);
    return runtime;
  }

  public async installRuntime(
    runtimeId: ManagedRuntimeId,
    onProgress: ManagedRuntimeProgressListener
  ): Promise<void> {
    const channel = new Channel<ManagedRuntimeProgress>();
    channel.onmessage = (event) => {
      this.upsertRuntime(event.runtime);
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
    await this.listRuntimes();
  }

  public async cancelRuntimeJob(runtimeId: ManagedRuntimeId): Promise<void> {
    await invoke('cancel_managed_runtime_job', { binaryId: runtimeId });
    await this.inspectRuntime(runtimeId);
  }

  public async pauseRuntimeJob(runtimeId: ManagedRuntimeId): Promise<void> {
    await invoke('pause_managed_runtime_job', { binaryId: runtimeId });
    await this.inspectRuntime(runtimeId);
  }

  public async selectRuntimeVersion(
    runtimeId: ManagedRuntimeId,
    version: string | null
  ): Promise<ManagedRuntimeManagerRuntimeView> {
    const runtime = await invoke<ManagedRuntimeManagerRuntimeView>(
      'select_managed_runtime_version',
      {
        binaryId: runtimeId,
        version,
      }
    );
    this.upsertRuntime(runtime);
    return runtime;
  }

  public async setDefaultRuntimeVersion(
    runtimeId: ManagedRuntimeId,
    version: string | null
  ): Promise<ManagedRuntimeManagerRuntimeView> {
    const runtime = await invoke<ManagedRuntimeManagerRuntimeView>(
      'set_default_managed_runtime_version',
      {
        binaryId: runtimeId,
        version,
      }
    );
    this.upsertRuntime(runtime);
    return runtime;
  }
}

export const managedRuntimeService = new ManagedRuntimeServiceClass();
