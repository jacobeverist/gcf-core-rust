# CLAUDE.md - Gnomic Computing Framework

## Project Overview

Gnomic Computing is a C++ framework for building scalable Machine Learning applications using computational neuroscience principles. The framework models neuron activations with **binary patterns** (vectors of 1s and 0s) that form a "cortical language" for computation.

**Key Characteristics:**
- Single-threaded C++ backend
- Low-level bitwise operations for performance
- Hierarchical block architecture
- Inspired by Hierarchical Temporal Memory (HTM) principles
- Focus on binary pattern processing and learning

## Architecture

### Core Components

#### 1. **Block System** (`src/cpp/block.hpp`, `src/cpp/block.cpp`)

The `Block` class is the base class for all computational units in Gnomics. It provides a lifecycle management system with virtual functions:

- **`init()`** - Initialize block memories and parameters
- **`step()`** - Update block output history index
- **`pull()`** - Pull data from child block outputs into inputs
- **`push()`** - Push data from inputs to child block outputs
- **`encode()`** - Convert inputs to outputs
- **`decode()`** - Convert outputs to inputs (for feedback)
- **`learn()`** - Update internal memories/weights
- **`store()`** - Store current state into history
- **`save()`/`load()`** - Persistence operations
- **`clear()`** - Reset state

Two high-level operations orchestrate the lifecycle:
- **`feedforward(bool learn_flag)`** - Executes: step → pull → encode → store → [optional: learn]
- **`feedback()`** - Executes: decode → push

Each block has a unique ID and random number generator (RNG) for reproducible randomness.

#### 2. **BitArray** (`src/cpp/bitarray.hpp`, `src/cpp/bitarray.cpp`)

High-performance bit manipulation class using 32-bit words:

**Key Operations:**
- Individual bit manipulation: `set_bit()`, `get_bit()`, `clear_bit()`, `toggle_bit()`
- Bulk operations: `set_all()`, `clear_all()`, `set_range()`
- Vector operations: `set_acts()` (set from indices), `get_acts()` (get active indices)
- Counting: `num_set()`, `num_cleared()`, `num_similar()`
- Search: `find_next_set_bit()` with wrapping
- Random: `random_shuffle()`, `random_set_num()`, `random_set_pct()`
- Logic operators: `~`, `&`, `|`, `^`
- Comparison: `==`, `!=`

**Implementation Details:**
- Uses `word_t` (uint32_t) for storage
- Efficient popcount for counting set bits
- Platform-specific optimizations (builtin functions for trailing/leading zeros)
- Memory-efficient with contiguous storage

#### 3. **BlockInput** (`src/cpp/block_input.hpp`)

Manages inputs to a block from multiple child blocks:

- **`state`** - Current input BitArray
- **`add_child(BlockOutput* src, uint32_t src_t)`** - Connect to child output at time offset
- **`pull()`** - Aggregate data from all children into state
- **`push()`** - Distribute state back to children
- **`children_changed()`** - Check if any child outputs changed

Tracks child connections with:
- `children` - Pointers to child BlockOutput objects
- `times` - Time offsets for each child
- `word_offsets` - Bit positions for concatenation
- `word_sizes` - Size of each child's contribution

#### 4. **BlockOutput** (`src/cpp/block_output.hpp`)

Manages outputs from a block with history:

- **`state`** - Working BitArray for current output
- **`setup(num_t, num_b)`** - Configure history depth and bit size
- **`step()`** - Advance to next time step
- **`store()`** - Save current state into history
- **`get_bitarray(t)`** or **`operator[](t)`** - Access history at time offset
- **`has_changed()`** - Check if output changed

History access uses relative time:
- `CURR` (0) - Current time step
- `PREV` (1) - Previous time step
- `history` vector stores multiple time steps

#### 5. **BlockMemory** (`src/cpp/block_memory.hpp`)

Implements synaptic-like learning mechanisms with dendrites and receptors:

