// Minimal interface for the WASM network exposed by pkg/gnomics.js
// This matches the methods used in viewer_live.html. Extend as needed.

export interface IWasmNetwork {
  build(): void;
  rebuild(): void;
  num_blocks(): number;
  get_block_name(handle: number): string;
  get_block_type(handle: number): string;

  // Add blocks
  add_scalar_transformer(
    name: string,
    min: number,
    max: number,
    statelets: number,
    active: number,
    num_t: number,
    seed: number
  ): number;
  add_discrete_transformer(
    name: string,
    categories: number,
    statelets: number,
    num_t: number,
    seed: number
  ): number;
  add_persistence_transformer(
    name: string,
    min: number,
    max: number,
    statelets: number,
    active: number,
    num_t: number,
    seed: number
  ): number;
  add_pattern_pooler(
    name: string,
    statelets: number,
    active: number,
    perm_thr: number,
    seed: number
  ): number;
  add_pattern_classifier(
    name: string,
    num_l: number,
    statelets: number,
    active: number,
    perm_thr: number,
    seed: number
  ): number;
  add_sequence_learner(
    name: string,
    num_c: number,
    num_spc: number,
    num_dps: number,
    num_rpd: number,
    seed: number
  ): number;
  add_context_learner(
    name: string,
    num_c: number,
    num_spc: number,
    num_dps: number,
    num_rpd: number,
    seed: number
  ): number;

  // Connections
  connect_to_input(src: number, dst: number): void;
  connect_to_context(src: number, dst: number): void;
  remove_connection(src: number, dst: number, kind: number): void;

  // Execution
  execute(learn: boolean): void;
  get_trace_json(): string;

  // Config IO
  export_config(): string;
  import_config(json: string): void;

  // Block operations
  remove_block(handle: number): void;
}
