import type { BlockId } from '../types/domain';

// Centralized editor state, replacing global mutable variables from viewer_live.html
export class EditorStore {
  mode: 'select' | 'connect' | 'delete' = 'select';
  selectedBlocks = new Set<BlockId>();
  selectedConnections = new Set<string>();
  connectionDragStart: { blockId: BlockId; port: 'input' | 'context' } | null = null;
  clipboard: unknown = null;
  isDirty = false;
  nextBlockId = 1000; // UI-only ids if used for temporary elements

  resetSelections() {
    this.selectedBlocks.clear();
    this.selectedConnections.clear();
  }
}
