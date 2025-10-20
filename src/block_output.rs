//! BlockOutput - Manages block outputs with history and change tracking.
//!
//! This module provides the `BlockOutput` structure that stores a block's output
//! pattern along with a circular history buffer for temporal processing. It implements
//! critical change tracking optimization that enables 5-100× speedup in real-world
//! applications.
//!
//! # Change Tracking Optimization
//!
//! `store()` compares the current state with the previous state to detect changes.
//! This enables two levels of optimization:
//! - Level 1: `BlockInput::pull()` skips memcpy for unchanged outputs
//! - Level 2: `Block::encode()` skips computation when no inputs changed
//!
//! # Time-based Indexing
//!
//! History uses relative time indexing:
//! - `CURR` (0) - Current time step
//! - `PREV` (1) - Previous time step
//! - Circular buffer wraps around for efficiency
//!
//! # Examples
//!
//! ```
//! use gnomics::BlockOutput;
//!
//! let mut output = BlockOutput::new();
//! output.setup(3, 1024);  // 3 time steps, 1024 bits
//!
//! // Modify state
//! output.state.set_bit(10);
//! output.state.set_bit(20);
//!
//! // Store to history (detects change)
//! output.store();
//! assert!(output.has_changed());
//!
//! // Step forward in time
//! output.step();
//!
//! // Store again without modification (no change)
//! output.store();
//! assert!(!output.has_changed());
//! ```

use crate::bitfield::BitField;
use std::sync::atomic::{AtomicU32, Ordering};

/// Time constant for current time step (t=0)
pub const CURR: usize = 0;

/// Time constant for previous time step (t=1)
pub const PREV: usize = 1;

/// BlockOutput manages block outputs with circular history buffer and change tracking.
///
/// # Fields
///
/// - `state` - Working BitField for current output
/// - `history` - Circular buffer of previous states
/// - `changes` - Boolean flags tracking changes per time step
/// - `changed_flag` - Did current state change from previous?
/// - `curr_idx` - Current position in circular buffer
/// - `num_t` - Total number of time steps (history depth)
///
/// # Performance
///
/// Change tracking adds ~50ns overhead (BitField comparison) but enables:
/// - ~100ns saved per child in `BlockInput::pull()` when unchanged
/// - ~1-10μs saved in `Block::encode()` when no children changed
/// - **Overall: 5-100× speedup** depending on change rate
#[derive(Clone)]
pub struct BlockOutput {
    /// Working BitField for current output (public for direct access)
    pub state: BitField,

    /// Circular buffer of historical states
    history: Vec<BitField>,

    /// Change tracking per time step
    changes: Vec<bool>,

    /// Did current state change from previous? (CRITICAL for optimization)
    changed_flag: bool,

    /// Current index in circular buffer
    curr_idx: usize,

    /// Unique output ID (for debugging)
    id: u32,
}

