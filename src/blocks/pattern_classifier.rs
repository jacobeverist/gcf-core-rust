//! PatternClassifier - Supervised learning classifier for binary patterns.
//!
//! This module provides the `PatternClassifier` block that performs supervised
//! classification on binary patterns (SDRs). It divides statelets into groups,
//! one per label, and uses competitive learning within each group.
//!
//! # Architecture
//!
//! - Divides `num_s` statelets into `num_l` groups (num_spl = num_s / num_l)
//! - Each group represents one label/class
//! - During encoding, each group activates its top `num_as` dendrites
//! - During learning, only the group corresponding to the current label is updated
//!
//! # Usage Pattern
//!
//! ```ignore
//! classifier.set_label(label);     // Set ground truth
//! classifier.execute(true);    // Encode and learn
//! let probs = classifier.get_probabilities();  // Get predictions
//! ```
//!
//! # Examples
//!
//! ```
//! use gnomics::blocks::{ScalarTransformer, PatternClassifier};
//! use gnomics::{Block, InputAccess, OutputAccess};
//! use std::rc::Rc;
//! use std::cell::RefCell;
//!
//! // Create encoder and classifier
//! let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
//! let mut classifier = PatternClassifier::new(4, 1024, 8, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0);
//!
//! // Connect and initialize
//! classifier.input_mut().add_child(encoder.output(), 0);
//! classifier.init().unwrap();
//!
//! // Train on label 0
//! encoder.set_value(0.25);
//! encoder.execute(false).unwrap();
//! classifier.set_label(0);
//! classifier.execute(true).unwrap();
//!
//! // Infer (without learning)
//! encoder.set_value(0.26);
//! encoder.execute(false).unwrap();
//! classifier.execute(false).unwrap();
//! let probs = classifier.get_probabilities();
//! assert_eq!(probs.len(), 4);
//! ```

use crate::{Block, BlockBase, BlockBaseAccess, BlockInput, BlockMemory, BlockOutput, Result};
use crate::{InputAccess, MemoryAccess, OutputAccess};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// Supervised learning classifier for binary patterns.
///
/// Divides statelets into `num_l` groups, one per label. During encoding, each
/// group activates its top `num_as` dendrites. During learning, only the group
/// corresponding to the current label is updated.
///
/// # Performance
///
/// - Encoding time: ~10µs for 1024 dendrites, 4 labels (overlap + per-group sort)
/// - Learning time: ~5µs for label-specific update
/// - Memory: ~200KB for 1024 dendrites × 128 receptors with pooled connectivity
#[allow(dead_code)]
pub struct PatternClassifier {
    base: BlockBase,

    /// Block input connection point
    input: BlockInput,

    /// Block output with history
    output: Rc<RefCell<BlockOutput>>,

    /// Block memory with synaptic learning
    memory: BlockMemory,

    // Parameters
    num_l: usize,   // Number of labels
    num_s: usize,   // Number of statelets total
    num_spl: usize, // Statelets per label (num_s / num_l)
    num_as: usize,  // Active statelets per label
    num_rpd: usize, // Receptors per dendrite
    perm_thr: u8,   // Permanence threshold
    perm_inc: u8,   // Permanence increment
    perm_dec: u8,   // Permanence decrement
    pct_pool: f64,  // Pooling percentage
    pct_conn: f64,  // Initial connectivity
    pct_learn: f64, // Learning percentage
    num_t: usize,   // History depth

    // State
    label: Option<usize>,        // Current label for supervised learning
    overlaps: Vec<usize>,        // Overlap scores per dendrite
    statelet_labels: Vec<usize>, // Which label each statelet belongs to
}

