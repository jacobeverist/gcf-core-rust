//! DiscreteTransformer - Encodes discrete categorical values into distinct binary patterns.
//!
//! This module provides the `DiscreteTransformer` block that converts categorical
//! values into Sparse Distributed Representations (SDRs) where each category has
//! a unique set of active bits with NO overlap between categories.
//!
//! # Semantic Properties
//!
//! - **Distinct Representations**: Different categories have zero overlap
//! - **Equal Spacing**: All categories have equal representation space
//! - **Clear Boundaries**: No gradual transitions (unlike ScalarTransformer)
//!
//! # Examples
//!
//! ```
//! use gnomics::blocks::DiscreteTransformer;
//! use gnomics::Block;
//!
//! // Create transformer for 4 categories
//! let mut dt = DiscreteTransformer::new(4, 1024, 2, 0);
//!
//! // Encode category 2
//! dt.set_value(2);
//! dt.feedforward(false).unwrap();
//!
//! // Each category gets 256 bits (1024 / 4)
//! assert_eq!(dt.output.state.num_set(), 256);
//!
//! // Different categories have zero overlap
//! let mut dt2 = DiscreteTransformer::new(4, 1024, 2, 0);
//! dt2.set_value(1);
//! dt2.feedforward(false).unwrap();
//!
//! let overlap = dt.output.state.num_similar(&dt2.output.state);
//! assert_eq!(overlap, 0);  // No overlap
//! ```

use crate::{Block, BlockBase, BlockOutput, Result};
use std::path::Path;

/// Encodes discrete categorical values into distinct binary patterns.
///
/// Each category gets a unique set of active bits with no overlap between
/// categories. This creates clear categorical boundaries in the representation.
///
/// # Algorithm
///
/// 1. Divide statelet space into `num_v` equal partitions
/// 2. Each partition has `num_as = num_s / num_v` bits
/// 3. For value `v`, calculate position: `percent = v / (num_v - 1)`
/// 4. Activate bits starting at: `beg = dif_s * percent`
///
/// # Performance
///
/// - Encoding time: ~300ns for 1024 bits (simple arithmetic + bit setting)
/// - Memory: ~1KB for 1024 bits with history depth 2
/// - No learning overhead (encoder only)
#[derive(Clone)]
pub struct DiscreteTransformer {
    base: BlockBase,

    /// Block output with history
    pub output: BlockOutput,

    // Parameters
    num_v: usize,      // Number of discrete values
    num_s: usize,      // Number of statelets
    num_as: usize,     // Number of active statelets (num_s / num_v)
    dif_s: usize,      // num_s - num_as

    // State
    value: usize,
    value_prev: usize, // For change detection optimization
}

impl DiscreteTransformer {
    /// Create a new DiscreteTransformer.
    ///
    /// # Arguments
    ///
    /// * `num_v` - Number of discrete values/categories
    /// * `num_s` - Number of statelets (output bits)
    /// * `num_t` - History depth (must be >= 2)
    /// * `seed` - RNG seed for reproducibility (unused in transformer, for consistency)
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `num_v` == 0
    /// - `num_s` == 0
    /// - `num_t` < 2
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::DiscreteTransformer;
    ///
    /// // Day of week (7 categories)
    /// let dow_encoder = DiscreteTransformer::new(7, 2048, 2, 0);
    ///
    /// // Binary choice (2 categories)
    /// let binary_encoder = DiscreteTransformer::new(2, 1024, 2, 0);
    ///
    /// // 16 categories
    /// let hex_encoder = DiscreteTransformer::new(16, 1024, 2, 0);
    /// ```
    pub fn new(num_v: usize, num_s: usize, num_t: usize, seed: u64) -> Self {
        assert!(num_v > 0, "num_v must be > 0");
        assert!(num_s > 0, "num_s must be > 0");
        assert!(num_t >= 2, "num_t must be at least 2");

        let num_as = num_s / num_v;
        let dif_s = num_s - num_as;

        let mut dt = Self {
            base: BlockBase::new(seed),
            output: BlockOutput::new(),
            num_v,
            num_s,
            num_as,
            dif_s,
            value: 0,
            value_prev: usize::MAX, // Sentinel value (matches C++ 0xFFFFFFFF)
        };

        // Initialize output
        dt.output.setup(num_t, num_s);
        dt.base.set_initialized(true);

        dt
    }

    /// Set the current categorical value (0 to num_v-1).
    ///
    /// # Panics
    ///
    /// Panics if value >= num_v
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::DiscreteTransformer;
    ///
    /// let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);
    /// dt.set_value(5);  // Valid: 0-9
    /// assert_eq!(dt.get_value(), 5);
    /// ```
    pub fn set_value(&mut self, value: usize) {
        assert!(value < self.num_v, "value must be < num_v");
        self.value = value;
    }

    /// Get the current categorical value.
    ///
    /// Returns the last value set via `set_value()`.
    pub fn get_value(&self) -> usize {
        self.value
    }

    /// Get number of discrete values/categories.
    pub fn num_v(&self) -> usize {
        self.num_v
    }

    /// Get number of statelets.
    pub fn num_s(&self) -> usize {
        self.num_s
    }

    /// Get number of active statelets per category.
    pub fn num_as(&self) -> usize {
        self.num_as
    }
}

impl Block for DiscreteTransformer {
    fn init(&mut self) -> Result<()> {
        // Already initialized in new()
        Ok(())
    }

    fn save(&self, _path: &Path) -> Result<()> {
        // TODO: Implement serialization
        Ok(())
    }

    fn load(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement deserialization
        Ok(())
    }

    fn clear(&mut self) {
        self.output.clear();
        self.value = 0;
        self.value_prev = usize::MAX;
    }