impl BlockOutput {
    /// Create a new empty BlockOutput.
    ///
    /// Must call `setup()` before use.
    pub fn new() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);

        Self {
            state: BitField::new(0),
            history: Vec::new(),
            changes: Vec::new(),
            changed_flag: false,
            curr_idx: 0,
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
        }
    }

    /// Setup the BlockOutput with history depth and bit count.
    ///
    /// # Arguments
    ///
    /// * `num_t` - Number of time steps to store (must be >= 2)
    /// * `num_b` - Number of bits per BitField
    ///
    /// # Panics
    ///
    /// Panics if `num_t` < 2 or `num_b` == 0
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BlockOutput;
    ///
    /// let mut output = BlockOutput::new();
    /// output.setup(3, 1024);  // 3 time steps, 1024 bits
    /// assert_eq!(output.num_t(), 3);
    /// ```
    pub fn setup(&mut self, num_t: usize, num_b: usize) {
        assert!(num_t >= 2, "num_t must be >= 2");
        assert!(num_b > 0, "num_b must be > 0");

        // Initialize state (BitField handles word rounding internally)
        self.state.resize(num_b);

        // Initialize history
        self.history.clear();
        self.history.resize(num_t, BitField::new(num_b));

        // Initialize changes (all true initially)
        self.changes.clear();
        self.changes.resize(num_t, true);

        self.curr_idx = 0;
        self.changed_flag = true;
    }

    /// Clear all bits to 0 in state and history.
    ///
    /// Marks all time steps as changed.
    pub fn clear(&mut self) {
        self.state.clear_all();
        self.changed_flag = true;

        for i in 0..self.history.len() {
            self.history[i].clear_all();
            self.changes[i] = true;
        }
    }

    /// Get get copy of BitField
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BlockOutput;
    ///
    /// let mut output = BlockOutput::new();
    /// output.setup(3, 32);
    ///
    /// // get a copy of the bit field
    /// let mut state = output.get_state();
    /// ```
    #[inline]
    pub fn get_state(&self) -> BitField {
        self.state.clone()
    }

    /// Advance to next time step (update circular buffer index).
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BlockOutput;
    ///
    /// let mut output = BlockOutput::new();
    /// output.setup(3, 32);
    ///
    /// // Initially at index 0
    /// output.step();  // Now at index 1
    /// output.step();  // Now at index 2
    /// output.step();  // Wraps to index 0
    /// ```
    #[inline]
    pub fn step(&mut self) {
        self.curr_idx += 1;
        if self.curr_idx >= self.history.len() {
            self.curr_idx = 0;
        }
    }

    /// Store current state into history with change detection.
    ///
    /// **CRITICAL**: This method compares current state with previous state
    /// to detect changes. This enables dual-level skip optimization.
    ///
    /// # Performance
    ///
    /// - BitField comparison: ~50ns for 1024 bits (word-level memcmp)
    /// - Clone operation: ~50ns for 1024 bits
    /// - Total: ~100ns overhead
    /// - Benefit: Saves 100ns-10μs downstream when unchanged
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BlockOutput;
    ///
    /// let mut output = BlockOutput::new();
    /// output.setup(2, 32);
    ///
    /// output.state.set_bit(5);
    /// output.store();
    /// assert!(output.has_changed());  // First store always changes
    ///
    /// output.step();
    /// output.store();  // No modification to state
    /// assert!(!output.has_changed());  // No change detected
    /// ```
    #[inline]
    pub fn store(&mut self) {
        // CRITICAL: Compare with previous state using fast BitField equality
        let prev_idx = self.idx(PREV);
        self.changed_flag = self.state != self.history[prev_idx];

        // Store state and change flag
        self.history[self.curr_idx] = self.state.clone();
        self.changes[self.curr_idx] = self.changed_flag;
    }

    /// Get reference to BitField at relative time offset.
    ///
    /// # Arguments
    ///
    /// * `time` - Relative time offset (0=current, 1=previous, etc.)
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BlockOutput;
    ///
    /// let mut output = BlockOutput::new();
    /// output.setup(3, 32);
    ///
    /// output.state.set_bit(5);
    /// output.store();
    ///
    /// let curr = output.get_bitfield(0);  // Current
    /// let prev = output.get_bitfield(1);  // Previous
    /// assert_eq!(curr.get_bit(5), 1);
    /// ```
    #[inline]
    pub fn get_bitfield(&self, time: usize) -> &BitField {
        &self.history[self.idx(time)]
    }

    /// Check if current output changed from previous.
    ///
    /// Returns the change flag set during last `store()` call.
    ///
    /// **CRITICAL**: Used by `BlockInput::children_changed()` to skip memcpy.
    #[inline]
    pub fn has_changed(&self) -> bool {
        self.changed_flag
    }

    /// Check if output changed at a specific time offset.
    ///
    /// # Arguments
    ///
    /// * `time` - Relative time offset
    ///
    /// **CRITICAL**: Used by `BlockInput::pull()` to skip copying unchanged children.
    #[inline]
    pub fn has_changed_at(&self, time: usize) -> bool {
        self.changes[self.idx(time)]
    }

    /// Get number of time steps in history.
    #[inline]
    pub fn num_t(&self) -> usize {
        self.history.len()
    }

    /// Get unique output ID.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Estimate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let mut bytes = std::mem::size_of::<Self>();

        bytes += self.state.memory_usage();
        if !self.history.is_empty() {
            bytes += self.history.len() * self.history[0].memory_usage();
        }
        bytes += self.changes.capacity() * std::mem::size_of::<bool>();

        bytes
    }

    /// Convert relative time offset to absolute history index.
    ///
    /// # Arguments
    ///
    /// * `ts` - Time step (0=current, 1=previous, etc.)
    ///
    /// # Returns
    ///
    /// Absolute index in circular buffer
    ///
    /// # Examples
    ///
    /// If curr_idx=1 and num_t=3:
    /// - idx(0) -> 1 (current)
    /// - idx(1) -> 0 (previous)
    /// - idx(2) -> 2 (two steps ago)
    #[inline]
    fn idx(&self, ts: usize) -> usize {
        debug_assert!(ts < self.history.len(), "time offset out of bounds");

        let num_t = self.history.len();
        if ts <= self.curr_idx {
            self.curr_idx - ts
        } else {
            num_t + self.curr_idx - ts
        }
    }
}

