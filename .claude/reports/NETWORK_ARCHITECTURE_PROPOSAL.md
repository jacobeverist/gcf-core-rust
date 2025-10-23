# Network Architecture Proposal

**Date**: October 22, 2025
**Issue**: Manual execution order and dependency tracking
**Goal**: Automatic block execution with dependency graph management

## Problem Statement

Currently, after blocks are connected via `add_child()`, execution must be handled manually:

```rust
// Current approach - manual execution order
encoder.execute(false)?;
pooler.execute(true)?;
classifier.execute(true)?;
```

**Issues**:
- Manual iteration through `execute()` calls
- User must track execution order
- Dependencies must be managed manually
- Error-prone for complex networks
- No cycle detection

## Proposed Solutions

### Option 1: Network/Assemblage with Dependency Graph (Recommended)

Most robust approach, similar to computational graphs in TensorFlow/PyTorch:

```rust
pub struct Network {
    blocks: HashMap<BlockId, Box<dyn Block>>,
    execution_order: Vec<BlockId>,
    is_built: bool,
}

impl Network {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            execution_order: Vec::new(),
            is_built: false,
        }
    }

    /// Add a block and return its ID for reference
    pub fn add_block(&mut self, block: Box<dyn Block>) -> BlockId {
        let id = BlockId::new();
        self.blocks.insert(id, block);
        self.is_built = false;  // Invalidate execution order
        id
    }

    /// Build dependency graph and compute execution order
    pub fn build(&mut self) -> Result<()> {
        // 1. Analyze block connections (via OutputAccess/InputAccess traits)
        // 2. Build directed acyclic graph (DAG)
        // 3. Detect cycles (error if found)
        // 4. Compute topological sort for execution order
        // 5. Validate all inputs are connected

        self.execution_order = self.topological_sort()?;
        self.is_built = true;
        Ok(())
    }

    /// Execute all blocks in dependency order
    pub fn execute(&mut self, learn: bool) -> Result<()> {
        if !self.is_built {
            return Err(Error::NetworkNotBuilt);
        }

        for block_id in &self.execution_order {
            self.blocks.get_mut(block_id).unwrap().execute(learn)?;
        }
        Ok(())
    }

    /// Run infinite loop (or bounded iterations)
    pub fn run(&mut self, learn: bool, max_iterations: Option<usize>) -> Result<()> {
        let mut iteration = 0;
        loop {
            self.execute(learn)?;

            iteration += 1;
            if let Some(max) = max_iterations {
                if iteration >= max {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Get mutable reference to a specific block for parameter updates
    pub fn get_block_mut<T: Block + 'static>(&mut self, id: BlockId) -> Option<&mut T> {
        self.blocks.get_mut(&id)
            .and_then(|b| b.as_any_mut().downcast_mut::<T>())
    }
}
```

**Pros**:
- ✅ Clean separation of concerns
- ✅ Automatic execution order
- ✅ Cycle detection
- ✅ Can optimize (parallel execution, etc.)
- ✅ Familiar pattern from ML frameworks

