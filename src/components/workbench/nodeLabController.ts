import { NodeLabService, type CustomNodeAuthoringEntry, type NodeLabDraft } from '../../services/workflow/customNodeAuthoring.ts';
import type { GraphNode } from '../../services/workflow/types.ts';

export interface NodeLabView {
  catalog: CustomNodeAuthoringEntry[] | null;
  draft: NodeLabDraft | null;
  savedPath: string;
  busy: boolean;
  error: string | null;
  notice: string;
  pendingCleanup: number;
}
/** One page owns one separate edit session and observes all in-flight completion. */
export class NodeLabController {
  private disposed = false;
  private operation: Promise<void> | null = null;
  private readonly ownedSessions = new Set<string>();
  cleanupError: unknown = null;
  readonly view: NodeLabView = { catalog: null, draft: null, savedPath: '', busy: false, error: null, notice: '', pendingCleanup: 0 };
  private readonly service: NodeLabService;
  private readonly changed: (view: NodeLabView) => void;
  constructor(service: NodeLabService, changed: (view: NodeLabView) => void) { this.service = service; this.changed = changed; this.view.pendingCleanup = service.pendingCleanupCount; }
  private publish(): void { if (!this.disposed) this.changed({ ...this.view }); }
  private perform(action: () => Promise<void>): Promise<void> {
    if (this.disposed || this.operation) return Promise.resolve();
    this.view.busy = true; this.view.error = null; this.view.notice = ''; this.publish();
    const operation = (async () => {
      try { await action(); }
      catch (error) { this.view.error = error instanceof Error ? error.message : 'NodeLab operation failed'; }
      finally { this.view.busy = false; this.operation = null; this.view.pendingCleanup = this.service.pendingCleanupCount; this.publish(); }
    })();
    this.operation = operation;
    return operation;
  }
  retryCleanup(): Promise<void> { return this.perform(() => this.service.retryCleanup()); }
  loadCatalog(): Promise<void> {
    return this.perform(async () => {
      this.view.catalog = null;
      this.publish();
      this.view.catalog = await this.service.catalog();
    });
  }
  private async replace(create: () => Promise<NodeLabDraft>): Promise<void> {
    const next = await create();
    this.ownedSessions.add(next.session_id);
    if (this.view.draft) {
      try {
        await this.service.close(this.view.draft.session_id);
        this.ownedSessions.delete(this.view.draft.session_id);
      } catch (error) {
        this.service.retainActiveSession(this.view.draft.session_id);
        try { await this.service.close(next.session_id); this.ownedSessions.delete(next.session_id); }
        catch (cleanupError) { this.cleanupError = cleanupError; throw new AggregateError([error, cleanupError], 'Draft replacement and cleanup failed; previous draft retained'); }
        throw error;
      }
    }
    this.view.draft = next;
    this.view.savedPath = '';
  }
  create(entry: CustomNodeAuthoringEntry): Promise<void> {
    return this.perform(() => this.replace(() => this.service.create(entry.identity)));
  }
  reopen(path: string): Promise<void> {
    return this.perform(async () => { await this.replace(() => this.service.reopen(path)); this.view.savedPath = path; });
  }
  update(node: GraphNode, text: string): Promise<void> {
    return this.perform(async () => {
      if (!this.view.draft) throw new Error('No NodeLab draft');
      this.view.draft = await this.service.update(this.view.draft.session_id, node, text);
      this.view.notice = 'Input saved in draft';
    });
  }
  save(name: string): Promise<void> {
    return this.perform(async () => {
      if (!this.view.draft) throw new Error('No NodeLab draft');
      const result = await this.service.save(this.view.draft.session_id, name);
      this.view.draft = result.draft; this.view.savedPath = result.path; this.view.notice = 'Workflow saved';
    });
  }
  async dispose(): Promise<void> {
    this.disposed = true;
    await this.operation;
    for (const id of this.ownedSessions) {
      try {
        await this.service.close(id); this.ownedSessions.delete(id);
        if (this.view.draft?.session_id === id) this.view.draft = null;
      } catch (error) { this.cleanupError = error; }
    }
  }
}