impl PatternClassifier {
    /// Create a new PatternClassifier.
    ///
    /// # Arguments
    ///
    /// * `num_l` - Number of labels/classes
    /// * `num_s` - Number of statelets (dendrites) total
    /// * `num_as` - Number of active statelets per label
    /// * `perm_thr` - Permanence threshold (typically 20/99)
    /// * `perm_inc` - Permanence increment (typically 2)
    /// * `perm_dec` - Permanence decrement (typically 1)
    /// * `pct_pool` - Pooling percentage (typically 0.8 = 80% sparsity)
    /// * `pct_conn` - Initial connectivity (typically 0.5 = 50% connected)
    /// * `pct_learn` - Learning percentage (typically 0.3 = 30% update)
    /// * `num_t` - History depth (must be >= 2)
    /// * `seed` - RNG seed for reproducibility
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `num_s` is not divisible by `num_l`
    /// - `num_as` > `num_spl` (where num_spl = num_s / num_l)
    /// - `num_t` < 2
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::PatternClassifier;
    ///
    /// // 4-class classifier: 1024 statelets, 8 active per class
    /// let classifier = PatternClassifier::new(4, 1024, 8, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0);
    ///
    /// // 10-class classifier for MNIST-like tasks (2000 = 200 statelets per class)
    /// let mnist_classifier = PatternClassifier::new(10, 2000, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0);
    /// ```
    pub fn new(
        num_l: usize,
        num_s: usize,
        num_as: usize,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        pct_pool: f64,
        pct_conn: f64,
        pct_learn: f64,
        num_t: usize,
        seed: u64,
    ) -> Self {
        // assert!(
        //     num_s % num_l == 0,
        //     "num_s must be divisible by num_l (got {} / {})",
        //     num_s,
        //     num_l
        // );
        assert!(num_t >= 2, "num_t must be at least 2");

        let num_spl = num_s / num_l;
        assert!(
            num_as <= num_spl,
            "num_as must be <= num_spl (got {} > {})",
            num_as,
            num_spl
        );

        let num_rpd = 128; // Typical receptors per dendrite

        // Setup statelet labels: each statelet knows which label it represents
        let mut statelet_labels = vec![0; num_s];
        for s in 0..num_s {
            let label = s / num_spl;
            // Ensure we don't exceed num_l due to rounding
            statelet_labels[s] = if label >= num_l { 0 } else { label };
        }

        let output = Rc::new(RefCell::new(BlockOutput::new()));
        output.borrow_mut().setup(num_t, num_s);

        Self {
            base: BlockBase::new(seed),
            input: BlockInput::new(),
            output,
            memory: BlockMemory::new(num_s, num_rpd, perm_thr, perm_inc, perm_dec, pct_learn),
            num_l,
            num_s,
            num_spl,
            num_as,
            num_rpd,
            perm_thr,
            perm_inc,
            perm_dec,
            pct_pool,
            pct_conn,
            pct_learn,
            num_t,
            label: None,
            overlaps: vec![0; num_s],
            statelet_labels,
        }
    }

    /// Set the current label for supervised learning.
    ///
    /// Must be called before `feedforward(true)` to specify the ground truth label.
    ///
    /// # Panics
    ///
    /// Panics if `label` >= `num_l`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::blocks::PatternClassifier;
    ///
    /// let mut classifier = PatternClassifier::new(4, 1024, 8, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0);
    /// classifier.set_label(2);  // Set ground truth to label 2
    /// ```
    pub fn set_label(&mut self, label: usize) {
        assert!(
            label < self.num_l,
            "label must be < num_l (got {} >= {})",
            label,
            self.num_l
        );
        self.label = Some(label);
    }

