//! ScalarTransformer - Encodes continuous scalar values into overlapping binary patterns.
//!
//! This module provides the `ScalarTransformer` block that converts continuous
//! scalar values into Sparse Distributed Representations (SDRs) where similar
//! values have overlapping active bits, preserving semantic similarity.
//!
//! # Semantic Properties
//!
//! - **Overlapping Representations**: Similar values have similar bit patterns
//! - **Continuous Gradation**: Smooth transitions between values
//! - **Semantic Similarity**: Overlap percentage correlates with value similarity
//!
//! # Examples
//!
//! ```
//! use gnomics::blocks::ScalarTransformer;
//! use gnomics::{Block, OutputAccess};
//!
//! // Create transformer for range [0.0, 1.0]
//! let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
//!
//! // Encode value 0.5
//! st.set_value(0.5);
//! st.execute(false).unwrap();
//!
//! // Output has exactly 128 active bits
//! assert_eq!(st.output().borrow().state.num_set(), 128);
//!
//! // Test semantic similarity
//! let mut st2 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
//! st2.set_value(0.51);  // Similar value
//! st2.execute(false).unwrap();
//!
//! // Similar values have high overlap
//! let overlap = st.output().borrow().state.num_similar(&st2.output().borrow().state);
//! assert!(overlap > 100);  // Significant overlap
//! ```

use crate::{Block, BlockBase, BlockBaseAccess, BlockOutput, OutputAccess, Result};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// Encodes continuous scalar values into overlapping binary patterns.
///
/// Creates Sparse Distributed Representations (SDRs) where similar values
/// have overlapping active bits. This preserves semantic similarity in the
/// encoded representation.
///
/// # Algorithm
///
/// 1. Normalize value to [0, 1] range
/// 2. Calculate center position: `center = normalized * (num_s - num_as)`
/// 3. Activate contiguous window of `num_as` bits starting at center
///
/// # Performance
///
/// - Encoding time: ~500ns for 1024 bits, 128 active (simple arithmetic + bit setting)
/// - Memory: ~1KB for 1024 bits with history depth 2
/// - No learning overhead (encoder only)
#[derive(Clone)]
#[allow(dead_code)]
pub struct ScalarTransformer {
    base: BlockBase,

    /// Block output with history
    output: Rc<RefCell<BlockOutput>>,

    // Parameters
    min_val: f64,
    max_val: f64,
    dif_val: f64,  // max_val - min_val
    num_s: usize,  // Number of statelets
    num_as: usize, // Number of active statelets
    dif_s: usize,  // num_s - num_as

    // State
    value: f64,
    value_prev: f64, // For change detection optimization
}

impl ScalarTransformer {
    /// Create a new ScalarTransformer.
    ///
    /// # Arguments
    ///
    /// * `min_val` - Minimum input value (inclusive)
    /// * `max_val` - Maximum input value (inclusive)
    /// * `num_s` - Number of statelets (output bits)
    /// * `num_as` - Number of active statelets (typically 10-20% of num_s)
    /// * `num_t` - History depth (must be >= 2)
    /// * `seed` - RNG seed for reproducibility (unused in transformer, for consistency)
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `max_val` <= `min_val`
    /// - `num_as` > `num_s`
    /// - `num_t` < 2
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::ScalarTransformer;
    ///
    /// // Temperature sensor: 0-100 degrees
    /// let temp_encoder = ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0);
    ///
    /// // Normalized values: 0-1
    /// let norm_encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    /// ```
    pub fn new(
        min_val: f64,
        max_val: f64,
        num_s: usize,
        num_as: usize,
        num_t: usize,
        seed: u64,
    ) -> Self {
        assert!(max_val > min_val, "max_val must be greater than min_val");
        assert!(num_as <= num_s, "num_as must be <= num_s");
        assert!(num_t >= 2, "num_t must be at least 2");

        let dif_val = max_val - min_val;
        let dif_s = num_s - num_as;

        let output = Rc::new(RefCell::new(BlockOutput::new()));
        output.borrow_mut().setup(num_t, num_s);

        let st = Self {
            base: BlockBase::new(seed),
            output,
            min_val,
            max_val,
            dif_val,
            num_s,
            num_as,
            dif_s,
            value: min_val,
            value_prev: 0.123456789, // Unlikely sentinel value (matches C++)
        };

        st
    }

    /// Set the current value to encode.
    ///
    /// Value is automatically clamped to [min_val, max_val] range.
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::ScalarTransformer;
    /// use gnomics::Block;
    ///
    /// let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    /// st.set_value(0.75);
    /// assert_eq!(st.get_value(), 0.75);
    ///
    /// // Values are clamped
    /// st.set_value(1.5);  // Out of range
    /// assert_eq!(st.get_value(), 1.0);  // Clamped to max
    /// ```
    pub fn set_value(&mut self, value: f64) {
        self.value = value.clamp(self.min_val, self.max_val);
    }

