export interface PendingArtifactDownload {
  id: number;
  objectUrl: string;
  filename: string;
}

interface DownloadEntry {
  request: PendingArtifactDownload;
  resolve?: () => void;
  reject?: (error: unknown) => void;
  timer?: number;
  activated: boolean;
}

interface DownloadLifecycleOptions {
  publish: (pending: PendingArtifactDownload[]) => void;
  revoke?: (objectUrl: string) => void;
  schedule?: (callback: () => void, delay: number) => number;
  cancel?: (timer: number) => void;
}

const REVOKE_DELAY_MS = 30_000;

/** Own each download URL from admission through click, delayed release, or disposal. */
export function createArtifactDownloadLifecycle(options: DownloadLifecycleOptions) {
  const {
    revoke = (objectUrl: string) => URL.revokeObjectURL(objectUrl),
    schedule = (callback: () => void, delay: number) => window.setTimeout(callback, delay),
    cancel = (timer: number) => window.clearTimeout(timer),
  } = options;
  let publishPending: DownloadLifecycleOptions['publish'] | undefined = options.publish;
  const entries = new Map<number, DownloadEntry>();
  let serial = 0;
  let disposed = false;

  function publish(): void {
    publishPending?.([...entries.values()].filter((entry) => !entry.activated).map((entry) => entry.request));
  }

  function release(entry: DownloadEntry): void {
    if (!entries.delete(entry.request.id)) return;
    if (entry.timer !== undefined) cancel(entry.timer);
    revoke(entry.request.objectUrl);
  }

  return {
    enqueue(objectUrl: string, filename: string): Promise<void> {
      if (disposed) {
        revoke(objectUrl);
        return Promise.reject(new Error('Artifact download owner is disposed.'));
      }
      return new Promise<void>((resolve, reject) => {
        const request = { id: ++serial, objectUrl, filename };
        entries.set(request.id, { request, resolve, reject, activated: false });
        publish();
      });
    },
    activate(id: number, click: () => void): void {
      const entry = entries.get(id);
      if (!entry || entry.activated || disposed) return;
      entry.activated = true;
      try {
        click();
        entry.timer = schedule(() => release(entry), REVOKE_DELAY_MS);
        entry.resolve?.();
      } catch (error) {
        release(entry);
        entry.reject?.(error);
      }
      entry.resolve = undefined;
      entry.reject = undefined;
      publish();
    },
    dispose(): void {
      if (disposed) return;
      disposed = true;
      for (const entry of entries.values()) {
        // A successful click keeps its grace timer even when the UI goes away.
        // In-progress activation finishes its success/error path after click returns.
        if (!entry.activated) {
          release(entry);
          entry.reject?.(new Error('Artifact download owner was disposed before activation.'));
          entry.resolve = undefined;
          entry.reject = undefined;
        }
      }
      publishPending?.([]);
      publishPending = undefined;
    },
  };
}
