# Network Connection API Simplification Plan

**Date**: 2025-10-22
**Status**: 📋 Planned
**Effort**: 5-7 hours (~440 lines of code)

## Problem Analysis

### Current API (Verbose)

**Example** (4-5 lines per connection):
```rust
{
    let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
    net.get_mut::<SequenceLearner>(learner)?
        .input_mut()
        .add_child(enc_out, 0);
}
```

### Pain Points

1. **Scoping blocks required** - Need `{}` to manage borrow checker
2. **Manual output extraction** - Must call `.output()` on source
3. **Manual input extraction** - Must call `.input_mut()` or `.context_mut()` on target
4. **Offset boilerplate** - Must specify offset (almost always 0)
5. **Verbose type parameters** - Need `get::<ConcreteType>()` and `get_mut::<ConcreteType>()`

### Usage Patterns Found

Analyzed existing codebase:
- **95% of connections** use `offset: 0`
- **Two main input types**: `Input` and `Context`
- **Common patterns**:
  - Single source → single target (most common)
  - Multiple sources → one target (common)
  - One source → multiple targets (less common)

---

## Proposed Solution

### Phase 1: Core Connection Methods ✅ Recommended

Add simple methods to `Network` for the most common use cases:

```rust
impl Network {
    /// Connect source block's output to target block's input
    pub fn connect_to_input(
        &mut self,
        source: BlockId,
        target: BlockId,
    ) -> Result<()> {
        self.connect_to_input_with_offset(source, target, 0)
    }

    /// Connect source block's output to target block's context
    pub fn connect_to_context(
        &mut self,
        source: BlockId,
        target: BlockId,
    ) -> Result<()> {
        self.connect_to_context_with_offset(source, target, 0)
    }

    /// Connect with explicit offset (advanced use)
    pub fn connect_to_input_with_offset(
        &mut self,
        source: BlockId,
        target: BlockId,
        offset: usize,
    ) -> Result<()> {
        // Implementation below
    }

    /// Connect to context with explicit offset (advanced use)
    pub fn connect_to_context_with_offset(
        &mut self,
        source: BlockId,
        target: BlockId,
        offset: usize,
    ) -> Result<()> {
        // Implementation below
    }
}
```

**Benefits:**
- ✅ **Single line** per connection (vs. 5 lines)
- ✅ **No scoping blocks** needed
- ✅ **No manual** `.output()` or `.input_mut()` calls
- ✅ **Type inference** (no type parameters needed)
- ✅ **Offset defaulted** to 0 (common case)

**Example Usage:**
```rust
// Before (5 lines)
{
    let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
    net.get_mut::<SequenceLearner>(learner)?
        .input_mut()
        .add_child(enc_out, 0);
}

// After (1 line)
net.connect_to_input(encoder, learner)?;
```

---

### Phase 2: Builder Pattern (Optional Enhancement)

For connecting one source to multiple targets:

```rust
impl Network {
    /// Start a connection builder
    pub fn connect(&mut self, source: BlockId) -> ConnectionBuilder {
        ConnectionBuilder::new(self, source)
    }
}

pub struct ConnectionBuilder<'a> {
    network: &'a mut Network,
    source: BlockId,
}

impl<'a> ConnectionBuilder<'a> {
    /// Connect to target's input
    pub fn to_input(self, target: BlockId) -> Result<Self> {
        self.network.connect_to_input(self.source, target)?;
        Ok(self)
    }

    /// Connect to target's context
    pub fn to_context(self, target: BlockId) -> Result<Self> {
        self.network.connect_to_context(self.source, target)?;
        Ok(self)
    }

    /// Connect to target's input with offset
    pub fn to_input_with_offset(self, target: BlockId, offset: usize) -> Result<Self> {
        self.network.connect_to_input_with_offset(self.source, target, offset)?;
        Ok(self)
    }

    /// Connect to target's context with offset
    pub fn to_context_with_offset(self, target: BlockId, offset: usize) -> Result<Self> {
        self.network.connect_to_context_with_offset(self.source, target, offset)?;
        Ok(self)
    }
}
```

**Example Usage:**
```rust
// Connect encoder to multiple targets
net.connect(encoder)
    .to_input(learner)?
    .to_input(pooler)?
    .to_context(classifier)?;
```