**Structure:**
- **Dendrites** - Computational units (num_d)
- **Receptors per dendrite** - Connection points (num_rpd)
- **Receptor addresses** - Which input bits connect (r_addrs)
- **Receptor permanences** - Connection strengths 0-99 (r_perms)
- **Dendrite connections** - Optional connectivity mask (d_conns)

**Learning Parameters:**
- `perm_thr` - Threshold for receptor to be "connected" (typically 20)
- `perm_inc` - Permanence increase on positive learning (typically 2)
- `perm_dec` - Permanence decrease on negative learning (typically 1)
- `pct_learn` - Percentage of receptors that can learn per update

**Core Operations:**
- **`overlap(d, input)`** - Count matching connected receptors for dendrite d
- **`learn(d, input)`** - Strengthen matching receptors, weaken non-matching
- **`punish(d, input)`** - Weaken matching receptors
- **`learn_move(d, input)`** - Move receptors to match input pattern

Variants with `_conn` suffix check dendrite connectivity mask before operations.

**Initialization Modes:**
- `init()` - Basic initialization with full connectivity
- `init_pooled()` - Sparse connectivity (pct_pool controls sparsity)

### Computational Blocks

#### 1. **ScalarTransformer** (`src/cpp/blocks/scalar_transformer.hpp`)

Encodes continuous scalar values into binary patterns:

**Parameters:**
- `min_val`, `max_val` - Input value range
- `num_s` - Number of statelets (output bits)
- `num_as` - Number of active statelets (typically 10-20% of num_s)
- `num_t` - History depth

**Operation:**
Maps scalar to a position in statelet space, activating a contiguous window of `num_as` bits. This creates overlapping representations where similar values have overlapping active bits.

**Example:** Value 0.5 in [0.0, 1.0] with 1024 statelets and 128 active → activates bits 448-575

#### 2. **DiscreteTransformer** (`src/cpp/blocks/discrete_transformer.hpp`)

Encodes discrete categorical values into binary patterns:

**Parameters:**
- `num_v` - Number of discrete values
- `num_s` - Number of statelets (automatically divides by num_v)
- `num_t` - History depth

**Operation:**
Each discrete value gets a distinct set of active bits with no overlap. Uses `num_as = num_s / num_v` bits per category.

**Example:** 4 categories with 1024 statelets → each category activates 256 unique bits

#### 3. **PatternClassifier** (`src/cpp/blocks/pattern_classifier.hpp`)

Supervised learning classifier for binary patterns:

**Parameters:**
- `num_l` - Number of labels/classes
- `num_s` - Number of statelets (dendrites)
- `num_as` - Number of active statelets in output
- `perm_thr`, `perm_inc`, `perm_dec` - Learning parameters
- `pct_pool`, `pct_conn` - Sparsity parameters
- `pct_learn` - Learning rate

**Architecture:**
- Divides statelets into `num_l` groups (num_spl = num_s / num_l)
- Each group represents one label
- Uses BlockMemory with pooled connectivity

**Operation:**
- **Encode:** Compute overlaps for all dendrites, activate top `num_as` per label group
- **Learn:** When `set_label(label)` called, strengthen winning dendrites for that label
- **Inference:** `get_probabilities()` returns likelihood for each label based on overlap

**Usage Pattern:**
```cpp
pc.set_label(label);           // Set ground truth
pc.feedforward(true);          // Encode and learn
vector<double> probs = pc.get_probabilities();  // Get predictions
```

#### 4. **PatternPooler** (`src/cpp/blocks/pattern_pooler.hpp`)

Learns sparse distributed representations from input patterns:

**Parameters:**
- `num_s` - Number of statelets (dendrites)
- `num_as` - Number of active statelets in output
- `perm_thr`, `perm_inc`, `perm_dec` - Learning parameters
- `pct_pool`, `pct_conn` - Sparsity (typically 0.8, 0.5)
- `pct_learn` - Learning rate (typically 0.3)
- `always_update` - Whether to update even when input unchanged