**Cons**:
- ❌ Blocks must be owned by Network (can't access directly)
- ❌ Need `as_any()` for downcasting to concrete types
- ❌ More complex initial setup

---

### Option 2: Builder Pattern with Execution Plan

More flexible ownership model using `Rc<RefCell<>>`:

```rust
pub struct NetworkBuilder {
    blocks: Vec<Rc<RefCell<dyn Block>>>,
}

impl NetworkBuilder {
    pub fn add<B: Block + 'static>(&mut self, block: B) -> Rc<RefCell<B>> {
        let wrapped = Rc::new(RefCell::new(block));
        self.blocks.push(wrapped.clone() as Rc<RefCell<dyn Block>>);
        wrapped
    }

    pub fn build(self) -> Result<ExecutionPlan> {
        ExecutionPlan::from_blocks(self.blocks)
    }
}

pub struct ExecutionPlan {
    execution_order: Vec<Rc<RefCell<dyn Block>>>,
}

impl ExecutionPlan {
    pub fn execute(&self, learn: bool) -> Result<()> {
        for block in &self.execution_order {
            block.borrow_mut().execute(learn)?;
        }
        Ok(())
    }
}
```

**Usage**:
```rust
let mut builder = NetworkBuilder::new();
let encoder = builder.add(ScalarTransformer::new(...));
let pooler = builder.add(PatternPooler::new(...));

// Still have direct access to encoder and pooler
pooler.borrow_mut().input_mut().add_child(encoder.borrow().output());

let plan = builder.build()?;
plan.execute(true)?;
```

**Pros**:
- ✅ User retains access to blocks
- ✅ Familiar Rc<RefCell<>> pattern
- ✅ Flexible connection setup

**Cons**:
- ❌ RefCell runtime borrow checking overhead
- ❌ Less encapsulation

---

### Option 3: Declarative Pipeline DSL

Macro-based approach for clean syntax:

```rust
network! {
    // Declare blocks
    let encoder = ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0);
    let pooler = PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);
    let classifier = PatternClassifier::new(3, 1024, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0);

    // Declare connections
    connect!(encoder -> pooler.input);
    connect!(pooler -> classifier.input);

    // Execution handled automatically
}
```

**Pros**:
- ✅ Very clean user API
- ✅ Compile-time validation possible

**Cons**:
- ❌ Complex macro implementation
- ❌ Harder to debug
- ❌ Less flexible for dynamic graphs

---

### Option 4: Hybrid Approach (Recommended Implementation)

Combine the best of Options 1 and 2:

```rust
use gnomics::{Network, InputAccess, OutputAccess};

fn main() -> Result<()> {
    // Create network
    let mut net = Network::new();

    // Add blocks and get typed handles
    let encoder_id = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    let pooler_id = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));

    // Connect blocks (network tracks dependencies automatically)
    net.connect(encoder_id, pooler_id)?;

    // Build execution graph
    net.build()?;

    // Training loop
    for epoch in 0..10 {
        for value in training_data {
            // Set input on encoder
            net.get_mut::<ScalarTransformer>(encoder_id)?.set_value(value);

            // Execute entire network in correct order
            net.execute(true)?;
        }
    }

    Ok(())
}
```

**Key Features**:
- Auto-detects dependencies from `add_child()` calls
- Topological sort for execution order
- Type-safe block access via generics
- Cycle detection
- Optional: Parallel execution of independent blocks

**Pros**:
- ✅ Clean API
- ✅ Type-safe block access
- ✅ Automatic dependency management
- ✅ Room for optimization

**Cons**:
- ❌ Moderate implementation complexity

---

## Implementation Sketch

Here's a minimal implementation to get started:

```rust
// src/network.rs

use crate::{Block, InputAccess, OutputAccess, Error, Result};
use std::collections::{HashMap, HashSet};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(u32);

impl BlockId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        BlockId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

pub struct Network {
    blocks: HashMap<BlockId, Box<dyn BlockWrapper>>,
    execution_order: Vec<BlockId>,
    is_built: bool,
}

// Wrapper to add type information
trait BlockWrapper: Block {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn block_id(&self) -> BlockId;
}

impl<T: Block + 'static> BlockWrapper for (BlockId, T) {
    fn as_any(&self) -> &dyn Any { &self.1 }
    fn as_any_mut(&mut self) -> &mut dyn Any { &mut self.1 }
    fn block_id(&self) -> BlockId { self.0 }
}

impl Network {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            execution_order: Vec::new(),
            is_built: false,
        }
    }

    pub fn add<B: Block + 'static>(&mut self, block: B) -> BlockId {
        let id = BlockId::new();
        self.blocks.insert(id, Box::new((id, block)));
        self.is_built = false;
        id
    }

    pub fn build(&mut self) -> Result<()> {
        // Build dependency graph by analyzing OutputAccess connections
        let mut graph: HashMap<BlockId, Vec<BlockId>> = HashMap::new();

        // TODO: Traverse blocks, examine their inputs, find which outputs they're connected to
        // This requires extending the BlockInput to track source block IDs

        // Compute topological sort
        self.execution_order = topological_sort(&graph)?;
        self.is_built = true;
        Ok(())
    }

    pub fn execute(&mut self, learn: bool) -> Result<()> {
        if !self.is_built {
            return Err(Error::Other("Network not built".into()));
        }

        for &id in &self.execution_order {
            self.blocks.get_mut(&id).unwrap().execute(learn)?;
        }
        Ok(())
    }

    pub fn get_mut<T: Block + 'static>(&mut self, id: BlockId) -> Result<&mut T> {
        self.blocks.get_mut(&id)
            .and_then(|b| b.as_any_mut().downcast_mut::<T>())
            .ok_or_else(|| Error::Other("Block not found or wrong type".into()))
    }
}

fn topological_sort(graph: &HashMap<BlockId, Vec<BlockId>>) -> Result<Vec<BlockId>> {
    // Kahn's algorithm for topological sort
    let mut in_degree: HashMap<BlockId, usize> = HashMap::new();
    let mut result = Vec::new();

    // Calculate in-degrees
    for deps in graph.values() {
        for &dep in deps {
            *in_degree.entry(dep).or_insert(0) += 1;
        }
    }

    // Find nodes with no incoming edges
    let mut queue: Vec<_> = graph.keys()
        .filter(|k| in_degree.get(k).copied().unwrap_or(0) == 0)
        .copied()
        .collect();

    while let Some(node) = queue.pop() {
        result.push(node);

        if let Some(neighbors) = graph.get(&node) {
            for &neighbor in neighbors {
                let degree = in_degree.get_mut(&neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push(neighbor);
                }
            }
        }
    }

    // Check for cycles
    if result.len() != graph.len() {
        return Err(Error::Other("Cycle detected in network".into()));
    }

    Ok(result)
}
```

---

## Required Changes to Existing Code

To support this architecture, you'd need to:

### 1. Add block ID tracking to BlockOutput

```rust
pub struct BlockOutput {
    source_block_id: Option<BlockId>,  // NEW
    // ... existing fields
}
```

### 2. Extend BlockInput to track connections

```rust
impl BlockInput {
    pub fn get_source_blocks(&self) -> Vec<BlockId> {
        self.children.iter()
            .filter_map(|child| child.borrow().source_block_id)
            .collect()
    }
}
```

### 3. Add `as_any()` to Block trait

For downcasting trait objects to concrete types:

```rust
pub trait Block {
    // ... existing methods ...

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

Then implement for all blocks:

```rust
impl Block for ScalarTransformer {
    // ... existing implementations ...

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
```

---

## Implementation Roadmap

### Phase 1: Basic Network with Manual Connection Tracking
- Implement `Network` struct
- Add `BlockId` generation
- Implement `add()` and `execute()` methods
- Manually specify dependencies via `connect(source_id, dest_id)`

**Timeline**: 1-2 days
**Difficulty**: Low

### Phase 2: Auto-Discovery of Dependencies
- Add `source_block_id` to `BlockOutput`
- Implement `get_source_blocks()` on `BlockInput`
- Auto-build dependency graph in `build()`
- Implement topological sort

**Timeline**: 2-3 days
**Difficulty**: Medium

### Phase 3: Parallel Execution Optimization
- Analyze dependency graph for parallelizable blocks
- Use `rayon` for parallel execution
- Benchmark performance improvements

**Timeline**: 3-5 days
**Difficulty**: High

### Phase 4: Optional Macro DSL
- Create `network!` macro for declarative syntax
- Implement compile-time validation
- Write macro documentation

**Timeline**: 3-4 days
**Difficulty**: High

---

## Example Usage Scenarios

### Scenario 1: Simple Pipeline

```rust
let mut net = Network::new();

let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));