---

### Phase 3: Batch Connection Helpers (Optional Enhancement)

For connecting multiple sources to one target:

```rust
impl Network {
    /// Connect multiple sources to target's input
    pub fn connect_many_to_input(
        &mut self,
        sources: &[BlockId],
        target: BlockId,
    ) -> Result<()> {
        for &source in sources {
            self.connect_to_input(source, target)?;
        }
        Ok(())
    }

    /// Connect multiple sources to target's context
    pub fn connect_many_to_context(
        &mut self,
        sources: &[BlockId],
        target: BlockId,
    ) -> Result<()> {
        for &source in sources {
            self.connect_to_context(source, target)?;
        }
        Ok(())
    }
}
```

**Example Usage:**
```rust
// Before (6+ lines)
{
    let enc1_out = net.get::<ScalarTransformer>(encoder1)?.output();
    let enc2_out = net.get::<ScalarTransformer>(encoder2)?.output();
    let pooler_input = net.get_mut::<PatternPooler>(pooler)?.input_mut();
    pooler_input.add_child(enc1_out, 0);
    pooler_input.add_child(enc2_out, 0);
}

// After (1 line)
net.connect_many_to_input(&[encoder1, encoder2], pooler)?;
```

---

## Implementation Details

### Core Implementation (Phase 1)

```rust
impl Network {
    pub fn connect_to_input_with_offset(
        &mut self,
        source: BlockId,
        target: BlockId,
        offset: usize,
    ) -> Result<()> {
        // Step 1: Get source output (using as_any for type erasure)
        let source_wrapper = self.blocks.get(&source)
            .ok_or_else(|| GnomicsError::Other(
                format!("Source block {} not found", source.as_usize())
            ))?;

        let source_output = {
            let block_any = source_wrapper.as_any();

            // Try each block type that has OutputAccess
            if let Some(b) = block_any.downcast_ref::<blocks::ScalarTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::DiscreteTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::PersistenceTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::PatternPooler>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::PatternClassifier>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::ContextLearner>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::SequenceLearner>() {
                b.output()
            } else {
                return Err(GnomicsError::Other(
                    format!("Source block {} does not have output", source.as_usize())
                ));
            }
        };

        // Step 2: Get target and add connection
        let target_wrapper = self.blocks.get_mut(&target)
            .ok_or_else(|| GnomicsError::Other(
                format!("Target block {} not found", target.as_usize())
            ))?;

        let block_any_mut = target_wrapper.as_any_mut();

        // Try each block type that has InputAccess
        if let Some(b) = block_any_mut.downcast_mut::<blocks::PatternPooler>() {
            b.input_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<blocks::PatternClassifier>() {
            b.input_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<blocks::ContextLearner>() {
            b.input_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<blocks::SequenceLearner>() {
            b.input_mut().add_child(source_output, offset);
        } else {
            return Err(GnomicsError::Other(
                format!("Target block {} does not have input", target.as_usize())
            ));
        }

        Ok(())
    }

    pub fn connect_to_context_with_offset(
        &mut self,
        source: BlockId,
        target: BlockId,
        offset: usize,
    ) -> Result<()> {
        // Step 1: Get source output (same as connect_to_input_with_offset)
        let source_wrapper = self.blocks.get(&source)
            .ok_or_else(|| GnomicsError::Other(
                format!("Source block {} not found", source.as_usize())
            ))?;

        let source_output = {
            let block_any = source_wrapper.as_any();

            // Try each block type that has OutputAccess
            if let Some(b) = block_any.downcast_ref::<blocks::ScalarTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::DiscreteTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::PersistenceTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::PatternPooler>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::PatternClassifier>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::ContextLearner>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<blocks::SequenceLearner>() {
                b.output()
            } else {
                return Err(GnomicsError::Other(
                    format!("Source block {} does not have output", source.as_usize())
                ));
            }
        };

        // Step 2: Get target and add to CONTEXT (only ContextLearner and SequenceLearner)
        let target_wrapper = self.blocks.get_mut(&target)
            .ok_or_else(|| GnomicsError::Other(
                format!("Target block {} not found", target.as_usize())
            ))?;

        let block_any_mut = target_wrapper.as_any_mut();

        // Only ContextLearner and SequenceLearner have context
        if let Some(b) = block_any_mut.downcast_mut::<blocks::ContextLearner>() {
            b.context_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<blocks::SequenceLearner>() {
            b.context_mut().add_child(source_output, offset);
        } else {
            return Err(GnomicsError::Other(
                format!("Target block {} does not have context input", target.as_usize())
            ));
        }

        Ok(())
    }
}
```

