// Shared domain types for the visual editor and network operations

export type BlockType =
  | 'ScalarTransformer'
  | 'DiscreteTransformer'
  | 'PersistenceTransformer'
  | 'PatternPooler'
  | 'PatternClassifier'
  | 'SequenceLearner'
  | 'ContextLearner';

export type BlockId = number; // WASM handle

export type BlockParams = Record<string, number | string | boolean>;

export interface Connection {
  source: BlockId;
  target: BlockId;
  kind: 'input' | 'context';
}

export type Operation =
  | { type: 'AddBlock'; blockType: BlockType; params: BlockParams; x: number; y: number; handle?: BlockId }
  | { type: 'RemoveBlock'; handle: BlockId }
  | { type: 'Connect'; source: BlockId; target: BlockId; kind: Connection['kind'] }
  | { type: 'Disconnect'; source: BlockId; target: BlockId; kind: Connection['kind'] }
  | { type: 'MoveBlock'; handle: BlockId; from: { x: number; y: number }; to: { x: number; y: number } };
