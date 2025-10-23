# BlockOutput Change Detection Design Options

## Problem Statement

Currently, `BlockOutput` uses a manual comparison in `store()` to detect if the state has changed:

```rust
pub fn store(&mut self) {
    // CRITICAL: Compare with previous state using fast BitField equality
    let prev_idx = self.idx(PREV);
    self.changed_flag = self.state != self.history[prev_idx];  // ~50ns overhead

    // Store state and change flag
    self.history[self.curr_idx] = self.state.clone();
    self.changes[self.curr_idx] = self.changed_flag;
}
```

**Performance Cost**: ~50ns per `store()` call for BitField comparison (1024 bits)

**Goal**: Eliminate this comparison overhead by automatically tracking modifications to `BitField` when they occur.

---

## Design Options

### **Option 1: Interior Mutability with Change Flag**

Wrap `BitField` in a change-tracking container that automatically sets a flag when any mutation occurs.

#### Implementation

```rust
pub struct TrackedBitField {
    inner: BitField,
    modified: Cell<bool>,
}

impl TrackedBitField {
    pub fn new(n: usize) -> Self {
        Self {
            inner: BitField::new(n),
            modified: Cell::new(false),
        }
    }

    pub fn set_bit(&mut self, b: usize) {
        self.inner.set_bit(b);
        self.modified.set(true);
    }

    pub fn clear_bit(&mut self, b: usize) {
        self.inner.clear_bit(b);
        self.modified.set(true);
    }

    // ... wrap all 18 mutable methods ...

    pub fn was_modified(&self) -> bool {
        self.modified.get()
    }

    pub fn reset_modified(&self) {
        self.modified.set(false);
    }

    // Immutable access doesn't set flag
    pub fn get_bit(&self, b: usize) -> u8 {
        self.inner.get_bit(b)
    }
}
```