**Operation:**
- Computes overlap between each dendrite and input
- Activates top `num_as` dendrites with highest overlap
- During learning, winning dendrites strengthen connections to active input bits
- Creates stable, sparse representations similar to cortical minicolumns

**Use Case:** Dimensionality reduction, feature learning, creating pooled representations

#### 5. **ContextLearner** (`src/cpp/blocks/context_learner.hpp`)

Learns contextual associations and detects anomalies:

**Parameters:**
- `num_c` - Number of columns
- `num_spc` - Statelets per column
- `num_dps` - Dendrites per statelet
- `num_rpd` - Receptors per dendrite
- `d_thresh` - Dendrite activation threshold
- `perm_thr`, `perm_inc`, `perm_dec` - Learning parameters

**Architecture:**
- Two inputs: `input` (current pattern) and `context` (contextual pattern)
- Dendrites learn to predict input given context
- Columns organize statelets representing input space

**Operation:**
- **Recognition:** Input + context match learned patterns → activate predicted statelets
- **Surprise:** No matching dendrite → activate based on input alone, create new dendrite
- **Anomaly Score:** `get_anomaly_score()` returns percentage of unexpected input

**Use Cases:**
- Context-dependent pattern recognition
- Anomaly detection when patterns appear in wrong context
- Learning "what follows what" associations

#### 6. **SequenceLearner** (`src/cpp/blocks/sequence_learner.hpp`)

Learns temporal sequences and predicts next patterns:

**Parameters:** Same as ContextLearner

**Architecture:** Nearly identical to ContextLearner
- Two inputs: `input` (current) and `context` (previous time step from `output[PREV]`)
- Self-feedback loop for temporal learning

**Operation:**
- At each time step, uses previous output as context
- Learns transitions: "if pattern A active, pattern B follows"
- Predicts next pattern based on current state
- Flags anomaly when unexpected sequence occurs

**Use Cases:**
- Time series prediction
- Sequence learning (e.g., motor patterns, language)
- Anomaly detection in temporal data

**Key Difference from ContextLearner:** ContextLearner uses external context input, SequenceLearner uses its own history.

#### 7. **PersistenceTransformer** (`src/cpp/blocks/persistence_transformer.hpp`)

Maintains pattern persistence over time (based on file listing)

#### 8. **PatternClassifierDynamic** (`src/cpp/blocks/pattern_classifier_dynamic.hpp`)

Variant of PatternClassifier with dynamic label allocation (based on file listing)

## Data Flow

### Typical Processing Pipeline

```
Input Data → Transformer → PatternPooler → PatternClassifier → Output
                ↓              ↓                 ↓
            Binary         Sparse Coding    Supervised Learning
```

### Temporal Processing

```
Input Sequence → SequenceLearner → Prediction + Anomaly
                      ↑
                 Self-feedback
```

### Contextual Processing

```
Input Pattern  ┐
               ├→ ContextLearner → Contextual Prediction + Anomaly
Context Pattern┘
```

## Building and Testing

### Build Commands

```bash
# Basic build
mkdir build && cd build
cmake ..
make

# Build with tests
mkdir build && cd build
cmake -DGnomics_TESTS=true ..
make
```

### Project Structure

```
gnomics/
├── src/cpp/              # Core C++ implementation
│   ├── bitarray.cpp/hpp  # Binary array operations
│   ├── block.cpp/hpp     # Base block class
│   ├── block_input.cpp/hpp
│   ├── block_output.cpp/hpp
│   ├── block_memory.cpp/hpp
│   ├── utils.hpp         # Utility functions
│   └── blocks/           # Computational blocks
│       ├── scalar_transformer.cpp/hpp
│       ├── discrete_transformer.cpp/hpp
│       ├── pattern_classifier.cpp/hpp
│       ├── pattern_pooler.cpp/hpp
│       ├── context_learner.cpp/hpp
│       ├── sequence_learner.cpp/hpp
│       └── ...
├── tests/cpp/            # C++ unit tests
│   ├── test_bitarray.cpp
│   ├── test_pattern_classifier.cpp
│   └── ...
├── CMakeLists.txt        # Root CMake config
└── README.md
```

