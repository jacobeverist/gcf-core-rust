//! SequenceLearner - Learns temporal sequences and predicts next patterns.
//!
//! This module provides the `SequenceLearner` block that learns to predict the next
//! pattern in a sequence. It is nearly identical to ContextLearner but uses its own
//! previous output as context, enabling it to learn temporal transitions.
//!
//! # Algorithm
//!
//! Same as ContextLearner, but:
//! - Context = output[PREV] (previous time step)
//! - Self-feedback loop for temporal learning
//!
//! For each active column:
//! 1. **Recognition**: Check if any dendrite predicts based on previous output
//! 2. **Surprise**: If unpredicted, activate statelets and learn
//! 3. **Learning**: Dendrites learn the transition from previous → current
//!
//! # Architecture
//!
//! ```text
//! output           memory (showing statelet 15 dendrites)
//! -----------      +----------------------------+
//! 0 0 0 0 0[0] --> | addr[0]: {00 00 00 00 ...} |
//! 0 0 0 0 0 0      | perm[0]: {00 00 00 00 ...} |
//! 0 0 0 0 0 0      | addr[1]: {00 00 00 00 ...} |
//!                  | perm[1]: {00 00 00 00 ...} |
//! context          | addr[2]: {00 00 00 00 ...} |
//! (prev output)    | perm[2]: {00 00 00 00 ...} |
//! -----------      |  ...                       |
//! 0 0 0 0 0 0      +----------------------------+
//! 0 0 0 0 0 0          ^
//! 0 0 0 0 0 0          | (self-feedback loop)
//!      ----------------+
//! input
//! (column activations)
//! -----------
//! 0 0 0 0 0 0
//! ```
//!
//! # Use Cases
//!
//! - Time series prediction
//! - Sequence learning (motor patterns, language)
//! - Anomaly detection in temporal data
//! - Learning "what follows what" in sequences
//!
//! # Examples
//!
//! ```
//! use gnomics::blocks::{DiscreteTransformer, SequenceLearner};
//! use gnomics::Block;
//! use std::rc::Rc;
//! use std::cell::RefCell;
//!
//! // Create input encoder
//! let mut encoder = DiscreteTransformer::new(10, 512, 2, 0);
//!
//! // Create sequence learner (context is auto-connected to output[PREV])
//! let mut learner = SequenceLearner::new(
//!     512,   // num_c: 512 columns
//!     4,     // num_spc: 4 statelets per column
//!     8,     // num_dps: 8 dendrites per statelet
//!     32,    // num_rpd: 32 receptors per dendrite
//!     20,    // d_thresh: dendrite activation threshold
//!     20,    // perm_thr: permanence threshold
//!     2,     // perm_inc: permanence increment
//!     1,     // perm_dec: permanence decrement
//!     2,     // num_t: history depth
//!     false, // always_update
//!     0,     // seed
//! );
//!
//! // Connect input
//! learner.input.add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
//! learner.init().unwrap();
//!
//! // Learn sequence: 0 → 1 → 2 → 0 → 1 → 2 ...
//! for &value in &[0, 1, 2, 0, 1, 2] {
//!     encoder.set_value(value);
//!     encoder.execute(false).unwrap();
//!     learner.execute(true).unwrap();  // Learn transitions
//!
//!     let anomaly = learner.get_anomaly_score();
//!     println!("Value: {}, Anomaly: {:.2}%", value, anomaly * 100.0);
//! }
//! ```

use crate::{Block, BlockBase, BlockInput, BlockMemory, BlockOutput, Result};
use crate::bitarray::BitArray;
use crate::utils;
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;

/// Learns temporal sequences and predicts next patterns.
///
/// SequenceLearner is nearly identical to ContextLearner, but uses its own
/// previous output as context. This creates a self-feedback loop enabling
/// temporal sequence learning.
///
/// # Performance
///
/// - Encoding time: ~50-100µs for 512 columns × 4 statelets (dendrite overlap checks)
/// - Learning time: ~20-50µs per active statelet (dendrite assignment + learning)
/// - Memory: ~500KB for 2048 statelets × 8 dendrites × 32 receptors
pub struct SequenceLearner {
    base: BlockBase,

    /// Block input for column activations
    pub input: BlockInput,

    /// Block input for contextual pattern (connected to output[PREV])
    pub context: BlockInput,