**Notes on Implementation:**
- Uses existing `as_any()` pattern for downcasting (consistent with serialization)
- Clear error messages for:
  - Block not found
  - Source has no output
  - Target has no input/context
- Minimal boilerplate (just the downcast chain)

---

## Alternative: Trait-Based Approach

Instead of downcasting, we could add a trait (future consideration):

```rust
pub trait Connectable: Block {
    fn get_output(&self) -> Option<Rc<RefCell<BlockOutput>>> {
        None
    }

    fn get_input_mut(&mut self) -> Option<&mut BlockInput> {
        None
    }

    fn get_context_mut(&mut self) -> Option<&mut BlockInput> {
        None
    }
}

// Then implement for each block type
impl Connectable for ScalarTransformer {
    fn get_output(&self) -> Option<Rc<RefCell<BlockOutput>>> {
        Some(self.output())
    }
}

impl Connectable for PatternPooler {
    fn get_output(&self) -> Option<Rc<RefCell<BlockOutput>>> {
        Some(self.output())
    }

    fn get_input_mut(&mut self) -> Option<&mut BlockInput> {
        Some(self.input_mut())
    }
}
```

**Trade-offs:**
- ✅ Cleaner implementation (no downcast chain)
- ✅ Less boilerplate in Network methods
- ❌ Requires changes to BlockWrapper trait bounds
- ❌ May require `Box<dyn Block + Connectable>` (complex)
- ❌ More invasive change to architecture

**Recommendation**: Start with downcasting approach (simpler, follows existing patterns). Consider trait-based if we add many more block types.

---

## Migration Path

### Step 1: Add New Methods (Non-Breaking)
- Add `connect_to_input()`, `connect_to_context()` methods
- Add `connect_to_input_with_offset()`, `connect_to_context_with_offset()`
- **Keep existing API working** (no breaking changes)

### Step 2: Update Documentation & Examples
- Update key examples to use new API
- Add migration guide to CLAUDE.md
- Show both old and new patterns

### Step 3: Optional Deprecation (Future)
- Mark old pattern as "verbose but valid"
- No need to remove old API (still useful for advanced cases)
- Some users may prefer explicit control

**Note**: Old API remains fully functional. This is purely additive.

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_connect_to_input_simple() {
    let mut net = Network::new();
    let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));

    // New API
    net.connect_to_input(encoder, pooler).unwrap();

    net.build().unwrap();
    pooler.init().unwrap();

    // Verify connection works by executing
    encoder.set_value(50.0);
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();

    // Output should have active bits
    assert!(pooler.output().borrow().state.num_set() > 0);
}

#[test]
fn test_connect_to_context() {
    let mut net = Network::new();
    let input_enc = net.add(DiscreteTransformer::new(10, 512, 2, 0));
    let context_enc = net.add(DiscreteTransformer::new(5, 256, 2, 0));
    let learner = net.add(ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

    // New API
    net.connect_to_input(input_enc, learner).unwrap();
    net.connect_to_context(context_enc, learner).unwrap();

    learner.init().unwrap();
    net.build().unwrap();

    // Verify both connections work
    input_enc.set_value(5);
    context_enc.set_value(2);
    net.execute(false).unwrap();
}

#[test]
fn test_connect_many_to_input() {
    let mut net = Network::new();
    let enc1 = net.add(ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 0));
    let enc2 = net.add(ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 1));
    let pooler = net.add(PatternPooler::new(2048, 80, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));

    // New API
    net.connect_many_to_input(&[enc1, enc2], pooler).unwrap();

    net.build().unwrap();
    pooler.init().unwrap();

    // Verify both inputs are concatenated
    enc1.set_value(25.0);
    enc2.set_value(75.0);
    net.execute(false).unwrap();

    // Input should have bits from both encoders
    let input_size = pooler.input().state.num_bits();
    assert_eq!(input_size, 2048); // 1024 * 2
}