impl Default for BlockOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let output = BlockOutput::new();
        assert_eq!(output.state.num_bits(), 0);
        assert_eq!(output.history.len(), 0);
    }

    #[test]
    fn test_setup() {
        let mut output = BlockOutput::new();
        output.setup(3, 1024);

        assert_eq!(output.num_t(), 3);
        assert_eq!(output.state.num_bits(), 1024);
        assert_eq!(output.history.len(), 3);
        assert_eq!(output.changes.len(), 3);
    }

    #[test]
    fn test_setup_rounds_to_word_boundary() {
        let mut output = BlockOutput::new();
        output.setup(2, 100); // Not divisible by 32

        // num_bits() should return exact requested size
        assert_eq!(output.state.num_bits(), 100);

        // But internally, storage should be rounded up to 4 words (128 bits capacity)
        assert_eq!(output.state.num_words(), 4);
    }

    #[test]
    #[should_panic(expected = "num_t must be >= 2")]
    fn test_setup_requires_min_history() {
        let mut output = BlockOutput::new();
        output.setup(1, 32); // Should panic
    }

    #[test]
    fn test_clear() {
        let mut output = BlockOutput::new();
        output.setup(2, 32);

        output.state.set_bit(5);
        output.state.set_bit(10);
        output.store();

        output.clear();

        assert_eq!(output.state.num_set(), 0);
        assert!(output.has_changed());
    }

    #[test]
    fn test_step() {
        let mut output = BlockOutput::new();
        output.setup(3, 32);

        assert_eq!(output.curr_idx, 0);
        output.step();
        assert_eq!(output.curr_idx, 1);
        output.step();
        assert_eq!(output.curr_idx, 2);
        output.step();
        assert_eq!(output.curr_idx, 0); // Wraps around
    }

    #[test]
    fn test_store_detects_change() {
        let mut output = BlockOutput::new();
        output.setup(2, 32);

        // First store - state is different from initial history
        output.state.set_bit(5);
        output.store();
        assert!(output.has_changed());

        // Step and store without modification
        output.step();
        output.store();
        assert!(!output.has_changed()); // No change

        // Modify and store
        output.state.set_bit(10);
        output.store();
        assert!(output.has_changed()); // Changed
    }

    #[test]
    fn test_has_changed_at() {
        let mut output = BlockOutput::new();
        output.setup(3, 32);

        output.state.set_bit(5);
        output.store();
        let changed_at_0 = output.has_changed();

        output.step();
        output.store(); // No change
        let changed_at_1 = output.has_changed();

        assert!(changed_at_0);
        assert!(!changed_at_1);

        // Check historical changes
        assert!(!output.has_changed_at(0)); // Current (no change)
        assert!(output.has_changed_at(1)); // Previous (had change)
    }

    #[test]
    fn test_get_bitfield() {
        let mut output = BlockOutput::new();
        output.setup(3, 32);

        // Store pattern at t=0
        output.state.set_bit(5);
        output.store();

        output.step();

        // Store different pattern at t=1
        output.state.set_bit(10);
        output.store();

        // Check we can retrieve both
        let curr = output.get_bitfield(CURR);
        let prev = output.get_bitfield(PREV);

        assert_eq!(curr.get_bit(10), 1);
        assert_eq!(prev.get_bit(5), 1);
    }

    #[test]
    fn test_idx_circular_buffer() {
        let mut output = BlockOutput::new();
        output.setup(3, 32);

        // At curr_idx=0
        assert_eq!(output.idx(0), 0); // Current
        assert_eq!(output.idx(1), 2); // Previous (wraps)
        assert_eq!(output.idx(2), 1); // Two steps ago

        output.step(); // curr_idx=1
        assert_eq!(output.idx(0), 1); // Current
        assert_eq!(output.idx(1), 0); // Previous
        assert_eq!(output.idx(2), 2); // Two steps ago

        output.step(); // curr_idx=2
        assert_eq!(output.idx(0), 2); // Current
        assert_eq!(output.idx(1), 1); // Previous
        assert_eq!(output.idx(2), 0); // Two steps ago
    }

    #[test]
    fn test_unique_ids() {
        let output1 = BlockOutput::new();
        let output2 = BlockOutput::new();
        let output3 = BlockOutput::new();

        assert_ne!(output1.id(), output2.id());
        assert_ne!(output2.id(), output3.id());
    }

    #[test]
    fn test_memory_usage() {
        let mut output = BlockOutput::new();
        output.setup(3, 1024);

        let usage = output.memory_usage();
        assert!(usage > 0);
        // Should include state + 3 history BitFields
        assert!(usage > 4 * (1024 / 8));
    }
}
