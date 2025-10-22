# Parallel Execution & Scalability Analysis

**Date**: October 22, 2025
**Context**: Network architecture for 100s-1000s of blocks
**Focus**: Parallel execution strategies and scalability

---

## Parallel Execution Options

### Option 1: Layer-Based Parallelization (Recommended)

Group blocks by dependency depth and execute each layer in parallel:

```rust
pub struct ExecutionPlan {
    layers: Vec<Vec<BlockId>>,  // Blocks grouped by dependency depth
}

impl ExecutionPlan {
    /// Build execution layers from dependency graph
    pub fn from_graph(graph: &DependencyGraph) -> Self {
        let mut layers = Vec::new();
        let mut remaining: HashSet<_> = graph.all_nodes().collect();

        while !remaining.is_empty() {
            // Find all nodes with no dependencies in remaining set
            let layer: Vec<_> = remaining.iter()
                .filter(|&node| {
                    graph.dependencies(node)
                        .all(|dep| !remaining.contains(&dep))
                })
                .copied()
                .collect();

            if layer.is_empty() {
                panic!("Cycle detected");
            }

            for &node in &layer {
                remaining.remove(&node);
            }

            layers.push(layer);
        }

        ExecutionPlan { layers }
    }

    /// Execute all layers with intra-layer parallelism
    pub fn execute_parallel(&mut self, blocks: &mut HashMap<BlockId, Box<dyn Block>>,
                           learn: bool) -> Result<()> {
        use rayon::prelude::*;

        for layer in &self.layers {
            // All blocks in a layer can execute in parallel
            layer.par_iter().try_for_each(|&block_id| {
                // PROBLEM: Can't mutably borrow multiple blocks from HashMap
                // Need different data structure
            })?;
        }
        Ok(())
    }
}
```

**Visualization**:
```
Layer 0: [Encoder1, Encoder2, Encoder3]  ← Execute in parallel
         ↓         ↓         ↓
Layer 1: [Pooler1, Pooler2]               ← Execute in parallel
         ↓         ↓
Layer 2: [Classifier]                      ← Execute sequentially
```

**Pros**:
- ✅ Maximizes parallelism within dependency constraints
- ✅ Simple to reason about
- ✅ No race conditions (layers are natural barriers)
- ✅ Scales well with wide networks

**Cons**:
- ❌ Requires mutable access to multiple blocks simultaneously
- ❌ Synchronization overhead between layers
- ❌ Poor parallelism for deep, narrow networks

**Scalability**: **Excellent** for wide networks (many independent branches)

---

### Option 2: Thread Pool with Work Stealing (Rayon)

Use Rayon's work-stealing scheduler with fine-grained tasks:

```rust
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

pub struct Network {
    blocks: Vec<Arc<Mutex<dyn Block>>>,  // Thread-safe blocks
    execution_order: Vec<BlockId>,
    dependency_graph: DependencyGraph,
}

impl Network {
    pub fn execute_parallel(&self, learn: bool) -> Result<()> {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let completed = AtomicUsize::new(0);
        let total = self.blocks.len();

        // Track completion count for each block
        let completion_count: Vec<_> = self.blocks.iter()
            .map(|_| AtomicUsize::new(0))
            .collect();

        // Execute blocks as dependencies complete
        self.execution_order.par_iter().for_each(|&block_id| {
            // Wait for dependencies
            while !self.dependencies_ready(block_id, &completion_count) {
                std::thread::yield_now();
            }

            // Execute block
            let block = self.blocks[block_id.0 as usize].lock().unwrap();
            block.execute(learn).unwrap();

            // Mark as completed
            completion_count[block_id.0 as usize].store(1, Ordering::Release);
            completed.fetch_add(1, Ordering::SeqCst);
        });

        Ok(())
    }

    fn dependencies_ready(&self, block: BlockId,
                         completed: &[AtomicUsize]) -> bool {
        self.dependency_graph.dependencies(block)
            .all(|dep| completed[dep.0 as usize].load(Ordering::Acquire) == 1)
    }
}
```

