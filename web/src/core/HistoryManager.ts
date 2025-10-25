import type { Operation } from '../types/domain';

// Manages undo/redo stacks and provides basic record/undo/redo mechanics
export class HistoryManager {
  undoStack: Operation[] = [];
  redoStack: Operation[] = [];
  maxSize = 50;

  record(op: Operation) {
    this.undoStack.push(op);
    if (this.undoStack.length > this.maxSize) this.undoStack.shift();
    this.redoStack = [];
  }

  canUndo() { return this.undoStack.length > 0; }
  canRedo() { return this.redoStack.length > 0; }

  popUndo(): Operation | undefined {
    return this.undoStack.pop();
  }

  pushRedo(op: Operation) { this.redoStack.push(op); }
  popRedo(): Operation | undefined { return this.redoStack.pop(); }

  clear() { this.undoStack = []; this.redoStack = []; }
}