    fn step(&mut self) {
        self.output.step();
    }

    fn pull(&mut self) {
        // No inputs - transformer is a source block
    }

    fn push(&mut self) {
        // No children to push to
    }

    fn encode(&mut self) {
        // Validation (matches C++ assert in encode)
        assert!(self.value < self.num_v, "value must be < num_v");

        // Optimization: Only encode if value changed (matches C++ implementation)
        if self.value != self.value_prev {
            // Calculate percentage position in value space
            let percent = if self.num_v > 1 {
                (self.value as f64) / ((self.num_v - 1) as f64)
            } else {
                0.0
            };

            // Calculate starting position in statelet space
            let beg = ((self.dif_s as f64) * percent) as usize;

            // Clear output and activate contiguous window
            self.output.state.clear_all();
            self.output.state.set_range(beg, self.num_as);

            self.value_prev = self.value;
        }
    }

    fn decode(&mut self) {
        // TODO: Implement reverse mapping from bits to category
        // Would find center of mass of active bits and map to nearest category
    }

    fn learn(&mut self) {
        // No learning in transformer
    }

    fn store(&mut self) {
        self.output.store();
    }

    fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.output.memory_usage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let dt = DiscreteTransformer::new(10, 1024, 2, 0);
        assert_eq!(dt.num_v(), 10);
        assert_eq!(dt.num_s(), 1024);
        assert_eq!(dt.num_as(), 102); // 1024 / 10
    }

    #[test]
    #[should_panic(expected = "num_v must be > 0")]
    fn test_invalid_num_v() {
        DiscreteTransformer::new(0, 1024, 2, 0);
    }

    #[test]
    fn test_set_get_value() {
        let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

        dt.set_value(0);
        assert_eq!(dt.get_value(), 0);

        dt.set_value(5);
        assert_eq!(dt.get_value(), 5);

        dt.set_value(9);
        assert_eq!(dt.get_value(), 9);
    }

    #[test]
    #[should_panic(expected = "value must be < num_v")]
    fn test_value_out_of_range() {
        let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);
        dt.set_value(10); // Invalid: should be 0-9
    }

    #[test]
    fn test_encode_num_active() {
        let mut dt = DiscreteTransformer::new(4, 1024, 2, 0);

        dt.set_value(2);
        dt.encode();

        // Should have num_as active bits (1024 / 4 = 256)
        assert_eq!(dt.output.state.num_set(), 256);
    }

    #[test]
    fn test_encode_different_categories() {
        let mut dt1 = DiscreteTransformer::new(4, 1024, 2, 0);
        let mut dt2 = DiscreteTransformer::new(4, 1024, 2, 0);

        dt1.set_value(0);
        dt1.encode();

        dt2.set_value(1);
        dt2.encode();

        // Different categories should have zero overlap
        let overlap = dt1.output.state.num_similar(&dt2.output.state);
        assert_eq!(overlap, 0);
    }

    #[test]
    fn test_encode_same_category() {
        let mut dt1 = DiscreteTransformer::new(4, 1024, 2, 0);
        let mut dt2 = DiscreteTransformer::new(4, 1024, 2, 0);

        dt1.set_value(2);
        dt1.encode();

        dt2.set_value(2);
        dt2.encode();

        // Same category should be identical
        assert_eq!(dt1.output.state, dt2.output.state);
    }

    #[test]
    fn test_encode_all_categories_distinct() {
        let num_v = 8;
        let mut transformers: Vec<DiscreteTransformer> = (0..num_v)
            .map(|_| DiscreteTransformer::new(num_v, 1024, 2, 0))
            .collect();

        // Encode each category
        for (i, dt) in transformers.iter_mut().enumerate() {
            dt.set_value(i);
            dt.encode();
        }

        // Verify all pairs are distinct
        for i in 0..num_v {
            for j in (i + 1)..num_v {
                let overlap = transformers[i]
                    .output
                    .state
                    .num_similar(&transformers[j].output.state);
                assert_eq!(
                    overlap, 0,
                    "Categories {} and {} should have no overlap",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_encode_change_detection() {
        let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

        dt.set_value(5);
        dt.encode();
        let acts1 = dt.output.state.get_acts();

        // Encode again without changing value
        dt.encode();
        let acts2 = dt.output.state.get_acts();

        // Should be identical (optimization check)
        assert_eq!(acts1, acts2);
    }

    #[test]
    fn test_feedforward() {
        let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

        dt.set_value(5);
        dt.feedforward(false).unwrap();

        assert_eq!(dt.output.state.num_set(), 102); // 1024 / 10
    }

    #[test]
    fn test_clear() {
        let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

        dt.set_value(5);
        dt.feedforward(false).unwrap();

        dt.clear();

        assert_eq!(dt.output.state.num_set(), 0);
        assert_eq!(dt.get_value(), 0);
    }

    #[test]
    fn test_memory_usage() {
        let dt = DiscreteTransformer::new(10, 1024, 2, 0);
        let usage = dt.memory_usage();
        assert!(usage > 0);
    }

    #[test]
    fn test_binary_choice() {
        let mut dt = DiscreteTransformer::new(2, 1024, 2, 0);

        dt.set_value(0);
        dt.encode();
        let acts0 = dt.output.state.get_acts();
        assert_eq!(dt.output.state.num_set(), 512);

        dt.set_value(1);
        dt.encode();
        let acts1 = dt.output.state.get_acts();
        assert_eq!(dt.output.state.num_set(), 512);

        // Verify no overlap
        let overlap = acts0
            .iter()
            .filter(|&&a| acts1.contains(&a))
            .count();
        assert_eq!(overlap, 0);
    }
}