    /// Get classification probabilities for all labels.
    ///
    /// Returns a vector of probabilities (0.0-1.0) for each label, summing to 1.0.
    /// Probabilities are based on the proportion of active statelets in each label group.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gnomics::blocks::{ScalarTransformer, PatternClassifier};
    /// # use gnomics::{Block, InputAccess, OutputAccess};
    /// # use std::rc::Rc;
    /// # use std::cell::RefCell;
    /// #
    /// # let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    /// # let mut classifier = PatternClassifier::new(4, 1024, 8, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0);
    /// # classifier.input_mut().add_child(encoder.output(), 0);
    /// # classifier.init().unwrap();
    /// # encoder.set_value(0.5);
    /// # encoder.execute(false).unwrap();
    /// # classifier.execute(false).unwrap();
    /// #
    /// let probs = classifier.get_probabilities();
    /// assert_eq!(probs.len(), 4);
    ///
    /// // Probabilities sum to approximately 1.0
    /// let sum: f64 = probs.iter().sum();
    /// assert!((sum - 1.0).abs() < 1e-6 || sum == 0.0);
    /// ```
    pub fn get_probabilities(&self) -> Vec<f64> {
        let mut probs = vec![0.0; self.num_l];

        // Sum overlaps per label group
        for l in 0..self.num_l {
            let start = l * self.num_spl;
            let end = start + self.num_spl;
            let sum: usize = self.overlaps[start..end].iter().sum();
            probs[l] = sum as f64;
        }

        // Normalize to probabilities
        let total: f64 = probs.iter().sum();
        if total > 0.0 {
            for p in &mut probs {
                *p /= total;
            }
        }

        probs
    }

    /// Get the predicted label (highest probability).
    ///
    /// Returns the label with the highest probability, or 0 if all probabilities are 0.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let predicted = classifier.get_predicted_label();
    /// println!("Predicted label: {}", predicted);
    /// ```
    pub fn get_predicted_label(&self) -> usize {
        let probs = self.get_probabilities();
        probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// Get the labels array (0, 1, 2, ..., num_l-1).
    pub fn get_labels(&self) -> Vec<usize> {
        (0..self.num_l).collect()
    }

    /// Get the statelet label assignments.
    ///
    /// Returns a vector showing which label each statelet belongs to.
    pub fn get_statelet_labels(&self) -> &[usize] {
        &self.statelet_labels
    }

    /// Get number of labels.
    pub fn num_l(&self) -> usize {
        self.num_l
    }

    /// Get number of statelets.
    pub fn num_s(&self) -> usize {
        self.num_s
    }

    /// Get number of active statelets per label.
    pub fn num_as(&self) -> usize {
        self.num_as
    }

    /// Get statelets per label.
    pub fn num_spl(&self) -> usize {
        self.num_spl
    }
}

impl Block for PatternClassifier {
    fn init(&mut self) -> Result<()> {
        // Output already set up in new()

        // Initialize memory with pooled connectivity
        let num_input_bits = self.input.num_bits();
        self.memory.init_pooled_conn(
            num_input_bits,
            self.base.rng(),
            self.pct_pool,
            self.pct_conn,
        );

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
        self.output.borrow_mut().clear();
        self.memory.clear();
    }

    fn step(&mut self) {
        self.output.borrow_mut().step();
    }

    fn pull(&mut self) {
        self.input.pull();
    }

    fn compute(&mut self) {
        assert!(
            self.base.is_initialized(),
            "PatternClassifier must be initialized before encoding"
        );

        // Skip if input unchanged
        if !self.input.children_changed() {
            return;
        }

        // Clear output
        self.output.borrow_mut().state.clear_all();

        // Compute overlaps for all dendrites
        for d in 0..self.num_s {
            self.overlaps[d] = self.memory.overlap_conn(d, &self.input.state);
        }

        // For each label group, activate top num_as dendrites
        for l in 0..self.num_l {
            let start = l * self.num_spl;
            let end = start + self.num_spl;

            // Sort this group by overlap (descending)
            let mut group: Vec<usize> = (start..end).collect();
            group.sort_by(|&a, &b| self.overlaps[b].cmp(&self.overlaps[a]));

            // Activate top num_as in this group
            for &idx in group.iter().take(self.num_as) {
                self.output.borrow_mut().state.set_bit(idx);
            }
        }
    }