### Test Structure

Tests demonstrate typical usage patterns:
- Create blocks with parameters
- Connect blocks via `input.add_child(&child.output, time_offset)`
- Call `init()` on parent blocks
- Run processing loop:
  - Set inputs via transformer blocks
  - Call `feedforward(learn_flag)` to propagate
  - Read outputs and metrics

Example from `test_pattern_classifier.cpp`:
```cpp
ScalarTransformer st(0.0, 1.0, 1024, 128);
PatternClassifier pc(4, 1024, 8, 20, 2, 1, 0.8, 0.5, 0.3, 2);
pc.input.add_child(&st.output, 0);
pc.init();

st.set_value(0.5);
pc.set_label(0);
st.feedforward();
pc.feedforward(true);  // Encode and learn
```

## Memory Efficiency

Gnomics is designed for minimal memory footprint:
- BitArray uses packed 32-bit words (32x compression vs bool arrays)
- BlockMemory uses sparse connectivity via pooling percentages
- All classes provide `memory_usage()` for tracking

## Performance Considerations

1. **Bitwise Operations:** Core computations use word-level bit operations rather than individual bits
2. **Single-threaded:** Designed for single CPU core (SIMD opportunities exist)
3. **Memory Locality:** Contiguous vectors for cache efficiency
4. **Sparse Connectivity:** Only store and process connected synapses

## Key Design Patterns

### 1. Block Hierarchy
Blocks connect via parent-child relationships. Parent pulls data from children's output history.

### 2. Time History
BlockOutput maintains circular history buffer for temporal processing. Access via relative offsets (CURR, PREV).

### 3. Lazy Initialization
Blocks defer initialization until first `feedforward()` call, allowing dynamic configuration based on input sizes.

### 4. Pooled Connectivity
Instead of dense all-to-all connections, dendrites connect to random subsets of inputs (controlled by `pct_pool` and `pct_conn`).

### 5. Permanence-based Learning
Synaptic strengths (permanences 0-99) slowly adjust. Only permanences above threshold contribute. This creates stable, noise-resistant learning.

## Common Parameters Across Blocks

- **`num_t`** - History depth (typically 2 for current + previous)
- **`num_s`** - Number of statelets/dendrites (typically 1024-4096)
- **`num_as`** - Active statelets (typically 10-20% of num_s)
- **`perm_thr`** - Permanence threshold (typically 20 out of 99)
- **`perm_inc`** - Permanence increment (typically 2)
- **`perm_dec`** - Permanence decrement (typically 1)
- **`pct_pool`** - Pooling percentage (typically 0.8 = 80% sparsity)
- **`pct_conn`** - Initial connectivity (typically 0.5 = 50% connected)
- **`pct_learn`** - Learning percentage (typically 0.3 = 30% update)
- **`seed`** - RNG seed for reproducibility

## HTM/Neuroscience Inspiration

Gnomics implements several concepts from neuroscience and HTM:

1. **Sparse Distributed Representations (SDRs):** Binary patterns with small percentage of active bits
2. **Minicolumns:** Organized groups of neurons (statelets) that compete
3. **Dendrites:** Computational subunits that detect patterns
4. **Synaptic Permanence:** Connection strengths that slowly adapt
5. **Temporal Memory:** Using history to predict sequences
6. **Contextual Learning:** Different responses based on context

## Development Notes

### TODOs in Code
- Virtual destructor for Block class (block.hpp:20)
- 64-bit word support for BitArray (commented out)
- Additional BitArray methods (hamming distance, shifts, cycles)
- BlockMemory improvements (bit-indexing vs word-indexing)
- Push/decode implementations for some blocks
- Memory usage implementations