net.connect(encoder, pooler)?;
net.build()?;

// Training
for value in training_data {
    net.get_mut::<ScalarTransformer>(encoder)?.set_value(value);
    net.execute(true)?;
}
```

### Scenario 2: Multi-Input Network

```rust
let mut net = Network::new();

let input_enc = net.add(DiscreteTransformer::new(10, 512, 2, 0));
let context_enc = net.add(DiscreteTransformer::new(5, 256, 2, 0));
let learner = net.add(ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

// ContextLearner has two inputs
net.connect_to_input(input_enc, learner, "input")?;
net.connect_to_input(context_enc, learner, "context")?;
net.build()?;

// Execute
for (in_val, ctx_val) in data {
    net.get_mut::<DiscreteTransformer>(input_enc)?.set_value(in_val);
    net.get_mut::<DiscreteTransformer>(context_enc)?.set_value(ctx_val);
    net.execute(true)?;
}
```

### Scenario 3: Recurrent Network (SequenceLearner)

```rust
let mut net = Network::new();

let encoder = net.add(DiscreteTransformer::new(10, 512, 2, 0));
let learner = net.add(SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

net.connect(encoder, learner)?;
// SequenceLearner connects to its own output internally for temporal feedback
net.build()?;

// Sequence learning
for epoch in 0..10 {
    for value in &[0, 1, 2, 3] {
        net.get_mut::<DiscreteTransformer>(encoder)?.set_value(*value);
        net.execute(true)?;
    }
}
```

---

## Alternative Considerations

### Option A: Keep Manual Execution

**Pros**:
- Simple, no new infrastructure
- User has complete control
- Easy to debug

**Cons**:
- Error-prone for large networks
- No cycle detection
- Boilerplate code

### Option B: Lightweight Helper

Just add a simple execution helper without full Network:

```rust
pub struct ExecutionOrder {
    blocks: Vec<Box<dyn Block>>,
}

impl ExecutionOrder {
    pub fn new(blocks: Vec<Box<dyn Block>>) -> Self {
        Self { blocks }
    }

    pub fn execute(&mut self, learn: bool) -> Result<()> {
        for block in &mut self.blocks {
            block.execute(learn)?;
        }
        Ok(())
    }
}

// Usage
let order = ExecutionOrder::new(vec![
    Box::new(encoder),
    Box::new(pooler),
    Box::new(classifier),
]);
order.execute(true)?;
```

**Pros**:
- Very simple
- No dependency analysis needed

**Cons**:
- Still manual ordering
- No cycle detection
- Blocks become owned

---

## Recommendation

**Implement Option 4 (Hybrid Approach) in Phases**:

1. **Phase 1**: Basic Network with manual `connect()` calls
2. **Phase 2**: Auto-discovery of dependencies via BlockInput/BlockOutput introspection
3. **Phase 3**: Parallel execution optimization
4. **Phase 4**: Optional macro DSL for convenience

This approach balances usability, performance, and implementation complexity while maintaining flexibility for future enhancements.

---

## Questions for Discussion

1. **Ownership Model**: Should Network own blocks, or use Rc<RefCell<>>?
2. **Block Access**: How often do users need to access individual blocks after building?
3. **Dynamic Networks**: Do we need to support adding/removing blocks at runtime?
4. **Serialization**: Should Network be serializable for saving/loading entire pipelines?
5. **Parallel Execution**: Is this a priority, or can it be deferred?

---

## Next Steps

1. Review this proposal and select preferred option
2. Create initial implementation plan
3. Start with Phase 1 prototype
4. Validate API ergonomics with real examples
5. Iterate based on usage patterns