**Pros**:
- ✅ Dynamic load balancing via work stealing
- ✅ Efficient use of thread pool
- ✅ Handles irregular dependency patterns

**Cons**:
- ❌ Mutex overhead on every block execution
- ❌ Busy-waiting for dependencies (CPU waste)
- ❌ Complex to debug
- ❌ Potential deadlocks with circular dependencies

**Scalability**: **Good** for mixed workloads, but Mutex overhead grows

---

### Option 3: Arena Allocation for Parallel Block Access

Use indices instead of pointers to enable safe parallel mutation:

```rust
pub struct BlockArena {
    blocks: Vec<Box<dyn Block>>,
}

impl BlockArena {
    /// Get multiple mutable references using disjoint indices
    pub fn execute_layer_parallel(&mut self, layer: &[usize], learn: bool) -> Result<()> {
        use rayon::prelude::*;

        // SAFETY: All indices in layer are unique and valid
        unsafe {
            layer.par_iter().try_for_each(|&idx| {
                let block = &mut *(self.blocks[idx].as_mut() as *mut dyn Block);
                block.execute(learn)
            })
        }
    }
}
```

**Pros**:
- ✅ No locking overhead
- ✅ Direct mutable access
- ✅ Cache-friendly (blocks in contiguous memory)

**Cons**:
- ❌ Requires `unsafe` code
- ❌ Must prove indices are disjoint
- ❌ More complex memory management

**Scalability**: **Excellent** - minimal overhead

---

### Option 4: Message Passing (Actor Model)

Each block runs in its own thread/task, communicating via channels:

```rust
use tokio::sync::mpsc;

pub enum BlockMessage {
    Execute { learn: bool, respond_to: mpsc::Sender<BlockResult> },
    SetValue(f64),
    GetOutput(mpsc::Sender<BitField>),
    Shutdown,
}

pub struct BlockActor {
    block: Box<dyn Block>,
    receiver: mpsc::Receiver<BlockMessage>,
    downstream: Vec<mpsc::Sender<BlockMessage>>,
}

impl BlockActor {
    pub async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                BlockMessage::Execute { learn, respond_to } => {
                    let result = self.block.execute(learn);
                    respond_to.send(result).await.unwrap();

                    // Notify downstream blocks
                    for downstream in &self.downstream {
                        downstream.send(BlockMessage::Execute {
                            learn,
                            respond_to: respond_to.clone()
                        }).await.unwrap();
                    }
                }
                BlockMessage::Shutdown => break,
                _ => {}
            }
        }
    }
}
```

**Pros**:
- ✅ No shared mutable state
- ✅ Natural async execution
- ✅ Easy to distribute across machines

**Cons**:
- ❌ High overhead (channel communication)
- ❌ Complex orchestration
- ❌ Memory overhead (one thread/task per block)
- ❌ Overkill for in-process execution

**Scalability**: **Poor** for 1000s of blocks (too many threads)

---

### Option 5: GPU Acceleration (CUDA/OpenCL)

Offload BitField operations to GPU:

```rust
use ocl::{ProQue, Buffer};

pub struct GpuBitField {
    words: Buffer<u32>,
    num_bits: usize,
}

impl GpuBitField {
    /// Parallel bit counting on GPU
    pub fn num_set_gpu(&self, queue: &ProQue) -> Result<usize> {
        let kernel = queue.kernel_builder("popcount")
            .arg(&self.words)
            .build()?;

        kernel.enq()?;

        // Reduce results on GPU
        // ...
    }

    /// Parallel bitwise AND on GPU
    pub fn and_gpu(&self, other: &GpuBitField, queue: &ProQue) -> Result<GpuBitField> {
        let kernel = queue.kernel_builder("bitwise_and")
            .arg(&self.words)
            .arg(&other.words)
            .build()?;

        kernel.enq()?;
        // ...
    }
}
```

**Pros**:
- ✅ Massive parallelism (1000s of cores)
- ✅ Fast for large BitFields (>10k bits)
- ✅ Batch operations across multiple blocks

**Cons**:
- ❌ PCIe transfer overhead
- ❌ Complex programming model
- ❌ Not worth it for small blocks
- ❌ Platform dependencies