    /// Block output with history (also feeds back to context)
    pub output: Rc<RefCell<BlockOutput>>,

    /// Block memory with synaptic learning
    pub memory: BlockMemory,

    // Architecture parameters
    num_c: usize,      // Number of columns
    num_spc: usize,    // Statelets per column
    num_dps: usize,    // Dendrites per statelet
    num_dpc: usize,    // Dendrites per column (num_spc × num_dps)
    num_rpd: usize,    // Receptors per dendrite
    num_s: usize,      // Total statelets (num_c × num_spc)
    num_d: usize,      // Total dendrites (num_s × num_dps)
    d_thresh: u32,     // Dendrite activation threshold
    num_t: usize,      // History depth

    // Learning parameters
    perm_thr: u8,      // Permanence threshold
    perm_inc: u8,      // Permanence increment
    perm_dec: u8,      // Permanence decrement

    // State
    next_sd: Vec<usize>,  // Next available dendrite per statelet
    d_used: BitArray,     // Dendrite usage mask (1=used, 0=available)
    anomaly_score: f64,   // Current anomaly score (0.0-1.0)
    always_update: bool,  // Update even if inputs unchanged

    // Working memory
    input_acts: Vec<usize>,  // Active column indices
    d_acts: Vec<usize>,      // Active dendrite indices
    surprise_flag: bool,     // Surprise detected for current column
}

