import { Logger } from '../services/Logger';
import { createUndoStore } from './createUndoStore.ts';

export type { UndoableAction, UndoEntry } from './undoPersistence.ts';

export const undoStore = createUndoStore({
  storage: () => localStorage,
  log: (type, payload, severity) => Logger.log(type, payload, severity),
});
