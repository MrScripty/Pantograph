import { derived, writable, type Readable } from 'svelte/store';

export const WORKBENCH_PAGE_IDS = [
  'scheduler',
  'diagnostics',
  'graph',
  'io_inspector',
  'library',
  'network',
  'node_lab',
] as const;

export type WorkbenchPageId = (typeof WORKBENCH_PAGE_IDS)[number];

export interface WorkbenchPageDefinition {
  id: WorkbenchPageId;
  label: string;
}

export interface ActiveWorkflowRunContext {
  workflow_run_id: string;
  workflow_id?: string | null;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  status?: string | null;
  selected_at_ms: number;
}

export interface WorkbenchState {
  selected_page_id: WorkbenchPageId;
  active_run: ActiveWorkflowRunContext | null;
}

export const WORKBENCH_PAGES: WorkbenchPageDefinition[] = [
  { id: 'scheduler', label: 'Scheduler' },
  { id: 'diagnostics', label: 'Diagnostics' },
  { id: 'graph', label: 'Graph' },
  { id: 'io_inspector', label: 'I/O Inspector' },
  { id: 'library', label: 'Library' },
  { id: 'network', label: 'Network' },
  { id: 'node_lab', label: 'Node Lab' },
];

export const DEFAULT_WORKBENCH_STATE: WorkbenchState = {
  selected_page_id: 'scheduler',
  active_run: null,
};

export function isWorkbenchPageId(value: string): value is WorkbenchPageId {
  return WORKBENCH_PAGE_IDS.includes(value as WorkbenchPageId);
}

export function normalizeWorkbenchPageId(value: string | null | undefined): WorkbenchPageId {
  return value && isWorkbenchPageId(value) ? value : DEFAULT_WORKBENCH_STATE.selected_page_id;
}

export function withSelectedWorkbenchPage(
  state: WorkbenchState,
  pageId: string,
): WorkbenchState {
  return {
    ...state,
    selected_page_id: normalizeWorkbenchPageId(pageId),
  };
}

export function withActiveWorkflowRun(
  state: WorkbenchState,
  run: Omit<ActiveWorkflowRunContext, 'selected_at_ms'> | null,
  selectedAtMs: number,
): WorkbenchState {
  return {
    ...state,
    active_run: run
      ? {
          workflow_run_id: run.workflow_run_id,
          workflow_id: run.workflow_id ?? null,
          workflow_version_id: run.workflow_version_id ?? null,
          workflow_semantic_version: run.workflow_semantic_version ?? null,
          status: run.status ?? null,
          selected_at_ms: selectedAtMs,
        }
      : null,
  };
}

const workbenchStateStore = writable<WorkbenchState>({ ...DEFAULT_WORKBENCH_STATE });

export const workbenchState: Readable<WorkbenchState> = {
  subscribe: workbenchStateStore.subscribe,
};

export const selectedWorkbenchPage: Readable<WorkbenchPageId> = derived(
  workbenchStateStore,
  ($state) => $state.selected_page_id,
);

export const activeWorkflowRun: Readable<ActiveWorkflowRunContext | null> = derived(
  workbenchStateStore,
  ($state) => $state.active_run,
);

export function setWorkbenchPage(pageId: string): void {
  workbenchStateStore.update((state) => withSelectedWorkbenchPage(state, pageId));
}

export function selectActiveWorkflowRun(
  run: Omit<ActiveWorkflowRunContext, 'selected_at_ms'>,
  selectedAtMs = Date.now(),
): void {
  workbenchStateStore.update((state) => withActiveWorkflowRun(state, run, selectedAtMs));
}

export function clearActiveWorkflowRun(): void {
  workbenchStateStore.update((state) => withActiveWorkflowRun(state, null, Date.now()));
}

export function resetWorkbenchState(): void {
  workbenchStateStore.set({ ...DEFAULT_WORKBENCH_STATE });
}