impl SequenceLearner {
    /// Create a new SequenceLearner with self-feedback loop.
    ///
    /// # Arguments
    ///
    /// * `num_c` - Number of columns
    /// * `num_spc` - Statelets per column (alternative representations)
    /// * `num_dps` - Dendrites per statelet (multiple pattern detectors)
    /// * `num_rpd` - Receptors per dendrite (connections to context)
    /// * `d_thresh` - Dendrite activation threshold (receptors needed to fire)
    /// * `perm_thr` - Permanence threshold (0-99, typically 20)
    /// * `perm_inc` - Permanence increment (0-99, typically 2)
    /// * `perm_dec` - Permanence decrement (0-99, typically 1)
    /// * `num_t` - History depth (must be >= 2 for self-feedback)
    /// * `always_update` - Update even when inputs unchanged
    /// * `seed` - RNG seed for reproducibility
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `num_c` == 0
    /// - `num_spc` == 0
    /// - `num_dps` == 0
    /// - `num_rpd` == 0
    /// - `d_thresh` >= `num_rpd`
    /// - `num_t` < 2
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::SequenceLearner;
    ///
    /// // Standard configuration
    /// let learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
    ///
    /// // High capacity configuration
    /// let big_learner = SequenceLearner::new(1024, 8, 16, 64, 40, 20, 2, 1, 2, false, 0);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        num_c: usize,
        num_spc: usize,
        num_dps: usize,
        num_rpd: usize,
        d_thresh: u32,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        num_t: usize,
        always_update: bool,
        seed: u64,
    ) -> Self {
        assert!(num_c > 0, "num_c must be > 0");
        assert!(num_spc > 0, "num_spc must be > 0");
        assert!(num_dps > 0, "num_dps must be > 0");
        assert!(num_rpd > 0, "num_rpd must be > 0");
        assert!(d_thresh < num_rpd as u32, "d_thresh must be < num_rpd");
        assert!(num_t >= 2, "num_t must be at least 2");

        let num_s = num_c * num_spc;
        let num_d = num_s * num_dps;
        let num_dpc = num_spc * num_dps;

        let pct_learn = 1.0; // Learn on all receptors

        // Create output and self-feedback loop
        let output_rc = Rc::new(RefCell::new(BlockOutput::new()));

        // Setup output BEFORE adding as child (needed for time offset validation)
        output_rc.borrow_mut().setup(num_t, num_s);

        let mut context = BlockInput::new();

        // Self-feedback: context pulls from output[PREV] (time=1)
        context.add_child(Rc::clone(&output_rc), 1);

        Self {
            base: BlockBase::new(seed),
            input: BlockInput::new(),
            context,
            output: output_rc,
            memory: BlockMemory::new(num_d, num_rpd, perm_thr, perm_inc, perm_dec, pct_learn),
            num_c,
            num_spc,
            num_dps,
            num_dpc,
            num_rpd,
            num_s,
            num_d,
            d_thresh,
            num_t,
            perm_thr,
            perm_inc,
            perm_dec,
            next_sd: vec![0; num_s],
            d_used: BitArray::new(num_d),
            anomaly_score: 0.0,
            always_update,
            input_acts: Vec::new(),
            d_acts: Vec::new(),
            surprise_flag: false,
        }
    }

    /// Get current anomaly score.
    ///
    /// Returns percentage of input columns that were unexpected (0.0-1.0).
    /// - 0.0 = All columns were predicted by previous pattern
    /// - 1.0 = All columns were unexpected (sequence broken)
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::SequenceLearner;
    /// use gnomics::Block;
    ///
    /// let mut learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
    /// // ... initialize and process sequence ...
    /// let anomaly = learner.get_anomaly_score();
    /// println!("Sequence anomaly: {:.2}%", anomaly * 100.0);
    /// ```
    pub fn get_anomaly_score(&self) -> f64 {
        self.anomaly_score
    }

    /// Get count of statelets that have at least one dendrite.
    ///
    /// This indicates how many different temporal transitions have been learned.
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::SequenceLearner;
    ///
    /// let learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
    /// let count = learner.get_historical_count();
    /// println!("Learned {} unique transitions", count);
    /// ```
    pub fn get_historical_count(&self) -> usize {
        self.next_sd.iter().filter(|&&n| n > 0).count()
    }

    /// Get number of columns.
    pub fn num_c(&self) -> usize {
        self.num_c
    }

    /// Get statelets per column.
    pub fn num_spc(&self) -> usize {
        self.num_spc
    }

    /// Get dendrites per statelet.
    pub fn num_dps(&self) -> usize {
        self.num_dps
    }

    /// Get dendrite activation threshold.
    pub fn d_thresh(&self) -> u32 {
        self.d_thresh
    }

    /// Recognition phase: check if any dendrite predicts the column.
    ///
    /// For the given column, checks all its dendrites against the previous output.
    /// If any dendrite overlap exceeds threshold, activates the statelet and
    /// clears the surprise flag.
    fn recognition(&mut self, c: usize) {
        let d_beg = c * self.num_dpc;
        let d_end = d_beg + self.num_dpc;

        // For every dendrite on the column
        for d in d_beg..d_end {
            // If dendrite is used, compute overlap
            if self.d_used.get_bit(d) > 0 {
                let overlap = self.memory.overlap(d, &self.context.state);

                // If dendrite overlap exceeds threshold
                if overlap >= self.d_thresh as usize {
                    let s = d / self.num_dps;
                    self.d_acts.push(d);
                    self.output.borrow_mut().state.set_bit(s);
                    self.surprise_flag = false;
                }
            }
        }
    }

    /// Surprise phase: handle unexpected column activation.
    ///
    /// When no dendrite predicted the column based on previous output,
    /// activate statelets and assign dendrites to learn this new transition.
    fn surprise(&mut self, c: usize) {
        // Update anomaly score
        let num_input_acts = self.input_acts.len();
        if num_input_acts > 0 {
            self.anomaly_score += 1.0 / num_input_acts as f64;
        }

        // Get statelet range for this column
        let s_beg = c * self.num_spc;
        let s_end = s_beg + self.num_spc;

        // Choose random statelet in column
        let s_rand = if self.num_spc > 1 {
            utils::rand_uint(s_beg as u32, (s_end - 1) as u32, self.base.rng()) as usize
        } else {
            s_beg
        };

        // Activate random statelet
        self.output.borrow_mut().state.set_bit(s_rand);

        // Assign next available dendrite to random statelet
        self.set_next_available_dendrite(s_rand);

        // Activate historical statelets (those with at least one dendrite)
        for s in s_beg..s_end {
            if s != s_rand && self.next_sd[s] > 0 {
                self.output.borrow_mut().state.set_bit(s);
                self.set_next_available_dendrite(s);
            }
        }
    }

    /// Assign next available dendrite for a statelet.
    ///
    /// Marks the dendrite as active and increments the next available counter.
    fn set_next_available_dendrite(&mut self, s: usize) {
        let d_beg = s * self.num_dps;
        let d_next = d_beg + self.next_sd[s];

        self.d_acts.push(d_next);

        // Update next available dendrite (saturate at num_dps-1)
        if self.next_sd[s] < self.num_dps - 1 {
            self.next_sd[s] += 1;
        }
    }
}

