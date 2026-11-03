import { derived, writable, type Readable } from 'svelte/store';

export const WORKBENCH_PAGE_IDS = [
  'sessions',
  'scheduler',
  'diagnostics',
  'io_inspector',
  'network',
  'graph',
  'node_lab',
  'library',
  'settings',
] as const;

export type WorkbenchPageId = (typeof WORKBENCH_PAGE_IDS)[number];

/**
 * Tab bar group. The shell separates groups with a divider and pins
 * `settings` to the right edge of the header.
 */
export type WorkbenchPageGroup = 'sessions' | 'runs' | 'workflows' | 'settings';

export interface WorkbenchPageDefinition {
  id: WorkbenchPageId;
  label: string;
  group: WorkbenchPageGroup;
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

export type SettingsFocusTargetId = 'diagnostics_retention';

export interface SettingsFocusTarget {
  target_id: SettingsFocusTargetId;
  requested_at_ms: number;
}

export interface WorkbenchState {
  selected_page_id: WorkbenchPageId;
  active_run: ActiveWorkflowRunContext | null;
  diagnostics_focus: DiagnosticsFocusTarget | null;
  settings_focus: SettingsFocusTarget | null;
}

export const WORKBENCH_PAGES: WorkbenchPageDefinition[] = [
  { id: 'sessions', label: 'Sessions', group: 'sessions' },
  { id: 'scheduler', label: 'Scheduler', group: 'runs' },
  { id: 'diagnostics', label: 'Diagnostics', group: 'runs' },
  { id: 'io_inspector', label: 'I/O Inspector', group: 'runs' },
  { id: 'network', label: 'Network', group: 'runs' },
  { id: 'graph', label: 'Graph', group: 'workflows' },
  { id: 'node_lab', label: 'Node Editor', group: 'workflows' },
  { id: 'library', label: 'Library', group: 'workflows' },
  { id: 'settings', label: 'Settings', group: 'settings' },
];

/** Tab groups in header order, left to right, excluding the pinned Settings tab. */
export const WORKBENCH_TAB_GROUPS: readonly WorkbenchPageGroup[] = ['sessions', 'runs', 'workflows'];

export function workbenchPagesInGroup(group: WorkbenchPageGroup): WorkbenchPageDefinition[] {
  return WORKBENCH_PAGES.filter((page) => page.group === group);
}

export const DEFAULT_WORKBENCH_STATE: WorkbenchState = {
  selected_page_id: 'scheduler',
  active_run: null,
  diagnostics_focus: null,
  settings_focus: null,
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

export function withSettingsFocus(
  state: WorkbenchState,
  targetId: SettingsFocusTargetId | null,
  requestedAtMs: number,
): WorkbenchState {
  return {
    ...state,
    selected_page_id: targetId ? 'settings' : state.selected_page_id,
    settings_focus: targetId
      ? {
          target_id: targetId,
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

export const settingsFocus: Readable<SettingsFocusTarget | null> = derived(
  workbenchStateStore,
  ($state) => $state.settings_focus,
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

export function focusSettingsSection(
  targetId: SettingsFocusTargetId,
  requestedAtMs = Date.now(),
): void {
  workbenchStateStore.update((state) => withSettingsFocus(state, targetId, requestedAtMs));
}

export function clearSettingsFocus(): void {
  workbenchStateStore.update((state) => withSettingsFocus(state, null, Date.now()));
}

export function resetWorkbenchState(): void {
  workbenchStateStore.set({ ...DEFAULT_WORKBENCH_STATE });
}