**Scalability**: **Excellent** for large blocks, **poor** for small blocks

---

## Scalability Analysis: 100s-1000s of Blocks

### Memory Overhead

**Per-Block Overhead**:
```rust
// Minimum overhead per block
struct BlockOverhead {
    vtable_ptr: usize,      // 8 bytes
    block_base: BlockBase,  // ~40 bytes (id, init_flag, rng)
    // Block-specific fields vary
}

// With Network tracking:
struct NetworkOverhead {
    block_id: BlockId,           // 4 bytes
    hash_entry: (u64, *mut),     // 16 bytes
    execution_order_entry: u32,  // 4 bytes
}

// Total per block: ~72 bytes + block data
```

**For 1000 blocks**:
- Network overhead: ~72 KB
- Block data (varies by type):
  - ScalarTransformer: ~20 KB each
  - PatternPooler: ~200 KB each
  - Total: Depends on block mix

**Memory is NOT the bottleneck** for 1000s of blocks.

---

### Execution Overhead

**Sequential Execution (Current)**:
```
For 1000 blocks:
- Function call overhead: ~2-5ns per block
- Virtual dispatch: ~1-2ns per block
- Total overhead: ~3-7µs (negligible)
```

**Parallel Execution (Layer-based)**:
```
For 1000 blocks in 10 layers (100 blocks/layer):
- Thread spawn overhead: ~1-5µs per layer
- Synchronization overhead: ~0.5-2µs per layer
- Total overhead: ~15-70µs

Speedup if blocks take >1ms each:
- Sequential: 1000 × 1ms = 1000ms
- Parallel (10 layers): 10 × 1ms + 70µs ≈ 10ms
- Speedup: ~100× (ideal case)
```

**Execution overhead is acceptable** with proper parallelization.

---

### Dependency Graph Overhead

**Topological Sort**:
```rust
// Kahn's algorithm: O(V + E)
// V = number of blocks
// E = number of connections

For 1000 blocks, 3000 connections:
- Build graph: ~10-20µs
- Topological sort: ~30-50µs
- Total: ~40-70µs (one-time cost)
```

**Graph traversal is NOT a bottleneck**.

---

### Synchronization Overhead

**Layer-based synchronization**:
```
For 10 layers:
- Barrier sync per layer: ~0.5-2µs
- Total: ~5-20µs per execute() call

Percentage overhead:
- If blocks take 1ms each: 0.5-2%
- If blocks take 10µs each: 50-200% (BAD!)
```

**Conclusion**: Parallelization only helps if blocks take **>100µs** each.

---

### Real-World Scalability Estimates

#### Scenario 1: Wide Network (Good for Parallelism)
```
100 encoders → 50 poolers → 10 classifiers

Layers: 3
Parallelism within layers: ~100 / ~50 / ~10
Sequential time: 160 blocks × 100µs = 16ms
Parallel time: 3 × 100µs + 6µs sync = 306µs
Speedup: ~52×
```

#### Scenario 2: Deep Network (Poor for Parallelism)
```
Encoder → Pooler → Classifier → ... (100 stages)

Layers: 100
Parallelism: 1 per layer
Sequential time: 100 blocks × 100µs = 10ms
Parallel time: 100 × 100µs + 100×2µs sync = 10.2ms
Speedup: ~0.98× (WORSE due to overhead!)
```

#### Scenario 3: Mixed Network (Realistic)
```
10 input encoders
  ↓
30 feature poolers (3 per encoder)
  ↓
10 high-level poolers (3 poolers → 1)
  ↓
3 classifiers

Layers: 4
Width: 10 → 30 → 10 → 3
Sequential: 53 blocks × 50µs = 2.65ms
Parallel: 4 × 50µs + 8µs = 208µs
Speedup: ~12.7×
```

---

## Recommended Architecture for Scalability

### Hybrid Approach: Layer + Data Parallelism