impl Block for SequenceLearner {
    fn init(&mut self) -> Result<()> {
        // Verify input size matches num_c
        assert_eq!(
            self.input.num_bits(),
            self.num_c,
            "input size must equal num_c"
        );

        // Note: Output setup now happens in new() before self-feedback connection

        // Initialize memory (dendrites learn from previous output)
        let num_context_bits = self.context.num_bits();
        self.memory.init(num_context_bits, self.base.rng());

        self.base.set_initialized(true);
        Ok(())
    }

    fn save(&self, _path: &Path) -> Result<()> {
        // TODO: Implement save
        Ok(())
    }

    fn load(&mut self, _path: &Path) -> Result<()> {
        // TODO: Implement load
        Ok(())
    }

    fn clear(&mut self) {
        self.input.clear();
        self.context.clear();
        self.output.borrow_mut().clear();
        self.memory.clear();
        self.anomaly_score = 0.0;
        self.input_acts.clear();
        self.d_acts.clear();
    }

    fn step(&mut self) {
        self.output.borrow_mut().step();
    }

    fn pull(&mut self) {
        self.input.pull();
        self.context.pull();
    }

    fn compute(&mut self) {
        assert!(self.base.is_initialized(), "must call init() first");

        // Check if any input changed
        if self.always_update || self.input.children_changed() || self.context.children_changed() {
            // Get active columns
            self.input_acts = self.input.state.get_acts();

            // Clear state
            self.anomaly_score = 0.0;
            self.output.borrow_mut().state.clear_all();
            self.d_acts.clear();

            // Process each active column
            let input_acts = self.input_acts.clone();
            for c in input_acts {
                self.surprise_flag = true;

                // Try recognition
                self.recognition(c);

                // Handle surprise if no dendrite predicted
                if self.surprise_flag {
                    self.surprise(c);
                }
            }
        }
    }

    fn learn(&mut self) {
        assert!(self.base.is_initialized(), "must call init() first");

        // Check if any input changed
        if self.always_update || self.input.children_changed() || self.context.children_changed() {
            // Learn on all active dendrites
            let d_acts = self.d_acts.clone();
            for d in d_acts {
                self.memory.learn_move(d, &self.context.state, self.base.rng());
                self.d_used.set_bit(d);
            }
        }
    }

    fn store(&mut self) {
        self.output.borrow_mut().store();
    }

    fn memory_usage(&self) -> usize {
        let mut bytes = std::mem::size_of::<Self>();
        bytes += self.input.memory_usage();
        bytes += self.context.memory_usage();
        bytes += self.output.borrow().memory_usage();
        bytes += self.memory.memory_usage();
        bytes += self.next_sd.capacity() * std::mem::size_of::<usize>();
        bytes += self.d_used.memory_usage();
        bytes += self.input_acts.capacity() * std::mem::size_of::<usize>();
        bytes += self.d_acts.capacity() * std::mem::size_of::<usize>();
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
        assert_eq!(learner.num_c(), 512);
        assert_eq!(learner.num_spc(), 4);
        assert_eq!(learner.num_dps(), 8);
        assert_eq!(learner.num_s, 512 * 4);
        assert_eq!(learner.num_d, 512 * 4 * 8);
    }

    #[test]
    fn test_self_feedback_connection() {
        let learner = SequenceLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
        // Context should have one child (output)
        assert_eq!(learner.context.num_children(), 1);
    }

    #[test]
    fn test_get_anomaly_score() {
        let learner = SequenceLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
        assert_eq!(learner.get_anomaly_score(), 0.0);
    }

    #[test]
    fn test_get_historical_count_empty() {
        let learner = SequenceLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
        assert_eq!(learner.get_historical_count(), 0);
    }

    #[test]
    fn test_memory_usage() {
        let learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
        let usage = learner.memory_usage();
        assert!(usage > 0);
    }

    #[test]
    #[should_panic(expected = "num_c must be > 0")]
    fn test_new_zero_columns() {
        SequenceLearner::new(0, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
    }

    #[test]
    #[should_panic(expected = "d_thresh must be < num_rpd")]
    fn test_new_thresh_too_high() {
        SequenceLearner::new(10, 4, 8, 32, 32, 20, 2, 1, 2, false, 0);
    }
}
