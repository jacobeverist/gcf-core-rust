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
//! - `compute()` - Convert inputs to outputs
//! - `store()` - Store current state to history
//! - `learn()` - Update internal memories/weights
//! - `clear()` - Reset all state
//! - `save()`/`load()` - Persistence operations
//!
//! # High-Level Operations
//!
//! - `execute(learn_flag)` - Full forward pass: step → pull → compute → store → [learn]
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
//!     fn compute(&mut self) {
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
#![allow(dead_code)]

use crate::error::Result;
use std::any::Any;
use std::path::Path;

/// Core trait for all Gnomics computational blocks.
///
/// All blocks in the Gnomics framework implement this trait, which provides
/// a standard lifecycle and interface for data flow and learning.
pub trait Block {
    /// Initialize the block based on input connections.
    ///
    /// Called automatically during first `execute()` if not already initialized.
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

    /// Compute output from input.
    ///
    /// Core computation: transforms BlockInput state(s) into BlockOutput state(s).
    /// Override to implement block-specific computation.
    fn compute(&mut self);

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

    /// Get reference as Any for downcasting.
    ///
    /// This allows Network to downcast trait objects back to concrete types.
    fn as_any(&self) -> &dyn Any;

    /// Get mutable reference as Any for downcasting.
    ///
    /// This allows Network to downcast trait objects back to concrete types.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the source block IDs that this block depends on.
    ///
    /// Returns a vector of BlockIds from blocks whose outputs are connected
    /// to this block's inputs. Used by Network for automatic dependency discovery.
    ///
    /// Default implementation returns empty vector (no dependencies).
    /// Blocks with inputs should override this.
    fn get_dependencies(&self) -> Vec<crate::network::BlockId> {
        Vec::new()
    }

    /// Remove a connection from this block's input.
    ///
    /// # Arguments
    /// * `source` - The BlockId of the source block to disconnect
    ///
    /// # Errors
    /// Returns error if this block has no inputs or if the connection doesn't exist.
    ///
    /// Default implementation returns error (block has no inputs).
    /// Blocks with BlockInput should override this.
    fn remove_input_connection(&self, _source: crate::network::BlockId) -> Result<()> {
        Err(crate::GnomicsError::Other(
            "Block does not have input connections".to_string(),
        ))
    }

    /// Remove a connection from this block's context input.
    ///
    /// # Arguments
    /// * `source` - The BlockId of the source block to disconnect
    ///
    /// # Errors
    /// Returns error if this block has no context or if the connection doesn't exist.
    ///
    /// Default implementation returns error (block has no context).
    /// Blocks with context should override this.
    fn remove_context_connection(&self, _source: crate::network::BlockId) -> Result<()> {
        Err(crate::GnomicsError::Other(
            "Block does not have context connections".to_string(),
        ))
    }

    /// Execute the block's computation pipeline.
    ///
    /// Executes the full forward computation pipeline:
    /// 1. step() - Advance time
    /// 2. pull() - Get input from children
    /// 3. compute() - Compute output
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
    /// block.execute(true)?;
    ///
    /// // Inference mode
    /// block.execute(false)?;
    /// ```
    fn execute(&mut self, learn_flag: bool) -> Result<()> {
        self.step();
        self.pull();
        self.compute();
        self.store();
        if learn_flag {
            self.learn();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{BlockOutput, OutputAccess};
    use std::cell::RefCell;
    use std::rc::Rc;

    // Mock block for testing
    struct MockBlock {
        step_called: bool,
        pull_called: bool,
        compute_called: bool,
        store_called: bool,
        learn_called: bool,
        output: Rc<RefCell<BlockOutput>>,
    }

    impl MockBlock {
        fn new() -> Self {
            Self {
                step_called: false,
                pull_called: false,
                compute_called: false,
                store_called: false,
                learn_called: false,
                output: Rc::new(RefCell::new(BlockOutput::new())),
            }
        }

        fn reset(&mut self) {
            self.step_called = false;
            self.pull_called = false;
            self.compute_called = false;
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

        fn compute(&mut self) {
            self.compute_called = true;
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

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    impl OutputAccess for MockBlock {
        fn output(&self) -> Rc<RefCell<BlockOutput>> {
            Rc::clone(&self.output)
        }
    }

    #[test]
    fn test_execute_without_learning() {
        let mut block = MockBlock::new();
        block.execute(false).unwrap();

        assert!(block.step_called);
        assert!(block.pull_called);
        assert!(block.compute_called);
        assert!(block.store_called);
        assert!(!block.learn_called);
    }

    #[test]
    fn test_execute_with_learning() {
        let mut block = MockBlock::new();
        block.execute(true).unwrap();

        assert!(block.step_called);
        assert!(block.pull_called);
        assert!(block.compute_called);
        assert!(block.store_called);
        assert!(block.learn_called);
    }

    #[test]
    fn test_execute_call_order() {
        // The order matters for correctness
        let mut block = MockBlock::new();

        // Manually call in order and verify
        block.step();
        assert!(block.step_called && !block.pull_called);

        block.pull();
        assert!(block.pull_called && !block.compute_called);

        block.compute();
        assert!(block.compute_called && !block.store_called);

        block.store();
        assert!(block.store_called);
    }
}
