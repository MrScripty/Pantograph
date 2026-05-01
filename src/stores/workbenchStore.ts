import { derived, writable, type Readable } from 'svelte/store';

export const WORKBENCH_PAGE_IDS = [
  'scheduler',
  'diagnostics',
  'graph',
  'io_inspector',
  'library',
  'network',
  'node_lab',
  'settings',
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

export interface DiagnosticsFocusTarget {
  workflow_run_id: string;
  diagnostic_event_id?: string | null;
  node_id?: string | null;
  requested_at_ms: number;
}

export interface WorkbenchState {
  selected_page_id: WorkbenchPageId;
  active_run: ActiveWorkflowRunContext | null;
  diagnostics_focus: DiagnosticsFocusTarget | null;
}

export const WORKBENCH_PAGES: WorkbenchPageDefinition[] = [
  { id: 'scheduler', label: 'Scheduler' },
  { id: 'diagnostics', label: 'Diagnostics' },
  { id: 'graph', label: 'Graph' },
  { id: 'io_inspector', label: 'I/O Inspector' },
  { id: 'library', label: 'Library' },
  { id: 'network', label: 'Network' },
  { id: 'node_lab', label: 'Node Editor' },
  { id: 'settings', label: 'Settings' },
];

export const DEFAULT_WORKBENCH_STATE: WorkbenchState = {
  selected_page_id: 'scheduler',
  active_run: null,
  diagnostics_focus: null,
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

export function withDiagnosticsFocus(
  state: WorkbenchState,
  focus: Omit<DiagnosticsFocusTarget, 'requested_at_ms'> | null,
  requestedAtMs: number,
): WorkbenchState {
  return {
    ...state,
    selected_page_id: focus ? 'diagnostics' : state.selected_page_id,
    diagnostics_focus: focus
      ? {
          workflow_run_id: focus.workflow_run_id,
          diagnostic_event_id: focus.diagnostic_event_id ?? null,
          node_id: focus.node_id ?? null,
          requested_at_ms: requestedAtMs,
        }
      : null,
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
    diagnostics_focus:
      run && state.diagnostics_focus?.workflow_run_id === run.workflow_run_id
        ? state.diagnostics_focus
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

export const diagnosticsFocus: Readable<DiagnosticsFocusTarget | null> = derived(
  workbenchStateStore,
  ($state) => $state.diagnostics_focus,
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

export function focusWorkflowDiagnostics(
  run: Omit<ActiveWorkflowRunContext, 'selected_at_ms'>,
  focus: Omit<DiagnosticsFocusTarget, 'workflow_run_id' | 'requested_at_ms'> = {},
  selectedAtMs = Date.now(),
): void {
  workbenchStateStore.update((state) =>
    withDiagnosticsFocus(
      withActiveWorkflowRun(state, run, selectedAtMs),
      {
        workflow_run_id: run.workflow_run_id,
        diagnostic_event_id: focus.diagnostic_event_id ?? null,
        node_id: focus.node_id ?? null,
      },
      selectedAtMs,
    ),
  );
}

export function clearDiagnosticsFocus(): void {
  workbenchStateStore.update((state) => withDiagnosticsFocus(state, null, Date.now()));
}

export function resetWorkbenchState(): void {
  workbenchStateStore.set({ ...DEFAULT_WORKBENCH_STATE });
}
