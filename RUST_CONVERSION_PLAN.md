# Rust Conversion Plan for Gnomic Computing

## Executive Summary

This document outlines a comprehensive plan to convert the Gnomic Computing framework from C++ to Rust. The conversion will maintain API compatibility where possible while leveraging Rust's memory safety, modern tooling, and performance characteristics.

**Estimated Timeline:** 8-12 weeks for full conversion with testing
**Approach:** Bottom-up, component-by-component migration with continuous testing
**Structure:** Side-by-side implementation (C++ in `src/cpp/`, Rust in `src/rust/`) for easy comparison and validation

---

## Table of Contents

1. [Rationale for Rust Conversion](#rationale)
2. [Architecture Mapping](#architecture-mapping)
3. [Technical Challenges](#technical-challenges)
4. [Rust Crates and Dependencies](#rust-crates)
5. [Migration Phases](#migration-phases)
6. [Implementation Details](#implementation-details)
7. [Testing Strategy](#testing-strategy)
8. [Performance Considerations](#performance-considerations)
9. [Future Enhancements](#future-enhancements)

---

## Rationale for Rust Conversion {#rationale}

### Benefits

1. **Memory Safety:** Eliminate segfaults, buffer overflows, and use-after-free bugs at compile time
2. **Modern Tooling:**
   - Cargo for dependency management and building
   - Built-in testing framework with `cargo test`
   - Documentation generation with `cargo doc`
   - Integrated benchmarking
3. **Concurrency Safety:** Future parallelization opportunities without data races
4. **Type System:** Stronger guarantees and better error handling with `Result<T, E>`
5. **Community:** Active ecosystem with high-quality crates
6. **Cross-platform:** Better cross-compilation support
7. **Maintenance:** Fearless refactoring with compiler assistance

### Trade-offs

1. **Learning Curve:** Team must learn Rust ownership model
2. **Compilation Time:** Rust can have longer compile times than C++
3. **Ecosystem Maturity:** Some specialized libraries may be less mature
4. **Interop:** If C++ bindings needed, requires FFI layer

### Side-by-Side Structure Benefits

By placing Rust code in `src/rust/` alongside the existing C++ code in `src/cpp/`, we gain:

1. **Easy Comparison:** Directly compare implementations for correctness
2. **Gradual Migration:** Keep both versions functional during transition
3. **Cross-Validation:** Use C++ version to validate Rust outputs
4. **Reference Implementation:** C++ code serves as reference during porting
5. **Parallel Development:** Multiple developers can work on different components
6. **Risk Mitigation:** Fall back to C++ if issues arise
7. **Documentation:** Code structure mirrors original design
8. **Eventual Cleanup:** Once validated, can remove C++ code or keep for comparison

---

## Architecture Mapping {#architecture-mapping}

### C++ → Rust Conversions

| C++ Component | Rust Equivalent | Notes |
|---------------|-----------------|-------|
| `class Block` with virtual methods | `trait Block` with default implementations | Trait objects `Box<dyn Block>` where needed |
| Inheritance | Composition + Traits | Prefer composition over inheritance |
| `std::vector<T>` | `Vec<T>` | Direct equivalent |
| `std::mt19937` RNG | `rand::rngs::StdRng` | Use `rand` crate |
| Raw pointers `*` | `&`, `&mut`, `Box<T>`, `Rc<T>` | Enforce ownership |
| `uint32_t`, `uint8_t` | `u32`, `u8` | Native Rust types |
| `FILE*` | `std::fs::File`, `std::io::BufWriter` | Rust standard library |
| `assert()` | `assert!()`, `debug_assert!()` | Rust macros |
| CMake | Cargo + `Cargo.toml` | Rust build system |
| Manual memory management | Automatic with RAII | Drop trait |

### Type Mappings

```rust
// C++: class Block
// Rust:
pub trait Block {
    fn init(&mut self) -> Result<(), GnomicsError>;
    fn save(&self, path: &Path) -> Result<(), GnomicsError>;
    fn load(&mut self, path: &Path) -> Result<(), GnomicsError>;
    fn clear(&mut self);
    fn step(&mut self);
    fn pull(&mut self);
    fn push(&mut self);
    fn encode(&mut self);
    fn decode(&mut self);
    fn learn(&mut self);
    fn store(&mut self);
    fn memory_usage(&self) -> usize;

    fn feedforward(&mut self, learn_flag: bool) -> Result<(), GnomicsError> {
        self.step();
        self.pull();
        self.encode();
        self.store();
        if learn_flag {
            self.learn();
        }
        Ok(())
    }

    fn feedback(&mut self) -> Result<(), GnomicsError> {
        self.decode();
        self.push();
        Ok(())
    }
}
```

---

## Technical Challenges {#technical-challenges}

### 1. Ownership and Borrowing in Block Connections

**C++ Problem:**
```cpp
BlockInput.add_child(&child.output, time);  // Raw pointer
```

**Rust Solutions:**

**Option A: Reference Counting (Rc/Arc)**
```rust
pub struct BlockInput {
    children: Vec<Rc<RefCell<BlockOutput>>>,
    // ...
}

input.add_child(Rc::clone(&child_output), time);
```

**Option B: Indices (Arena Pattern)**
```rust
pub struct BlockGraph {
    outputs: Vec<BlockOutput>,
    inputs: Vec<BlockInput>,
}

pub struct OutputHandle(usize);

input.add_child(output_handle, time);
```

**Recommendation:** Use **Option B (Arena Pattern)** for better performance and clearer ownership.

### 2. Trait Objects vs Generics

**Challenge:** C++ uses virtual methods; Rust has two options.

**Solution:** Use both strategically:
- **Trait objects** (`Box<dyn Block>`) for heterogeneous collections
- **Generics** (`impl Block`) for performance-critical paths

```rust
// For storage
pub struct BlockGraph {
    blocks: Vec<Box<dyn Block>>,
}

// For specific usage
pub fn process<B: Block>(block: &mut B) {
    block.feedforward(true).unwrap();
}
```

### 3. BitArray Implementation

**C++ uses:** 32-bit word manipulation with platform-specific intrinsics

**Rust solution:** Use `bitvec` crate or custom implementation

```rust
use bitvec::prelude::*;

pub struct BitArray {
    bits: BitVec<u32, Lsb0>,
    num_bits: usize,
}

// Or custom for maximum performance
pub struct BitArray {
    words: Vec<u32>,
    num_bits: usize,
}

impl BitArray {
    #[inline]
    pub fn set_bit(&mut self, b: usize) {
        let word_idx = b >> 5;  // b / 32
        let bit_idx = b & 31;    // b % 32
        self.words[word_idx] |= 1 << bit_idx;
    }

    #[inline]
    pub fn get_bit(&self, b: usize) -> bool {
        let word_idx = b >> 5;
        let bit_idx = b & 31;
        (self.words[word_idx] >> bit_idx) & 1 == 1
    }
}
```

**Recommendation:** Start with `bitvec` for correctness, optimize with custom implementation if needed.

### 4. Random Number Generation

**C++ uses:** `std::mt19937` with seed per block

**Rust solution:**
```rust
use rand::{SeedableRng, Rng};
use rand::rngs::StdRng;

pub struct BlockBase {
    id: u32,
    init_flag: bool,
    rng: StdRng,
}

impl BlockBase {
    pub fn new(seed: u64) -> Self {
        Self {
            id: Self::next_id(),
            init_flag: false,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}
```

### 5. File I/O and Serialization

**C++ uses:** Manual `fwrite`/`fread`

**Rust solution:** Use `serde` for serialization
```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct BitArray {
    words: Vec<u32>,
    num_bits: usize,
}

impl BitArray {
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let file = File::create(path)?;
        bincode::serialize_into(file, self)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        bincode::deserialize_from(file)
    }
}
```

### 6. Error Handling

**C++ uses:** Return bool, sometimes assert

**Rust solution:** Use `Result` and custom error type
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GnomicsError {
    #[error("Block not initialized")]
    NotInitialized,

    #[error("Invalid input size: expected {expected}, got {actual}")]
    InvalidInputSize { expected: usize, actual: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
}

pub type Result<T> = std::result::Result<T, GnomicsError>;
```

---

## Rust Crates and Dependencies {#rust-crates}

### Core Dependencies

```toml
[dependencies]
# Bit manipulation
bitvec = "1.0"              # Efficient bit vector operations

# Random number generation
rand = "0.8"                # RNG framework
rand_chacha = "0.3"         # ChaCha RNG (alternative to MT19937)

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"             # Binary serialization

# Error handling
thiserror = "1.0"           # Error derive macros
anyhow = "1.0"              # Flexible error handling

# Numerics (optional)
ndarray = "0.15"            # N-dimensional arrays (if needed)

[dev-dependencies]
# Testing
criterion = "0.5"           # Benchmarking
proptest = "1.0"            # Property-based testing
approx = "0.5"              # Floating point comparison

[profile.release]
opt-level = 3
lto = true                  # Link-time optimization
codegen-units = 1          # Better optimization
```

### Optional Dependencies for Future Enhancements

```toml
# Parallelism
rayon = "1.7"               # Data parallelism

# Memory-mapped files
memmap2 = "0.5"             # Fast file I/O

# SIMD
packed_simd = "0.3"         # Explicit SIMD operations
```

---

## Migration Phases {#migration-phases}

### Phase 1: Foundation (Weeks 1-2)

**Goal:** Set up Rust project structure and implement core utilities

**Tasks:**
1. Create Cargo workspace structure
2. Implement `BitArray` with comprehensive tests
3. Implement `utils` module (shuffle, random)
4. Implement error types
5. Set up CI/CD with GitHub Actions

**Deliverables:**
- `src/rust/bitarray.rs` with complete implementation
- `src/rust/utils.rs` with utility functions
- Test suite in `tests/rust/` with 90%+ coverage
- Documentation with examples

**Files to Convert:**
- `src/cpp/bitarray.hpp/cpp` → `src/rust/bitarray.rs`
- `src/cpp/utils.hpp` → `src/rust/utils.rs`

### Phase 2: Block Infrastructure (Weeks 3-4)

**Goal:** Implement block system and I/O

**Tasks:**
1. Define `Block` trait
2. Implement `BlockOutput` with history
3. Implement `BlockInput` with child connections
4. Implement `BlockMemory` with learning algorithms
5. Create `BlockBase` helper struct
6. Design arena/graph system for block connections

**Deliverables:**
- `Block` trait system
- `BlockInput`, `BlockOutput`, `BlockMemory` modules
- Unit tests for each component

**Files to Convert:**
- `src/cpp/block.hpp/cpp` → `src/rust/block.rs`
- `src/cpp/block_input.hpp/cpp` → `src/rust/block_input.rs`
- `src/cpp/block_output.hpp/cpp` → `src/rust/block_output.rs`
- `src/cpp/block_memory.hpp/cpp` → `src/rust/block_memory.rs`

### Phase 3: Transformer Blocks (Week 5)

**Goal:** Implement encoding blocks

**Tasks:**
1. Implement `ScalarTransformer`
2. Implement `DiscreteTransformer`
3. Implement `PersistenceTransformer`
4. Port corresponding tests
5. Create integration tests

**Deliverables:**
- All transformer blocks in `src/rust/blocks/`
- Complete transformer implementations
- Tests in `tests/rust/` matching C++ behavior

**Files to Convert:**
- `src/cpp/blocks/scalar_transformer.hpp/cpp` → `src/rust/blocks/scalar_transformer.rs`
- `src/cpp/blocks/discrete_transformer.hpp/cpp` → `src/rust/blocks/discrete_transformer.rs`
- `src/cpp/blocks/persistence_transformer.hpp/cpp` → `src/rust/blocks/persistence_transformer.rs`

### Phase 4: Learning Blocks (Weeks 6-7)

**Goal:** Implement learning algorithms

**Tasks:**
1. Implement `PatternPooler`
2. Implement `PatternClassifier`
3. Implement `PatternClassifierDynamic`
4. Port all learning tests
5. Benchmark against C++ version

**Deliverables:**
- All learning blocks functional
- Performance within 10% of C++
- Comprehensive tests

**Files to Convert:**
- `src/cpp/blocks/pattern_pooler.hpp/cpp` → `src/rust/blocks/pattern_pooler.rs`
- `src/cpp/blocks/pattern_classifier.hpp/cpp` → `src/rust/blocks/pattern_classifier.rs`
- `src/cpp/blocks/pattern_classifier_dynamic.hpp/cpp` → `src/rust/blocks/pattern_classifier_dynamic.rs`

### Phase 5: Temporal Blocks (Week 8)

**Goal:** Implement temporal learning

**Tasks:**
1. Implement `ContextLearner`
2. Implement `SequenceLearner`
3. Port temporal tests
4. Create end-to-end sequence learning examples

**Deliverables:**
- Temporal blocks complete
- Anomaly detection working
- Example applications

**Files to Convert:**
- `src/cpp/blocks/context_learner.hpp/cpp` → `src/rust/blocks/context_learner.rs`
- `src/cpp/blocks/sequence_learner.hpp/cpp` → `src/rust/blocks/sequence_learner.rs`

### Phase 6: Testing and Documentation (Weeks 9-10)

**Goal:** Comprehensive testing and documentation

**Tasks:**
1. Port all C++ tests to Rust
2. Add property-based tests with `proptest`
3. Create benchmarks with `criterion`
4. Write API documentation
5. Create examples and tutorials
6. Write migration guide

**Deliverables:**
- 95%+ test coverage
- Performance benchmarks
- Complete documentation
- Example projects

### Phase 7: Polish and Release (Weeks 11-12)

**Goal:** Production-ready release

**Tasks:**
1. Performance optimization based on benchmarks
2. API cleanup and stabilization
3. Create Python bindings (using PyO3)
4. Create C FFI for compatibility
5. Publish to crates.io
6. Create release notes

**Deliverables:**
- Gnomics v1.0.0 Rust release
- Python bindings package
- C compatibility layer

---

## Implementation Details {#implementation-details}

### Project Structure

```
gnomics/
├── Cargo.toml                # Rust workspace definition
├── CMakeLists.txt            # C++ build (existing)
├── README.md
├── CLAUDE.md
├── RUST_CONVERSION_PLAN.md
├── src/
│   ├── cpp/                  # Existing C++ source
│   │   ├── bitarray.cpp/hpp
│   │   ├── block.cpp/hpp
│   │   ├── block_input.cpp/hpp
│   │   ├── block_output.cpp/hpp
│   │   ├── block_memory.cpp/hpp
│   │   ├── utils.hpp
│   │   └── blocks/
│   │       └── ...
│   └── rust/                 # New Rust source (mirrors C++ structure)
│       ├── lib.rs
│       ├── bitarray.rs
│       ├── utils.rs
│       ├── error.rs
│       ├── block.rs          # Trait definition
│       ├── block_base.rs     # Common implementation
│       ├── block_input.rs
│       ├── block_output.rs
│       ├── block_memory.rs
│       └── blocks/
│           ├── mod.rs
│           ├── scalar_transformer.rs
│           ├── discrete_transformer.rs
│           ├── pattern_pooler.rs
│           ├── pattern_classifier.rs
│           ├── pattern_classifier_dynamic.rs
│           ├── context_learner.rs
│           ├── sequence_learner.rs
│           ├── persistence_transformer.rs
│           └── blank_block.rs
├── tests/
│   ├── cpp/                  # Existing C++ tests
│   │   ├── test_bitarray.cpp
│   │   ├── test_pattern_classifier.cpp
│   │   └── ...
│   └── rust/                 # New Rust tests (mirrors C++ tests)
│       ├── test_bitarray.rs
│       ├── test_pattern_classifier.rs
│       ├── test_pattern_pooler.rs
│       ├── test_sequence_learner.rs
│       ├── test_context_learner.rs
│       └── ...
├── examples/                 # Rust examples
│   ├── classification.rs
│   └── sequence_learning.rs
├── benches/                  # Rust benchmarks
│   ├── bitarray_bench.rs
│   └── blocks_bench.rs
└── bindings/                 # Language bindings (Phase 7)
    ├── python/
    │   ├── Cargo.toml
    │   └── src/lib.rs
    └── ffi/
        ├── Cargo.toml
        ├── src/lib.rs
        └── gnomics.h
```

### Code Style and Conventions

1. **Naming:**
   - `snake_case` for functions and variables
   - `PascalCase` for types and traits
   - `SCREAMING_SNAKE_CASE` for constants

2. **Documentation:**
   - Use `///` for public API docs
   - Use `//!` for module-level docs
   - Include examples in documentation

3. **Error Handling:**
   - Return `Result` for fallible operations
   - Use `?` operator for propagation
   - Provide context with error messages

4. **Testing:**
   - Unit tests in same file with `#[cfg(test)]`
   - Integration tests in `tests/` directory
   - Benchmarks in `benches/` directory

### Example: BitArray Implementation

```rust
// src/rust/bitarray.rs

use std::ops::{BitAnd, BitOr, BitXor, Not};

/// Efficient bit array using 32-bit words
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitArray {
    words: Vec<u32>,
    num_bits: usize,
}

impl BitArray {
    /// Create a new BitArray with `n` bits, all initialized to 0
    pub fn new(n: usize) -> Self {
        let num_words = (n + 31) / 32;
        Self {
            words: vec![0u32; num_words],
            num_bits: n,
        }
    }

    /// Set bit at position `b` to 1
    #[inline]
    pub fn set_bit(&mut self, b: usize) {
        debug_assert!(b < self.num_bits, "bit index out of bounds");
        let word_idx = b >> 5;
        let bit_idx = b & 31;
        self.words[word_idx] |= 1 << bit_idx;
    }

    /// Get bit at position `b`
    #[inline]
    pub fn get_bit(&self, b: usize) -> bool {
        debug_assert!(b < self.num_bits, "bit index out of bounds");
        let word_idx = b >> 5;
        let bit_idx = b & 31;
        (self.words[word_idx] >> bit_idx) & 1 == 1
    }

    /// Clear bit at position `b` (set to 0)
    #[inline]
    pub fn clear_bit(&mut self, b: usize) {
        debug_assert!(b < self.num_bits, "bit index out of bounds");
        let word_idx = b >> 5;
        let bit_idx = b & 31;
        self.words[word_idx] &= !(1 << bit_idx);
    }

    /// Count number of set bits
    pub fn num_set(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Get indices of all set bits
    pub fn get_acts(&self) -> Vec<usize> {
        let mut acts = Vec::with_capacity(self.num_set());
        for (word_idx, &word) in self.words.iter().enumerate() {
            if word == 0 { continue; }
            let base = word_idx << 5;
            for bit_idx in 0..32 {
                if base + bit_idx >= self.num_bits { break; }
                if (word >> bit_idx) & 1 == 1 {
                    acts.push(base + bit_idx);
                }
            }
        }
        acts
    }

    /// Set bits from indices
    pub fn set_acts(&mut self, idxs: &[usize]) {
        self.clear_all();
        for &idx in idxs {
            if idx < self.num_bits {
                self.set_bit(idx);
            }
        }
    }

    /// Clear all bits to 0
    pub fn clear_all(&mut self) {
        self.words.fill(0);
    }

    /// Count similar set bits between two BitArrays
    pub fn num_similar(&self, other: &BitArray) -> usize {
        assert_eq!(self.words.len(), other.words.len());
        self.words.iter()
            .zip(other.words.iter())
            .map(|(a, b)| (a & b).count_ones() as usize)
            .sum()
    }

    /// Number of bits in the array
    pub fn len(&self) -> usize {
        self.num_bits
    }

    /// Memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.words.capacity() * std::mem::size_of::<u32>()
    }
}

// Implement bitwise operators
impl BitAnd for &BitArray {
    type Output = BitArray;

    fn bitand(self, rhs: Self) -> BitArray {
        assert_eq!(self.num_bits, rhs.num_bits);
        let words = self.words.iter()
            .zip(rhs.words.iter())
            .map(|(a, b)| a & b)
            .collect();
        BitArray { words, num_bits: self.num_bits }
    }
}

impl BitOr for &BitArray {
    type Output = BitArray;

    fn bitor(self, rhs: Self) -> BitArray {
        assert_eq!(self.num_bits, rhs.num_bits);
        let words = self.words.iter()
            .zip(rhs.words.iter())
            .map(|(a, b)| a | b)
            .collect();
        BitArray { words, num_bits: self.num_bits }
    }
}

impl Not for &BitArray {
    type Output = BitArray;

    fn not(self) -> BitArray {
        let words = self.words.iter().map(|w| !w).collect();
        BitArray { words, num_bits: self.num_bits }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut ba = BitArray::new(32);
        assert_eq!(ba.num_set(), 0);

        ba.set_bit(5);
        ba.set_bit(10);
        ba.set_bit(15);

        assert_eq!(ba.num_set(), 3);
        assert!(ba.get_bit(5));
        assert!(ba.get_bit(10));
        assert!(!ba.get_bit(7));

        let acts = ba.get_acts();
        assert_eq!(acts, vec![5, 10, 15]);
    }

    #[test]
    fn test_bitwise_ops() {
        let mut ba1 = BitArray::new(32);
        ba1.set_bit(0);
        ba1.set_bit(5);

        let mut ba2 = BitArray::new(32);
        ba2.set_bit(5);
        ba2.set_bit(10);

        let result = &ba1 & &ba2;
        assert_eq!(result.num_set(), 1);
        assert!(result.get_bit(5));
    }
}
```

### Example: Block Trait

```rust
// src/rust/block.rs

use crate::error::Result;
use std::path::Path;

/// Core trait for all Gnomics computational blocks
pub trait Block {
    /// Initialize the block based on input connections
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Save block state to file
    fn save(&self, path: &Path) -> Result<()>;

    /// Load block state from file
    fn load(&mut self, path: &Path) -> Result<()>;

    /// Clear all internal state
    fn clear(&mut self);

    /// Advance time step
    fn step(&mut self);

    /// Pull data from child blocks
    fn pull(&mut self);

    /// Push data to child blocks
    fn push(&mut self);

    /// Encode input to output
    fn encode(&mut self);

    /// Decode output to input
    fn decode(&mut self);

    /// Update internal memories/weights
    fn learn(&mut self);

    /// Store current state to history
    fn store(&mut self);

    /// Estimate memory usage in bytes
    fn memory_usage(&self) -> usize;

    /// Process input to output (feedforward pass)
    fn feedforward(&mut self, learn_flag: bool) -> Result<()> {
        self.step();
        self.pull();
        self.encode();
        self.store();
        if learn_flag {
            self.learn();
        }
        Ok(())
    }

    /// Process output to input (feedback pass)
    fn feedback(&mut self) -> Result<()> {
        self.decode();
        self.push();
        Ok(())
    }
}

/// Common state shared by all blocks
pub struct BlockBase {
    id: u32,
    init_flag: bool,
    rng: rand::rngs::StdRng,
}

impl BlockBase {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;

        static NEXT_ID: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(0);

        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            init_flag: false,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    pub fn id(&self) -> u32 { self.id }
    pub fn is_initialized(&self) -> bool { self.init_flag }
    pub fn set_initialized(&mut self, flag: bool) { self.init_flag = flag; }
    pub fn rng(&mut self) -> &mut rand::rngs::StdRng { &mut self.rng }
}
```

---

## Testing Strategy {#testing-strategy}

### 1. Unit Tests

Test each component in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitarray_basic() {
        let mut ba = BitArray::new(1024);
        ba.set_bit(100);
        assert!(ba.get_bit(100));
        assert_eq!(ba.num_set(), 1);
    }

    #[test]
    fn test_scalar_transformer_encoding() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
        st.set_value(0.5);
        st.encode();

        let acts = st.output().state().get_acts();
        assert_eq!(acts.len(), 128);
    }
}
```

### 2. Integration Tests

Test block combinations:

```rust
// tests/rust/test_classification.rs

use gnomics::blocks::*;

#[test]
fn test_classification_pipeline() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut classifier = PatternClassifier::new(4, 1024, 8, /* params */);

    // Connect blocks
    classifier.input_mut().add_child(encoder.output(), 0);
    classifier.init().unwrap();

    // Train
    for i in 0..10 {
        encoder.set_value(0.25 * i as f64);
        classifier.set_label(i % 4);
        encoder.feedforward(false).unwrap();
        classifier.feedforward(true).unwrap();
    }

    // Test
    encoder.set_value(0.5);
    encoder.feedforward(false).unwrap();
    classifier.feedforward(false).unwrap();

    let probs = classifier.get_probabilities();
    assert_eq!(probs.len(), 4);
}
```

### 3. Property-Based Tests

Use `proptest` for fuzzing:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_bitarray_set_get_consistency(bits in prop::collection::vec(any::<bool>(), 1..1000)) {
        let mut ba = BitArray::new(bits.len());
        for (i, &b) in bits.iter().enumerate() {
            if b {
                ba.set_bit(i);
            }
        }

        for (i, &b) in bits.iter().enumerate() {
            prop_assert_eq!(ba.get_bit(i), b);
        }
    }
}
```

### 4. Benchmarks

Use `criterion` for performance testing:

```rust
// benches/bitarray_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gnomics::BitArray;

fn bench_set_bits(c: &mut Criterion) {
    c.bench_function("bitarray set 1000 bits", |b| {
        let mut ba = BitArray::new(10000);
        b.iter(|| {
            for i in 0..1000 {
                ba.set_bit(black_box(i));
            }
        });
    });
}

criterion_group!(benches, bench_set_bits);
criterion_main!(benches);
```

### 5. Cross-validation with C++

Run identical tests on both versions and compare outputs:

```bash
# Generate test data with C++ version
./cpp_tests --generate-data test_vectors.json

# Validate Rust version
cargo test --test cross_validation -- --test-data test_vectors.json
```

---

## Performance Considerations {#performance-considerations}

### Optimization Strategies

1. **Profile First:** Use `cargo flamegraph` and `perf` to identify hotspots
2. **Inline Hot Paths:** Use `#[inline]` for frequently called methods
3. **SIMD:** Use `packed_simd` for bit operations if needed
4. **Memory Layout:** Use `#[repr(C)]` or `#[repr(packed)]` for optimal layout
5. **Avoid Allocations:** Reuse buffers, use `Vec::with_capacity()`
6. **Parallel Processing:** Use `rayon` for data parallelism (future enhancement)

### Performance Targets

| Operation | C++ Time | Rust Target | Strategy |
|-----------|----------|-------------|----------|
| BitArray set_bit | ~2ns | <3ns | Inline, bounds check in debug only |
| BitArray num_set | ~50ns/1024bits | <60ns | SIMD popcount |
| Pattern encode | ~1μs | <1.2μs | Optimize hot loop |
| Learn step | ~10μs | <12μs | Inline memory access |

### Benchmarking Plan

```bash
# Run benchmarks
cargo bench

# Compare with baseline
cargo bench --bench bitarray -- --baseline c++_baseline

# Generate flame graph
cargo flamegraph --bench bitarray
```

---

## Future Enhancements {#future-enhancements}

### Post-Conversion Improvements

1. **Parallelization:**
   ```rust
   use rayon::prelude::*;

   // Process multiple blocks in parallel
   blocks.par_iter_mut().for_each(|block| {
       block.feedforward(true).unwrap();
   });
   ```

2. **WASM Support:**
   - Compile to WebAssembly for browser-based ML
   - Target: `wasm32-unknown-unknown`

3. **GPU Acceleration:**
   - Use `wgpu` or `cuda-rs` for GPU operations
   - Offload large block computations

4. **Python Bindings:**
   ```python
   import gnomics

   encoder = gnomics.ScalarTransformer(0.0, 1.0, 1024, 128)
   classifier = gnomics.PatternClassifier(4, 1024, 8)

   encoder.set_value(0.5)
   encoder.feedforward()
   ```

5. **Async I/O:**
   - Use `tokio` for async file operations
   - Parallel model loading

6. **Distributed Computing:**
   - Network protocol for distributed block graphs
   - Remote block execution

7. **Visualization:**
   - Real-time visualization of activations
   - Training progress monitoring

8. **Dynamic Block Graphs:**
   - Runtime graph construction
   - Conditional execution paths

---

## Risk Mitigation

### Identified Risks

1. **Performance Regression:**
   - Mitigation: Continuous benchmarking, optimization sprints

2. **API Breaking Changes:**
   - Mitigation: Maintain compatibility layer for major versions

3. **Learning Curve:**
   - Mitigation: Training sessions, pair programming, code reviews

4. **Incomplete Test Coverage:**
   - Mitigation: Require 90%+ coverage, property-based testing

5. **Memory Leaks in Unsafe Code:**
   - Mitigation: Minimize `unsafe`, use Miri for undefined behavior detection

---

## Success Criteria

### Must Have (v1.0)

- ✅ All C++ functionality ported
- ✅ All tests passing
- ✅ Performance within 10% of C++
- ✅ Complete API documentation
- ✅ Example applications

### Should Have (v1.1)

- ✅ Python bindings
- ✅ C FFI layer
- ✅ Parallel processing support
- ✅ Enhanced error messages

### Nice to Have (v2.0)

- ✅ WASM support
- ✅ GPU acceleration
- ✅ Distributed computing
- ✅ Real-time visualization

---

## Appendix A: Quick Start Commands

### Initialize Rust Project

```bash
# Navigate to project root
cd gnomics

# Create Cargo.toml at root level
cat > Cargo.toml << 'EOF'
[package]
name = "gnomics"
version = "1.0.0"
edition = "2021"

[lib]
name = "gnomics"
path = "src/rust/lib.rs"

[[test]]
name = "integration"
path = "tests/rust/test_bitarray.rs"

[dependencies]
bitvec = "1.0"
rand = "0.8"
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
thiserror = "1.0"

[dev-dependencies]
criterion = "0.5"
proptest = "1.0"
approx = "0.5"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
EOF

# Create src/rust directory structure
mkdir -p src/rust/blocks
mkdir -p tests/rust
mkdir -p benches
mkdir -p examples

# Create lib.rs
touch src/rust/lib.rs

# Run tests
cargo test

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open

# Build C++ alongside (existing)
mkdir build && cd build
cmake ..
make
cd ..
```

### Development Workflow

```bash
# Rust development
# ----------------

# Check code (fast)
cargo check

# Build with optimizations
cargo build --release

# Run tests with coverage
cargo tarpaulin --out Html

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Run specific test
cargo test test_bitarray

# Benchmark specific function
cargo bench bench_set_bits

# C++ development (existing)
# --------------------------

# Build C++ version
cd build
cmake ..
make

# Run C++ tests
cmake -DGnomics_TESTS=true ..
make
./tests/cpp/test_bitarray

# Compare outputs
# ---------------

# Run both versions and compare
./build/tests/cpp/test_bitarray > cpp_output.txt
cargo test test_bitarray -- --nocapture > rust_output.txt
diff cpp_output.txt rust_output.txt
```

---

## Appendix B: Resources

### Learning Rust

- [The Rust Programming Language](https://doc.rust-lang.org/book/) (The Book)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rustlings](https://github.com/rust-lang/rustlings) (Interactive exercises)

### Rust for C++ Programmers

- [Rust for C++ Programmers](https://github.com/nrc/r4cppp)
- [From C++ to Rust](https://locka99.gitbooks.io/a-guide-to-porting-c-to-rust/)

### Performance

- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Optimizing Rust](https://gist.github.com/jFransham/369a86eff00e5f280ed25121454acec1)

### Crates

- [docs.rs](https://docs.rs) - Documentation for all crates
- [crates.io](https://crates.io) - Rust package registry

---

## Conclusion

This conversion plan provides a comprehensive roadmap for migrating Gnomic Computing from C++ to Rust. The phased approach ensures continuous validation and allows for iterative improvements. The result will be a safer, more maintainable, and potentially faster implementation while preserving the core algorithms and design philosophy of the original framework.

**Next Steps:**
1. Review and approve this plan
2. Create `Cargo.toml` at project root
3. Set up `src/rust/` directory structure
4. Set up Rust development environment and toolchain
5. Begin Phase 1 implementation (BitArray + utils)
6. Schedule weekly progress reviews
7. Set up CI/CD to test both C++ and Rust versions

---

**Document Version:** 1.1
**Last Updated:** 2025-10-04
**Status:** Ready for Review
**Revision:** Updated to use side-by-side structure (C++ in `src/cpp/`, Rust in `src/rust/`)