### Platform Support
- Windows (7, 8, 10, 11)
- macOS (10.14+)
- Linux (Ubuntu 16+, CentOS 7+)

### Build System
- CMake 3.7+
- C++11 standard
- Creates static library `bbcore`
- Optional test builds with `-DGnomics_TESTS=true`

## Usage Examples

### Classification Pipeline

```cpp
// Create blocks
ScalarTransformer input_encoder(0.0, 100.0, 2048, 256);
PatternPooler feature_learner(2048, 40);
PatternClassifier classifier(10, 1024, 20);

// Connect pipeline
feature_learner.input.add_child(&input_encoder.output, 0);
classifier.input.add_child(&feature_learner.output, 0);

// Initialize
feature_learner.init();
classifier.init();

// Training loop
for (auto& sample : training_data) {
    input_encoder.set_value(sample.value);
    classifier.set_label(sample.label);

    input_encoder.feedforward();
    feature_learner.feedforward(true);  // Learn features
    classifier.feedforward(true);        // Learn classification
}

// Inference
input_encoder.set_value(test_value);
input_encoder.feedforward();
feature_learner.feedforward(false);     // No learning
classifier.feedforward(false);          // No learning
auto predictions = classifier.get_probabilities();
```

### Sequence Learning

```cpp
// Create sequence learner
DiscreteTransformer encoder(10, 2048);
SequenceLearner seq_learner(512, 4, 8, 32, 20, 20, 2, 1);

// Connect with feedback loop
seq_learner.input.add_child(&encoder.output, 0);
seq_learner.context.add_child(&seq_learner.output, 1);  // Previous time step

seq_learner.init();

// Process sequence
for (auto& item : sequence) {
    encoder.set_value(item);
    encoder.feedforward();
    seq_learner.feedforward(true);

    double anomaly = seq_learner.get_anomaly_score();
    if (anomaly > 0.5) {
        // Unexpected sequence detected
    }
}
```

## API Reference Summary

### Block Base Class
- `feedforward(bool learn)` - Process input to output
- `feedback()` - Process output to input (reconstruction)
- `init()` - Initialize block
- `save(file)` / `load(file)` - Persistence
- `clear()` - Reset state

### Transformers (ScalarTransformer, DiscreteTransformer)
- `set_value(val)` - Set input value
- `get_value()` - Get current value
- `output` - BlockOutput with encoded pattern

### Learning Blocks (PatternPooler, PatternClassifier)
- `input` - BlockInput connection point
- `output` - BlockOutput with processed pattern
- `memory` - BlockMemory with learned connections

### Temporal Blocks (ContextLearner, SequenceLearner)
- `input` - BlockInput for current pattern
- `context` - BlockInput for contextual pattern
- `output` - BlockOutput with predictions
- `get_anomaly_score()` - Returns 0.0-1.0 anomaly metric
- `get_historical_count()` - Number of learned patterns

### BitArray
- `set_bit(b)` / `get_bit(b)` / `clear_bit(b)`
- `set_acts(indices)` / `get_acts()`
- `num_set()` / `num_similar(other)`
- `random_set_num(rng, n)` / `random_set_pct(rng, pct)`
- Binary operators: `~`, `&`, `|`, `^`, `==`, `!=`

## Extending Gnomics

To create a new block type:

1. Inherit from `Block` class
2. Override virtual methods as needed
3. Add `BlockInput`, `BlockOutput`, `BlockMemory` as needed
4. Implement `init()` to configure sizes
5. Implement `encode()` for computation
6. Implement `learn()` for weight updates
7. Add to `src/cpp/blocks/` and CMakeLists.txt

See `src/cpp/blocks/_template.hpp` and `src/cpp/blocks/_template.cpp` for starting point.

---

**This documentation was auto-generated by Claude Code based on comprehensive code review of the Gnomics framework.**