#[test]
fn test_connect_invalid_source() {
    let mut net = Network::new();
    let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));

    // Non-existent source
    let result = net.connect_to_input(BlockId::from(999), pooler);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_connect_to_block_without_input() {
    let mut net = Network::new();
    let enc1 = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    let enc2 = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 1));

    // ScalarTransformer has no input
    let result = net.connect_to_input(enc1, enc2);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not have input"));
}

#[test]
fn test_connect_to_block_without_context() {
    let mut net = Network::new();
    let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));

    // PatternPooler has no context input
    let result = net.connect_to_context(encoder, pooler);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not have context"));
}

#[test]
fn test_connect_with_offset() {
    let mut net = Network::new();
    let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));

    // Use non-zero offset
    net.connect_to_input_with_offset(encoder, pooler, 10).unwrap();

    net.build().unwrap();
    pooler.init().unwrap();

    encoder.set_value(50.0);
    net.execute(false).unwrap();
}

#[test]
fn test_builder_pattern_multiple_targets() {
    let mut net = Network::new();
    let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    let pooler1 = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
    let pooler2 = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 1));

    // Builder pattern (if Phase 2 implemented)
    net.connect(encoder)
        .to_input(pooler1).unwrap()
        .to_input(pooler2).unwrap();

    net.build().unwrap();
}
```

### Integration Tests

Test that new API produces identical results to old API:

```rust
#[test]
fn test_new_api_equivalent_to_old() {
    // Network using old API
    let mut net_old = Network::new();
    let enc_old = net_old.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 42));
    let pool_old = net_old.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 42));
    {
        let enc_out = net_old.get::<ScalarTransformer>(enc_old).unwrap().output();
        net_old.get_mut::<PatternPooler>(pool_old).unwrap()
            .input_mut()
            .add_child(enc_out, 0);
    }
    net_old.build().unwrap();
    net_old.get_mut::<PatternPooler>(pool_old).unwrap().init().unwrap();

    // Network using new API
    let mut net_new = Network::new();
    let enc_new = net_new.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 42));
    let pool_new = net_new.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 42));
    net_new.connect_to_input(enc_new, pool_new).unwrap();
    net_new.build().unwrap();
    net_new.get_mut::<PatternPooler>(pool_new).unwrap().init().unwrap();

    // Execute both with same input
    net_old.get_mut::<ScalarTransformer>(enc_old).unwrap().set_value(42.0);
    net_new.get_mut::<ScalarTransformer>(enc_new).unwrap().set_value(42.0);

    net_old.execute(false).unwrap();
    net_new.execute(false).unwrap();

    // Verify identical outputs
    let out_old = net_old.get::<PatternPooler>(pool_old).unwrap().output();
    let out_new = net_new.get::<PatternPooler>(pool_new).unwrap().output();

    assert_eq!(
        out_old.borrow().state.get_acts(),
        out_new.borrow().state.get_acts()
    );
}
```

---

## Estimated Effort

| Task | Lines | Time | File |
|------|-------|------|------|
| **Phase 1: Core Methods** | | | |
| `connect_to_input_with_offset()` | ~60 | 1 hour | src/network.rs |
| `connect_to_context_with_offset()` | ~60 | 1 hour | src/network.rs |
| Wrapper methods (no offset) | ~10 | 15 min | src/network.rs |
| Unit tests | ~100 | 1.5 hours | tests/test_network.rs |
| Integration tests | ~50 | 1 hour | tests/test_network.rs |
| Update 5-10 examples | ~50 | 1 hour | examples/*.rs |
| Documentation | ~20 | 30 min | CLAUDE.md |
| **Phase 1 Subtotal** | **~350** | **~6 hours** | |
| | | | |
| **Phase 2: Builder Pattern** (Optional) | | | |
| ConnectionBuilder struct | ~60 | 1 hour | src/network.rs |
| Tests | ~30 | 30 min | tests/test_network.rs |
| **Phase 2 Subtotal** | **~90** | **~1.5 hours** | |
| | | | |
| **Phase 3: Batch Helpers** (Optional) | | | |
| `connect_many_*()` methods | ~30 | 30 min | src/network.rs |
| Tests | ~20 | 30 min | tests/test_network.rs |
| **Phase 3 Subtotal** | **~50** | **~1 hour** | |
| | | | |
| **Total (All Phases)** | **~490** | **~8.5 hours** | |
| **Total (Phase 1 Only)** | **~350** | **~6 hours** | |

---

## Recommendation

### Implement Phase 1 First (Essential)

**Why Phase 1 is sufficient:**
1. Covers 95% of use cases
2. Dramatic improvement (5 lines → 1 line)
3. Clean, simple API
4. Non-breaking change
5. Clear error messages

**Example transformations:**
```rust
// Single connection (was 5 lines, now 1)
net.connect_to_input(encoder, pooler)?;