#### Pros
- Zero overhead when not modified (no comparison)
- ~2-3ns overhead per mutation (one flag write)
- Clean API
- Type-safe (can't forget to check)

#### Cons
- Need to wrap all 18 mutable methods
- 1 extra byte per TrackedBitField
- Boilerplate code (could use macro)

#### Performance
- Current: ~50ns comparison on every `store()`
- With tracking: ~2-3ns per mutation + <1ns flag check in `store()`
- **Speedup**: ~10-25× depending on mutation frequency

---

### **Option 2: Dirty Flag in BlockOutput** (Simple)

Track modifications directly in `BlockOutput` by requiring explicit marking via RAII guard.

#### Implementation

```rust
impl BlockOutput {
    pub fn state_mut(&mut self) -> TrackedMut<'_, BitField> {
        TrackedMut {
            inner: &mut self.state,
            changed_flag: &mut self.changed_flag,
        }
    }
}

pub struct TrackedMut<'a, T> {
    inner: &'a mut T,
    changed_flag: &'a mut bool,
}

impl<'a> Drop for TrackedMut<'a, BitField> {
    fn drop(&mut self) {
        *self.changed_flag = true;
    }
}

impl<'a> Deref for TrackedMut<'a, BitField> {
    type Target = BitField;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a> DerefMut for TrackedMut<'a, BitField> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        *self.changed_flag = true; // Mark on any mutable access
        self.inner
    }
}
```

#### Usage

```rust
// Instead of: output.state.set_bit(5)
output.state_mut().set_bit(5); // Automatically marks changed
```

#### Pros
- Minimal code changes
- No wrapper type needed
- Automatic tracking via RAII

#### Cons
- Conservative: marks changed even for read-only `&mut` borrows
- Awkward API: `output.state_mut().set_bit(5)` instead of `output.state.set_bit(5)`
- Can't prevent direct field access if `state` field remains public

#### Performance
- Similar to Option 1: ~2ns overhead per mutable borrow
- **Speedup**: ~10-25×

---

### **Option 3: Manual Marking with Discipline** (Current approach)

Keep current design, rely on discipline to call `mark_changed()` after modifications.

#### Implementation

```rust
impl BlockOutput {
    pub fn mark_changed(&mut self) {
        self.changed_flag = true;
    }

    // Usage in blocks:
    // self.output.borrow_mut().state.set_bit(10);
    // self.output.borrow_mut().mark_changed();
}
```

#### Pros
- No API changes
- Zero overhead when correctly used
- Simple

#### Cons
- Easy to forget (not enforced by type system)
- Still need comparison in `store()` as fallback
- Not type-safe
- Defeats the purpose of automatic tracking

#### Performance
- No improvement over current approach

---

### **Option 4: Copy-on-Write with Hashing** (Over-engineered)

Use hash-based change detection (similar to React's memoization).

#### Implementation

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

impl BlockOutput {
    hash_cache: u64,

    pub fn store(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.state.hash(&mut hasher);
        let new_hash = hasher.finish();

        self.changed_flag = new_hash != self.hash_cache;
        self.hash_cache = new_hash;

        // ... rest of store
    }
}
```

#### Pros
- Automatic detection
- No wrapper needed
- No code changes to mutation sites

#### Cons
- Hash computation ~200-500ns (worse than current 50ns comparison)
- Still need to hash on every `store()`
- More complex
- Hash collisions possible (unlikely but not impossible)

#### Performance
- **Slowdown**: 4-10× worse than current approach
- Not recommended

---

### **Option 5: Version Numbers** ⭐ **RECOMMENDED**

Track a version number in `BitField` that increments on any modification.

#### Implementation

```rust
// src/bitfield.rs
use std::cell::Cell;

pub struct BitField {
    bv: BitVec<u32, Lsb0>,
    version: Cell<u64>,
}

impl BitField {
    pub fn new(n: usize) -> Self {
        Self {
            bv: BitVec::repeat(false, n),
            version: Cell::new(0),
        }
    }

    #[inline]
    pub fn set_bit(&mut self, b: usize) {
        debug_assert!(b < self.bv.len(), "bit index {} out of bounds (length: {})", b, self.bv.len());
        self.bv.set(b, true);
        self.increment_version();
    }

    #[inline]
    pub fn clear_bit(&mut self, b: usize) {
        debug_assert!(b < self.bv.len(), "bit index {} out of bounds (length: {})", b, self.bv.len());
        self.bv.set(b, false);
        self.increment_version();
    }

    // Apply to all 18 mutable methods:
    // - set_bit, clear_bit, toggle_bit, assign_bit
    // - set_range, clear_range, toggle_range
    // - set_all, clear_all, toggle_all
    // - set_bits, set_acts
    // - random_shuffle, random_set_num, random_set_pct
    // - resize, erase
    // - words_mut (trickier - see below)

    #[inline]
    fn increment_version(&self) {
        self.version.set(self.version.get().wrapping_add(1));
    }

    pub fn version(&self) -> u64 {
        self.version.get()
    }

    // For clone: preserve version or reset?
    // Option A: Reset version (treat clone as "new" object)
    // Option B: Copy version (preserve modification tracking)
    // Recommendation: Reset to 0 for fresh start
}

// src/block_output.rs
pub struct BlockOutput {
    pub state: BitField,
    history: Vec<BitField>,
    changes: Vec<bool>,
    changed_flag: bool,
    last_version: u64,  // NEW FIELD
    curr_idx: usize,
    id: u32,
    source_block_id: Option<BlockId>,
}

impl BlockOutput {
    pub fn new() -> Self {
        Self {
            state: BitField::new(0),
            history: Vec::new(),
            changes: Vec::new(),
            changed_flag: false,
            last_version: 0,  // NEW
            curr_idx: 0,
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            source_block_id: None,
        }
    }

    #[inline]
    pub fn store(&mut self) {
        // NEW: O(1) version comparison instead of O(n) BitField comparison
        let curr_version = self.state.version();
        self.changed_flag = curr_version != self.last_version;
        self.last_version = curr_version;

        // Store state and change flag (unchanged)
        self.history[self.curr_idx] = self.state.clone();
        self.changes[self.curr_idx] = self.changed_flag;
    }

    pub fn clear(&mut self) {
        self.state.clear_all();  // This increments version automatically
        self.changed_flag = true;

        for i in 0..self.history.len() {
            self.history[i].clear_all();
            self.changes[i] = true;
        }
    }
}
```

#### Mutable Methods to Update (18 total)

```rust
// Single bit operations
pub fn set_bit(&mut self, b: usize)         // ✅
pub fn clear_bit(&mut self, b: usize)       // ✅
pub fn toggle_bit(&mut self, b: usize)      // ✅
pub fn assign_bit(&mut self, b: usize, val: u8)  // ✅

// Range operations
pub fn set_range(&mut self, beg: usize, len: usize)    // ✅
pub fn clear_range(&mut self, beg: usize, len: usize)  // ✅
pub fn toggle_range(&mut self, beg: usize, len: usize) // ✅

// Bulk operations
pub fn set_all(&mut self)    // ✅
pub fn clear_all(&mut self)  // ✅
pub fn toggle_all(&mut self) // ✅

// Array operations
pub fn set_bits(&mut self, vals: &[u8])     // ✅
pub fn set_acts(&mut self, idxs: &[usize])  // ✅

// Random operations
pub fn random_shuffle<R: Rng>(&mut self, rng: &mut R)           // ✅
pub fn random_set_num<R: Rng>(&mut self, rng: &mut R, num: usize)  // ✅
pub fn random_set_pct<R: Rng>(&mut self, rng: &mut R, pct: f64)    // ✅

// Structure operations
pub fn resize(&mut self, n: usize)  // ✅
pub fn erase(&mut self)             // ✅

// Direct word access (TRICKY!)
pub fn words_mut(&mut self) -> &mut [Word]  // See below
```

#### Special Case: `words_mut()`

The `words_mut()` method returns a mutable slice, so we can't track when it's modified. Options:

1. **Conservative**: Always increment version when called
2. **Trust-based**: Don't increment, assume caller will use it correctly
3. **Guard pattern**: Return a RAII guard that increments on drop
4. **Deprecate**: Mark as unsafe or remove from public API

**Recommendation**: Option 1 (conservative) - increment version when `words_mut()` is called:

```rust
pub fn words_mut(&mut self) -> &mut [Word] {
    self.increment_version();  // Conservative: assume modification
    self.bv.as_raw_mut_slice()
}
```

#### Pros
- **Precise**: Only marks changed when actually modified
- **Fast**: Store comparison goes from ~50ns to <2ns (u64 comparison)
- **Clean API**: No changes to calling code
- **Type-safe**: Automatic tracking, can't forget
- **Minimal overhead**: ~1-2ns per mutation for version increment

#### Cons
- Version counter overhead (~1-2ns per mutation)
- Need to add `increment_version()` to all 18 mutable methods
- Risk of version overflow (negligible with u64: ~584 years at 1GHz mutation rate)
- `words_mut()` requires conservative handling

#### Performance Analysis

**Current approach:**
- Every `store()`: ~50ns BitField comparison (word-by-word memcmp)

**With versioning:**
- Every mutation: +1-2ns (version increment)
- Every `store()`: <2ns (u64 comparison)

**Best case** (no mutations between stores):
- Current: 50ns
- Versioned: 2ns
- **Speedup: 25×**

**Typical case** (5 mutations per store):
- Current: 50ns
- Versioned: 5 × 2ns + 2ns = 12ns
- **Speedup: 4×**

**Worst case** (many mutations):
- Break-even at ~25 mutations between stores
- Still better because version increment is cheaper than comparison

---

## Recommendation: **Option 5 (Version Numbers)**

**Why:**

1. **Best Performance**: 4-25× speedup depending on mutation frequency
2. **Correctness**: Precise tracking, only marks changed when truly modified
3. **Clean API**: Zero changes to calling code
4. **Type Safety**: Automatic tracking built into type system
5. **Minimal Overhead**: ~1-2ns per mutation, negligible in practice

**Implementation Effort**: Medium
- Add `version: Cell<u64>` field to `BitField` (1 line)
- Add `increment_version()` calls to 18 mutable methods (~18 lines)
- Add `last_version: u64` field to `BlockOutput` (1 line)
- Update `store()` to use version comparison (2 lines)
- Update tests as needed

**Trade-offs:**
- Small memory overhead: +8 bytes per BitField (negligible)
- Slightly more complex BitField implementation
- Need to handle `words_mut()` conservatively

---

## Comparison Matrix

| Option | Store Overhead | Mutation Overhead | API Changes | Type Safety | Complexity |
|--------|---------------|-------------------|-------------|-------------|------------|
| 1. TrackedBitField | <1ns | ~2-3ns | Minor (new type) | ✅ High | Medium |
| 2. Dirty Flag RAII | <1ns | ~2ns | Moderate (state_mut()) | ✅ High | Low |
| 3. Manual Marking | 50ns (fallback) | 0ns | None | ❌ Low | Low |
| 4. Hashing | 200-500ns | 0ns | None | ✅ High | High |
| **5. Version Numbers** | **<2ns** | **~1-2ns** | **None** | **✅ High** | **Medium** |

---

## Next Steps

1. Implement Option 5 (Version Numbers) in `BitField`
2. Update `BlockOutput` to use version-based change detection
3. Run benchmarks to validate performance improvement
4. Update tests to verify correctness
5. Update documentation to reflect new approach

---

## References

- Current implementation: `src/block_output.rs:238-247`
- BitField mutable methods: `src/bitfield.rs:110,116,142,164,175,185,202,214,226,239,244,251,277,291,480,493,516,538`
- Change tracking optimization: `CLAUDE.md` (lines 8-14)