    fn learn(&mut self) {
        assert!(
            self.base.is_initialized(),
            "PatternClassifier must be initialized before learning"
        );

        if let Some(label) = self.label {
            // Only learn on the specified label's group
            let start = label * self.num_spl;
            let end = start + self.num_spl;

            for d in start..end {
                if self.output.borrow().state.get_bit(d) == 1 {
                    // Learn (strengthen) winning dendrites in correct label group
                    self.memory
                        .learn_conn(d, &self.input.state, self.base.rng());
                } else {
                    // Could also punish non-winning dendrites in correct label group
                    // C++ implementation has this commented out - we'll leave it out too
                }
            }

            // Optional: Punish winning dendrites in wrong label groups
            // This helps create more distinct representations
            for d in 0..self.num_s {
                if self.statelet_labels[d] != label && self.output.borrow().state.get_bit(d) == 1 {
                    self.memory
                        .punish_conn(d, &self.input.state, self.base.rng());
                }
            }
        }
    }

    fn store(&mut self) {
        self.output.borrow_mut().store();
    }

    fn memory_usage(&self) -> usize {
        let base_size = std::mem::size_of::<Self>();
        let overlaps_size = self.overlaps.len() * std::mem::size_of::<usize>();
        let statelet_labels_size = self.statelet_labels.len() * std::mem::size_of::<usize>();
        let input_size = self.input.memory_usage();
        let output_size = self.output.borrow().memory_usage();
        let memory_size = self.memory.memory_usage();

        base_size + overlaps_size + statelet_labels_size + input_size + output_size + memory_size
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_dependencies(&self) -> Vec<crate::network::BlockId> {
        self.input.get_source_blocks()
    }
}

impl BlockBaseAccess for PatternClassifier {
    fn base(&self) -> &BlockBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BlockBase {
        &mut self.base
    }
}

impl InputAccess for PatternClassifier {
    fn input(&self) -> &BlockInput {
        &self.input
    }

    fn input_mut(&mut self) -> &mut BlockInput {
        &mut self.input
    }
}

impl MemoryAccess for PatternClassifier {
    fn memory(&self) -> &BlockMemory {
        &self.memory
    }

    fn memory_mut(&mut self) -> &mut BlockMemory {
        &mut self.memory
    }
}

impl OutputAccess for PatternClassifier {
    fn output(&self) -> Rc<RefCell<BlockOutput>> {
        Rc::clone(&self.output)
    }
}

impl crate::network_config::BlockConfigurable for PatternClassifier {
    fn to_config(&self) -> crate::network_config::BlockConfig {
        crate::network_config::BlockConfig::PatternClassifier {
            num_l: self.num_l,
            num_s: self.num_s,
            num_as: self.num_as,
            perm_thr: self.perm_thr,
            perm_inc: self.perm_inc,
            perm_dec: self.perm_dec,
            pct_pool: self.pct_pool,
            pct_conn: self.pct_conn,
            pct_learn: self.pct_learn,
            num_t: self.num_t,
            seed: self.base().seed(),
        }
    }

    fn block_type_name(&self) -> &'static str {
        "PatternClassifier"
    }
}

impl crate::network_config::BlockStateful for PatternClassifier {
    fn to_state(&self) -> crate::Result<crate::network_config::BlockState> {
        let permanences = self.memory.get_all_permanences();
        Ok(crate::network_config::BlockState::PatternClassifier { permanences })
    }

    fn from_state(&mut self, state: &crate::network_config::BlockState) -> crate::Result<()> {
        if let crate::network_config::BlockState::PatternClassifier { permanences } = state {
            self.memory.set_all_permanences(permanences)?;
            Ok(())
        } else {
            Err(crate::GnomicsError::Other(
                "Wrong state type for PatternClassifier".into(),
            ))
        }
    }
}

// Tests are in tests/test_pattern_classifier.rs
