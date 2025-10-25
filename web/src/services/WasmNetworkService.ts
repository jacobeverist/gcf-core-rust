// Typed wrapper around the WASM `WasmNetwork` exposed by ../../pkg/gnomics.js
// Centralizes initialization order and provides safe guards for calls.

import type { IWasmNetwork } from '../types/wasm';
// Import from pkg/ directory relative to src/services/
import init, { WasmNetwork } from '../../pkg/gnomics.js';

export class WasmNetworkService {
  private wasm?: IWasmNetwork;
  private initialized = false;

  async init(): Promise<void> {
    if (this.initialized) return;
    await init();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    this.wasm = new (WasmNetwork as any)();
    this.initialized = true;
  }

  isInitialized(): boolean { return this.initialized; }

  private ensure(): IWasmNetwork {
    if (!this.wasm) throw new Error('WASM not initialized');
    return this.wasm;
    }

  // Commonly used wrappers — add additional ones as needed
  build(): void { this.ensure().build(); }
  rebuild(): void { this.ensure().rebuild(); }
  numBlocks(): number { return this.ensure().num_blocks(); }

  addScalarTransformer(name: string, p: { min: number; max: number; statelets: number; active: number }, seed = 0): number {
    return this.ensure().add_scalar_transformer(name, p.min, p.max, p.statelets, p.active, 2, seed);
  }
  addDiscreteTransformer(name: string, p: { categories: number; statelets: number }, seed = 0): number {
    return this.ensure().add_discrete_transformer(name, p.categories, p.statelets, 2, seed);
  }
  addPersistenceTransformer(name: string, p: { min: number; max: number; statelets: number; active: number }, seed = 0): number {
    return this.ensure().add_persistence_transformer(name, p.min, p.max, p.statelets, p.active, 2, seed);
  }
  addPatternPooler(name: string, p: { statelets: number; active: number; perm_thr: number }, seed = 0): number {
    return this.ensure().add_pattern_pooler(name, p.statelets, p.active, p.perm_thr, seed);
  }
  addPatternClassifier(name: string, p: { num_l: number; statelets: number; active: number; perm_thr: number }, seed = 0): number {
    return this.ensure().add_pattern_classifier(name, p.num_l, p.statelets, p.active, p.perm_thr, seed);
  }
  addSequenceLearner(name: string, p: { num_c: number; num_spc: number; num_dps: number; num_rpd: number }, seed = 0): number {
    return this.ensure().add_sequence_learner(name, p.num_c, p.num_spc, p.num_dps, p.num_rpd, seed);
  }
  addContextLearner(name: string, p: { num_c: number; num_spc: number; num_dps: number; num_rpd: number }, seed = 0): number {
    return this.ensure().add_context_learner(name, p.num_c, p.num_spc, p.num_dps, p.num_rpd, seed);
  }

  connectInput(src: number, dst: number): void { this.ensure().connect_to_input(src, dst); }
  connectContext(src: number, dst: number): void { this.ensure().connect_to_context(src, dst); }
  removeConnection(src: number, dst: number, kind: number): void { this.ensure().remove_connection(src, dst, kind); }

  removeBlock(handle: number): void { this.ensure().remove_block(handle); }

  execute(learn: boolean): void { this.ensure().execute(learn); }
  getTraceJson(): string { return this.ensure().get_trace_json(); }

  exportConfig(): string { return this.ensure().export_config(); }
  importConfig(json: string): void { this.ensure().import_config(json); }
}
