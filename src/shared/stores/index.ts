/**
 * Shared Stores
 *
 * State utilities shared across features.
 */

// Panel state
export {
  sidePanelOpen,
  panelWidth,
  toggleSidePanel,
  openSidePanel,
  closeSidePanel,
} from '../../stores/panelStore';

// Interaction mode
export {
  interactionMode,
  toggleInteractionMode,
  setDrawMode,
  setInteractMode,
} from '../../stores/interactionModeStore';
export type { InteractionMode } from '../../stores/interactionModeStore';

// Prompt history
export { promptHistoryStore } from '../../stores/promptHistoryStore';

// Side panel tabs
export { activeSidePanelTab } from '../../stores/sidePanelTabStore';
export type { SidePanelTab } from '../../stores/sidePanelTabStore';