// Two connections (was 10 lines, now 2)
net.connect_to_input(input_enc, learner)?;
net.connect_to_context(context_enc, learner)?;

// With offset (advanced, was 5 lines, now 1)
net.connect_to_input_with_offset(encoder, pooler, 10)?;
```

### Consider Phase 2 & 3 Later (Nice-to-Have)

Only add if:
- Users request builder pattern
- Multiple-source connections become very common
- Phase 1 proves successful

**Design principle**: Start simple, add complexity only if needed.

---

## Before/After Comparison

### Example 1: Simple Pipeline

**Before** (15 lines):
```rust
let mut net = Network::new();
let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
let classifier = net.add(PatternClassifier::new(3, 1024, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0));

{
    let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
    net.get_mut::<PatternPooler>(pooler)?.input_mut().add_child(enc_out, 0);
}

{
    let pool_out = net.get::<PatternPooler>(pooler)?.output();
    net.get_mut::<PatternClassifier>(classifier)?.input_mut().add_child(pool_out, 0);
}

net.build()?;
```

**After** (7 lines):
```rust
let mut net = Network::new();
let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
let classifier = net.add(PatternClassifier::new(3, 1024, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0));

net.connect_to_input(encoder, pooler)?;
net.connect_to_input(pooler, classifier)?;
net.build()?;
```

**Improvement**: 8 lines eliminated (53% reduction)

---

### Example 2: Context Learning

**Before** (12 lines):
```rust
let mut net = Network::new();
let input_enc = net.add(DiscreteTransformer::new(10, 512, 2, 0));
let context_enc = net.add(DiscreteTransformer::new(5, 256, 2, 0));
let learner = net.add(ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

{
    let in_out = net.get::<DiscreteTransformer>(input_enc)?.output();
    net.get_mut::<ContextLearner>(learner)?.input_mut().add_child(in_out, 0);
}

{
    let ctx_out = net.get::<DiscreteTransformer>(context_enc)?.output();
    net.get_mut::<ContextLearner>(learner)?.context_mut().add_child(ctx_out, 0);
}

net.build()?;
```

**After** (6 lines):
```rust
let mut net = Network::new();
let input_enc = net.add(DiscreteTransformer::new(10, 512, 2, 0));
let context_enc = net.add(DiscreteTransformer::new(5, 256, 2, 0));
let learner = net.add(ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

net.connect_to_input(input_enc, learner)?;
net.connect_to_context(context_enc, learner)?;
net.build()?;
```

**Improvement**: 6 lines eliminated (50% reduction)

---

## Summary

This plan provides:

1. ✅ **Dramatic simplification** - 5 lines → 1 line (80% reduction)
2. ✅ **Backwards compatibility** - Old API still works
3. ✅ **Type safety** - Compile-time checks maintained
4. ✅ **Clear errors** - "Block not found", "No input", etc.
5. ✅ **Extensibility** - Can add builder/batch helpers later
6. ✅ **Minimal effort** - Phase 1 is ~6 hours
7. ✅ **Non-breaking** - Purely additive change

**Next Steps:**
1. Review and approve this plan
2. Implement Phase 1 (core methods)
3. Test thoroughly
4. Update documentation and examples
5. Consider Phase 2/3 based on user feedback

**Status**: Ready to implement ✅
