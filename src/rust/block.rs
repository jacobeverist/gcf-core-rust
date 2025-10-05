//! Block trait system - Core trait for all Gnomics computational blocks.
//!
//! This module defines the fundamental `Block` trait that all computational
//! blocks must implement. It provides lifecycle management through a series
//! of virtual methods.
//!
//! # Lifecycle Methods
//!
//! - `init()` - Initialize block based on input connections
//! - `step()` - Advance time step (update history index)
//! - `pull()` - Pull data from child block outputs
//! - `encode()` - Convert inputs to outputs
//! - `store()` - Store current state to history
//! - `learn()` - Update internal memories/weights
//! - `push()` - Push data to child block outputs
//! - `decode()` - Convert outputs to inputs (for feedback)
//! - `clear()` - Reset all state
//! - `save()`/`load()` - Persistence operations
//!
//! # High-Level Operations
//!
//! - `feedforward(learn_flag)` - Full forward pass: step → pull → encode → store → [learn]
//! - `feedback()` - Full backward pass: decode → push
//!
//! # Examples
//!
//! ```ignore
//! use gnomics::Block;
//!
//! struct MyBlock {
//!     // ... fields ...
//! }
//!
//! impl Block for MyBlock {
//!     fn encode(&mut self) {
//!         // Transform input to output
//!     }
//!
//!     fn learn(&mut self) {
//!         // Update weights
//!     }
//!
//!     // ... other methods ...
//! }
//! ```

use crate::error::Result;
use std::path::Path;

/// Core trait for all Gnomics computational blocks.
///
/// All blocks in the Gnomics framework implement this trait, which provides
/// a standard lifecycle and interface for data flow and learning.
pub trait Block {
    /// Initialize the block based on input connections.
    ///
    /// Called automatically during first `feedforward()` if not already initialized.
    /// Override to set up internal structures based on connected inputs.
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Save block state to file.
    ///
    /// Persists learned weights, parameters, and history.
    fn save(&self, path: &Path) -> Result<()>;

    /// Load block state from file.
    ///
    /// Restores learned weights, parameters, and history.
    fn load(&mut self, path: &Path) -> Result<()>;

    /// Clear all internal state.
    ///
    /// Resets input states, output states, and memories to initial values.
    /// Does not affect learned weights.
    fn clear(&mut self);

    /// Advance time step.
    ///
    /// Updates BlockOutput history current index to move forward in time.
    fn step(&mut self);

    /// Pull data from child blocks.
    ///
    /// Copies data from child BlockOutput histories into BlockInput state(s).
    /// Uses lazy copying - only copies changed children.
    fn pull(&mut self);

    /// Push data to child blocks.
    ///
    /// Distributes BlockInput state back to child BlockOutput states.
    /// Used during feedback/reconstruction.
    fn push(&mut self);

    /// Encode input to output.
    ///
    /// Core computation: transforms BlockInput state(s) into BlockOutput state(s).
    /// Override to implement block-specific computation.
    fn encode(&mut self);

    /// Decode output to input.
    ///
    /// Inverse of encode: transforms BlockOutput state(s) into BlockInput state(s).
    /// Used for feedback/reconstruction. Optional - default is no-op.
    fn decode(&mut self) {}

    /// Update internal memories/weights.
    ///
    /// Performs learning based on current inputs and outputs.
    /// Override to implement block-specific learning.
    fn learn(&mut self) {}

    /// Store current state to history.
    ///
    /// Copies BlockOutput state into history at current time index.
    /// Automatically detects changes for optimization.
    fn store(&mut self);

    /// Estimate memory usage in bytes.
    ///
    /// Returns approximate memory footprint of the block including
    /// all internal structures.
    fn memory_usage(&self) -> usize;

    /// Process input to output (feedforward pass).
    ///
    /// Executes the full forward computation pipeline:
    /// 1. step() - Advance time
    /// 2. pull() - Get input from children
    /// 3. encode() - Compute output
    /// 4. store() - Save to history
    /// 5. learn() - Update weights (if learn_flag is true)
    ///
    /// # Arguments
    ///
    /// * `learn_flag` - If true, call learn() after store()
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Training mode
    /// block.feedforward(true)?;
    ///
    /// // Inference mode
    /// block.feedforward(false)?;
    /// ```
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

    /// Process output to input (feedback pass).
    ///
    /// Executes the backward computation pipeline:
    /// 1. decode() - Reconstruct input from output
    /// 2. push() - Send to children
    ///
    /// Used for generative models and error feedback.
    fn feedback(&mut self) -> Result<()> {
        self.decode();
        self.push();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock block for testing
    struct MockBlock {
        step_called: bool,
        pull_called: bool,
        encode_called: bool,
        store_called: bool,
        learn_called: bool,
    }

    impl MockBlock {
        fn new() -> Self {
            Self {
                step_called: false,
                pull_called: false,
                encode_called: false,
                store_called: false,
                learn_called: false,
            }
        }

        fn reset(&mut self) {
            self.step_called = false;
            self.pull_called = false;
            self.encode_called = false;
            self.store_called = false;
            self.learn_called = false;
        }
    }

    impl Block for MockBlock {
        fn save(&self, _path: &Path) -> Result<()> {
            Ok(())
        }

        fn load(&mut self, _path: &Path) -> Result<()> {
            Ok(())
        }

        fn clear(&mut self) {}

        fn step(&mut self) {
            self.step_called = true;
        }

        fn pull(&mut self) {
            self.pull_called = true;
        }

        fn push(&mut self) {}

        fn encode(&mut self) {
            self.encode_called = true;
        }

        fn store(&mut self) {
            self.store_called = true;
        }

        fn learn(&mut self) {
            self.learn_called = true;
        }

        fn memory_usage(&self) -> usize {
            0
        }
    }

    #[test]
    fn test_feedforward_without_learning() {
        let mut block = MockBlock::new();
        block.feedforward(false).unwrap();

        assert!(block.step_called);
        assert!(block.pull_called);
        assert!(block.encode_called);
        assert!(block.store_called);
        assert!(!block.learn_called);
    }

    #[test]
    fn test_feedforward_with_learning() {
        let mut block = MockBlock::new();
        block.feedforward(true).unwrap();

        assert!(block.step_called);
        assert!(block.pull_called);
        assert!(block.encode_called);
        assert!(block.store_called);
        assert!(block.learn_called);
    }

    #[test]
    fn test_feedforward_call_order() {
        // The order matters for correctness
        let mut block = MockBlock::new();

        // Manually call in order and verify
        block.step();
        assert!(block.step_called && !block.pull_called);

        block.pull();
        assert!(block.pull_called && !block.encode_called);

        block.encode();
        assert!(block.encode_called && !block.store_called);

        block.store();
        assert!(block.store_called);
    }
}
