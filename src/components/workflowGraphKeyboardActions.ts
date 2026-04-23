import {
  clearWorkflowContainerSelection,
  resolveWorkflowContainerKeyboardAction,
} from './workflowContainerSelection';
import {
  dispatchWorkflowHorseshoeKeyboardAction,
  isEditableKeyboardTarget,
  resolveWorkflowHorseshoeSelectionSnapshot,
  type HorseshoeDragSessionState,
  type HorseshoeInsertFeedbackState,
} from '@pantograph/svelte-graph';
import type { InsertableNodeTypeCandidate } from '../services/workflow/types';

interface ContainerKeyDownParams {
  containerSelected: boolean;
  event: KeyboardEvent;
  horseshoeDisplayState: HorseshoeDragSessionState['displayState'];
  onClearConnectionInteraction: () => void;
  onCloseHorseshoeSelector: () => void;
  onZoomToOrchestration: () => void;
}

interface WindowKeyDownParams {
  event: KeyboardEvent;
  feedback: HorseshoeInsertFeedbackState;
  items: InsertableNodeTypeCandidate[] | undefined;
  onClose: () => void;
  onConfirmSelection: (candidate: InsertableNodeTypeCandidate) => void;
  onQueryUpdate: (query: string) => void;
  onRequestOpen: () => void;
  onRotateSelection: (delta: number) => void;
  onTrace: (trace: string) => void;
  query: string;
  selectedIndex: number;
  session: HorseshoeDragSessionState;
}

export function handleWorkflowGraphContainerKeyDown({
  containerSelected,
  event,
  horseshoeDisplayState,
  onClearConnectionInteraction,
  onCloseHorseshoeSelector,
  onZoomToOrchestration,
}: ContainerKeyDownParams): boolean {
  const containerAction = resolveWorkflowContainerKeyboardAction({
    key: event.key,
    containerSelected,
  });

  let nextContainerSelected = containerSelected;
  if (containerAction.type === 'zoom-to-orchestration') {
    event.preventDefault();
    nextContainerSelected = clearWorkflowContainerSelection();
    onZoomToOrchestration();
    return nextContainerSelected;
  }

  if (containerAction.type === 'deselect-container') {
    event.preventDefault();
    nextContainerSelected = clearWorkflowContainerSelection();
  }

  if (isEditableKeyboardTarget(event.target as HTMLElement | null)) {
    return nextContainerSelected;
  }

  if (horseshoeDisplayState === 'hidden') {
    if (event.key === 'Escape') {
      onClearConnectionInteraction();
    }
    return nextContainerSelected;
  }

  if (event.key === 'Escape') {
    event.preventDefault();
    onCloseHorseshoeSelector();
  }

  return nextContainerSelected;
}

export function handleWorkflowGraphWindowKeyDown({
  event,
  feedback,
  items,
  onClose,
  onConfirmSelection,
  onQueryUpdate,
  onRequestOpen,
  onRotateSelection,
  onTrace,
  query,
  selectedIndex,
  session,
}: WindowKeyDownParams) {
  if (isEditableKeyboardTarget(event.target as HTMLElement | null)) {
    return;
  }

  const selection = resolveWorkflowHorseshoeSelectionSnapshot({
    session,
    feedback,
    items,
    selectedIndex,
  });
  dispatchWorkflowHorseshoeKeyboardAction({
    event,
    query,
    selection,
    handlers: {
      onClose,
      onConfirmSelection,
      onQueryUpdate,
      onRequestOpen,
      onRotateSelection,
      onTrace,
    },
  });
}