```rust
pub struct ScalableNetwork {
    // Layer-based execution plan
    layers: Vec<Vec<BlockId>>,

    // Block storage (arena allocation)
    blocks: Vec<Box<dyn Block>>,

    // Parallel execution config
    config: ParallelConfig,
}

pub struct ParallelConfig {
    /// Minimum blocks per layer to enable parallelism
    min_parallel_blocks: usize,  // Default: 4

    /// Minimum expected block execution time
    min_block_time_us: u64,  // Default: 50µs

    /// Thread pool size
    num_threads: usize,  // Default: num_cpus
}

impl ScalableNetwork {
    pub fn execute(&mut self, learn: bool) -> Result<()> {
        for layer in &self.layers {
            if self.should_parallelize(layer) {
                self.execute_layer_parallel(layer, learn)?;
            } else {
                self.execute_layer_sequential(layer, learn)?;
            }
        }
        Ok(())
    }

    fn should_parallelize(&self, layer: &[BlockId]) -> bool {
        layer.len() >= self.config.min_parallel_blocks
    }

    fn execute_layer_parallel(&mut self, layer: &[BlockId], learn: bool) -> Result<()> {
        use rayon::prelude::*;

        // Use rayon's scoped threads for safe parallel mutation
        rayon::scope(|s| {
            for &block_id in layer {
                s.spawn(|_| {
                    // SAFETY: Each block_id is unique within layer
                    unsafe {
                        let block = &mut *(self.blocks[block_id.0 as usize].as_mut()
                                          as *mut dyn Block);
                        block.execute(learn).unwrap();
                    }
                });
            }
        });

        Ok(())
    }

    fn execute_layer_sequential(&mut self, layer: &[BlockId], learn: bool) -> Result<()> {
        for &block_id in layer {
            self.blocks[block_id.0 as usize].execute(learn)?;
        }
        Ok(())
    }
}
```

---

## Optimization Strategies for 1000+ Blocks

### 1. Block Fusion

Combine multiple small blocks into larger fused kernels:

```rust
// Instead of:
encoder1 → pooler1
encoder2 → pooler2
encoder3 → pooler3

// Fuse into:
BatchEncoder([encoder1, encoder2, encoder3])
  → BatchPooler([pooler1, pooler2, pooler3])
```

**Benefit**: Reduces function call overhead, better cache locality.

---

### 2. Lazy Copying Optimization (Already Implemented!)

Your existing lazy copying is **critical** for scalability:

```rust
// Only copy when output changed
if !output.changed { return; }  // Skip expensive copy
```

**Measurement**: 5-100× speedup already achieved.

---

### 3. SIMD BitField Operations

Use SIMD for parallel bit operations:

```rust
use std::arch::x86_64::*;

impl BitField {
    #[target_feature(enable = "avx2")]
    unsafe fn popcount_simd(&self) -> usize {
        let mut count = 0;
        for chunk in self.words.chunks_exact(8) {
            let v = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
            // Parallel popcount on 8 words at once
            count += _mm256_sad_epu8(v, _mm256_setzero_si256());
        }
        count
    }
}
```

**Benefit**: 2-4× speedup on `num_set()`, `get_acts()`.

---

### 4. Memory Pool for BlockOutput History

Pre-allocate memory to avoid allocation overhead:

```rust
pub struct MemoryPool {
    buffers: Vec<BitField>,
    free_list: Vec<usize>,
}

impl MemoryPool {
    pub fn allocate(&mut self, num_bits: usize) -> PooledBitField {
        if let Some(idx) = self.free_list.pop() {
            PooledBitField { idx, pool: self }
        } else {
            self.buffers.push(BitField::new(num_bits));
            PooledBitField { idx: self.buffers.len() - 1, pool: self }
        }
    }
}
```

**Benefit**: Reduces allocation overhead in tight loops.

---

### 5. Profile-Guided Optimization

Profile execution to identify bottlenecks:

```rust
pub struct ProfilingNetwork {
    network: Network,
    stats: HashMap<BlockId, BlockStats>,
}

struct BlockStats {
    total_time: Duration,
    call_count: u64,
    avg_time: Duration,
}

impl ProfilingNetwork {
    pub fn execute(&mut self, learn: bool) -> Result<()> {
        for &block_id in &self.network.execution_order {
            let start = Instant::now();
            self.network.blocks[block_id.0].execute(learn)?;
            let elapsed = start.elapsed();

            let stats = self.stats.entry(block_id).or_default();
            stats.total_time += elapsed;
            stats.call_count += 1;
            stats.avg_time = stats.total_time / stats.call_count as u32;
        }
        Ok(())
    }

    pub fn print_hotspots(&self) {
        let mut sorted: Vec<_> = self.stats.iter().collect();
        sorted.sort_by_key(|(_, s)| s.total_time);

        for (id, stats) in sorted.iter().rev().take(10) {
            println!("Block {:?}: {:?} total, {:?} avg",
                     id, stats.total_time, stats.avg_time);
        }
    }
}
```

---

## Scalability Test Plan

### Benchmark Suite

```rust
#[cfg(test)]
mod scalability_tests {
    use super::*;
    use criterion::{black_box, Criterion};

    fn benchmark_network_size(c: &mut Criterion) {
        for size in [10, 100, 1000, 10000] {
            c.bench_function(&format!("network_execute_{}", size), |b| {
                let mut net = create_test_network(size);
                net.build().unwrap();

                b.iter(|| {
                    net.execute(black_box(false)).unwrap();
                });
            });
        }
    }

    fn benchmark_network_depth(c: &mut Criterion) {
        for depth in [5, 10, 20, 50] {
            c.bench_function(&format!("network_depth_{}", depth), |b| {
                let mut net = create_deep_network(depth);
                net.build().unwrap();

                b.iter(|| {
                    net.execute(black_box(false)).unwrap();
                });
            });
        }
    }
}

fn create_test_network(num_blocks: usize) -> Network {
    // Create wide network for testing parallelism
    let mut net = Network::new();

    // Layer 1: num_blocks/2 encoders
    let encoders: Vec<_> = (0..num_blocks/2)
        .map(|i| net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, i as u64)))
        .collect();

    // Layer 2: num_blocks/2 poolers
    for encoder_id in encoders {
        net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
        net.connect(encoder_id, pooler_id).unwrap();
    }

    net
}
```

**Target Metrics**:
- 100 blocks: <1ms execution
- 1000 blocks: <10ms execution
- 10000 blocks: <100ms execution

---

## Conclusion: Scalability Assessment

### Can the Network Architecture Scale to 1000s of Blocks?

**YES**, with caveats:

✅ **Memory**: No problem (1000 blocks ≈ 20-200 MB)
✅ **Execution overhead**: Minimal (~7µs for 1000 blocks)
✅ **Graph construction**: Fast (O(V+E), ~50µs for 1000 blocks)
⚠️ **Parallelism**: Only beneficial for wide networks with >100µs/block
⚠️ **Synchronization**: Keep layers <100 for low overhead

### Recommendations

1. **Implement Layer-Based Parallelism** (Option 1 + Option 3)
   - Use arena allocation for safe parallel access
   - Only parallelize layers with >4 blocks
   - Use rayon's scoped threads

2. **Add Profiling Infrastructure**
   - Measure per-block execution time
   - Identify bottlenecks
   - Guide optimization efforts

3. **Optimize Hot Paths**
   - SIMD for BitField operations
   - Memory pooling for frequent allocations
   - Block fusion for small blocks

4. **Benchmark Early and Often**
   - Test with realistic network sizes
   - Measure parallel vs sequential
   - Profile memory usage

5. **Start Simple, Optimize Later**
   - Phase 1: Sequential execution
   - Phase 2: Layer-based parallelism
   - Phase 3: SIMD/GPU if needed

### Expected Performance

**Wide Network (1000 blocks, 10 layers)**:
- Sequential: ~100ms
- Parallel (8 cores): ~15-20ms
- Speedup: ~5-7×

**Deep Network (1000 blocks, 500 layers)**:
- Sequential: ~100ms
- Parallel: ~110ms (overhead dominates)
- Speedup: 0.9× (parallelism doesn't help)

### Bottom Line

The proposed Network architecture **scales well to 1000+ blocks**, especially for wide networks with substantial parallelism opportunities. The key is intelligent layer-based execution with adaptive parallelization based on network topology.