    /// Get the current value.
    ///
    /// Returns the last value set via `set_value()`, clamped to valid range.
    pub fn get_value(&self) -> f64 {
        self.value
    }

    /// Get minimum value.
    pub fn min_val(&self) -> f64 {
        self.min_val
    }

    /// Get maximum value.
    pub fn max_val(&self) -> f64 {
        self.max_val
    }

    /// Get number of statelets.
    pub fn num_s(&self) -> usize {
        self.num_s
    }

    /// Get number of active statelets.
    pub fn num_as(&self) -> usize {
        self.num_as
    }
}

impl Block for ScalarTransformer {
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
        self.output.borrow_mut().clear();
        self.value = self.min_val;
        self.value_prev = 0.123456789;
    }

    fn step(&mut self) {
        self.output.borrow_mut().step();
    }

    fn pull(&mut self) {
        // No inputs - transformer is a source block
    }

    fn compute(&mut self) {
        // Optimization: Only encode if value changed (matches C++ implementation)
        if self.value != self.value_prev {
            // Clamp value to valid range
            let clamped = self.value.clamp(self.min_val, self.max_val);

            // Normalize to [0, 1]
            let percent = (clamped - self.min_val) / self.dif_val;

            // Calculate starting position in statelet space
            let beg = ((self.dif_s as f64) * percent) as usize;

            // Clear output and activate contiguous window
            let mut output = self.output.borrow_mut();
            output.state.clear_all();
            output.state.set_range(beg, self.num_as);

            self.value_prev = self.value;
        }
    }

    fn learn(&mut self) {
        // No learning in transformer
    }

    fn store(&mut self) {
        self.output.borrow_mut().store();
    }

    fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.output.borrow().memory_usage()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl OutputAccess for ScalarTransformer {
    fn output(&self) -> Rc<RefCell<BlockOutput>> {
        Rc::clone(&self.output)
    }
}

impl BlockBaseAccess for ScalarTransformer {
    fn base(&self) -> &BlockBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BlockBase {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
        assert_eq!(st.min_val(), 0.0);
        assert_eq!(st.max_val(), 1.0);
        assert_eq!(st.num_s(), 1024);
        assert_eq!(st.num_as(), 128);
    }

    #[test]
    #[should_panic(expected = "max_val must be greater than min_val")]
    fn test_invalid_range() {
        ScalarTransformer::new(1.0, 0.0, 1024, 128, 2, 0);
    }

    #[test]
    #[should_panic(expected = "num_as must be <= num_s")]
    fn test_invalid_active_count() {
        ScalarTransformer::new(0.0, 1.0, 1024, 2048, 2, 0);
    }

    #[test]
    fn test_set_get_value() {
        let mut st = ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 0);

        st.set_value(50.0);
        assert_eq!(st.get_value(), 50.0);

        st.set_value(75.5);
        assert_eq!(st.get_value(), 75.5);
    }

    #[test]
    fn test_value_clamping() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

        // Below minimum
        st.set_value(-0.5);
        assert_eq!(st.get_value(), 0.0);

        // Above maximum
        st.set_value(1.5);
        assert_eq!(st.get_value(), 1.0);
    }

    #[test]
    fn test_encode_num_active() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

        st.set_value(0.5);
        st.compute();

        // Should have exactly num_as active bits
        assert_eq!(st.output().borrow().state.num_set(), 128);
    }

    #[test]
    fn test_encode_range_boundaries() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

        // Minimum value
        st.set_value(0.0);
        st.compute();
        assert_eq!(st.output().borrow().state.num_set(), 128);
        let acts_min = st.output().borrow().state.get_acts();
        assert_eq!(acts_min[0], 0); // Should start at bit 0

        // Maximum value
        st.set_value(1.0);
        st.compute();
        assert_eq!(st.output().borrow().state.num_set(), 128);
        let acts_max = st.output().borrow().state.get_acts();
        assert_eq!(acts_max[acts_max.len() - 1], 1023); // Should end at last bit
    }

    #[test]
    fn test_encode_change_detection() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

        st.set_value(0.5);
        st.compute();
        let acts1 = st.output().borrow().state.get_acts();

        // Encode again without changing value
        st.compute();
        let acts2 = st.output().borrow().state.get_acts();

        // Should be identical (optimization check)
        assert_eq!(acts1, acts2);
    }

    #[test]
    fn test_feedforward() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

        st.set_value(0.5);
        st.execute(false).unwrap();

        assert_eq!(st.output().borrow().state.num_set(), 128);
    }

    #[test]
    fn test_clear() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

        st.set_value(0.5);
        st.execute(false).unwrap();

        st.clear();

        assert_eq!(st.output().borrow().state.num_set(), 0);
        assert_eq!(st.get_value(), st.min_val());
    }

    #[test]
    fn test_memory_usage() {
        let st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
        let usage = st.memory_usage();
        assert!(usage > 0);
    }
}
