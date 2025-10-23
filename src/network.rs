//! Network - Automatic execution order management for block graphs.
//!
//! This module provides the `Network` struct for assembling blocks into
//! computational graphs with automatic dependency resolution and execution ordering.
//!
//! # Features
//!
//! - Automatic dependency discovery from block connections
//! - Automatic execution order via topological sort
//! - Cycle detection
//! - Type-safe block access
//!
//! # Example
//!
//! ```
//! use gnomics::{
//!     Network,
//!     blocks::{ScalarTransformer, PatternPooler},
//!     Block, InputAccess, OutputAccess, Result,
//! };
//!
//! # fn main() -> Result<()> {
//! let mut net = Network::new();
//!
//! // Add blocks
//! let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
//! let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
//!
//! // Connect outputs to inputs
//! let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
//! net.get_mut::<PatternPooler>(pooler)?.input_mut().add_child(enc_out, 0);
//!
//! // Build execution plan (auto-discovers dependencies)
//! net.build()?;
//! net.get_mut::<PatternPooler>(pooler)?.init()?;
//!
//! // Execute all blocks in correct dependency order
//! net.get_mut::<ScalarTransformer>(encoder)?.set_value(42.0);
//! net.execute(false)?;
//! # Ok(())
//! # }
//! ```

use crate::{Block, ContextAccess, GnomicsError, InputAccess, OutputAccess, Result};
use crate::execution_recorder::{
    BlockConnection, BlockMetadata, ConnectionType, ExecutionRecorder, ExecutionTrace,
    BitFieldSnapshot,
};
use std::any::Any;
use std::collections::{HashMap, VecDeque};

/// Unique identifier for a block in a Network.
///
/// BlockIds are automatically generated when blocks are added to a Network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BlockId(u32);

impl BlockId {
    /// Create a new unique BlockId.
    ///
    /// Uses an atomic counter to ensure uniqueness across the entire program.
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        BlockId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Create a BlockId from a raw u32 value (for testing).
    #[doc(hidden)]
    pub fn from_raw(id: u32) -> Self {
        BlockId(id)
    }

    /// Get the raw u32 value (for indexing).
    #[inline]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// Wrapper for blocks stored in Network.
///
/// Provides type erasure while allowing downcasting back to concrete types.
struct BlockWrapper {
    id: BlockId,
    block: Box<dyn Block>,
}

impl BlockWrapper {
    fn new<B: Block + 'static>(id: BlockId, block: B) -> Self {
        Self {
            id,
            block: Box::new(block),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self.block.as_any()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.block.as_any_mut()
    }

    fn block_id(&self) -> BlockId {
        self.id
    }

    fn block(&self) -> &dyn Block {
        &*self.block
    }

    fn block_mut(&mut self) -> &mut dyn Block {
        &mut *self.block
    }
}

/// Network manages a computational graph of connected blocks.
///
/// The Network owns all blocks and manages their execution order based on
/// dependencies. Blocks are connected using `connect()` or by manually
/// setting up `add_child()` relationships before calling `build()`.
///
/// # Lifecycle
///
/// 1. Create network: `Network::new()`
/// 2. Add blocks: `let id = net.add(block)`
/// 3. Connect blocks: `net.connect(source, dest)?`
/// 4. Build execution order: `net.build()?`
/// 5. Execute: `net.execute(learn)?`
///
/// # Examples
///
/// ```ignore
/// let mut net = Network::new();
///
/// let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
/// let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
///
/// net.connect(encoder, pooler)?;
/// net.build()?;
///
/// // Training loop
/// for value in training_data {
///     net.get_mut::<ScalarTransformer>(encoder)?.set_value(value);
///     net.execute(true)?;
/// }
/// ```
pub struct Network {
    /// All blocks owned by the network
    blocks: HashMap<BlockId, BlockWrapper>,

    /// Dependency graph: block_id -> [dependent_block_ids]
    dependencies: HashMap<BlockId, Vec<BlockId>>,

    /// Computed execution order (topologically sorted)
    execution_order: Vec<BlockId>,

    /// Whether build() has been called
    is_built: bool,

    /// Optional execution recorder for visualization
    recorder: Option<ExecutionRecorder>,

    /// Block names for visualization (optional, user-provided)
    block_names: HashMap<BlockId, String>,
}

impl Network {
    /// Create a new empty Network.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            dependencies: HashMap::new(),
            execution_order: Vec::new(),
            is_built: false,
            recorder: None,
            block_names: HashMap::new(),
        }
    }

    /// Add a block to the network and return its ID.
    ///
    /// The block will be owned by the network. Use the returned BlockId
    /// to reference the block later.
    ///
    /// # Arguments
    ///
    /// * `block` - The block to add
    ///
    /// # Returns
    ///
    /// BlockId that can be used to reference this block
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut net = Network::new();
    /// let encoder_id = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    /// ```
    pub fn add<B: Block + OutputAccess + 'static>(&mut self, block: B) -> BlockId {
        let id = BlockId::new();

        // Set source block ID on the output for auto-discovery
        block.output().borrow_mut().set_source_block_id(id);

        self.blocks.insert(id, BlockWrapper::new(id, block));
        self.dependencies.insert(id, Vec::new());
        self.is_built = false;
        id
    }

    /// Manually specify a dependency between two blocks.
    ///
    /// **Note**: As of Phase 2, this method is optional. Dependencies are automatically
    /// discovered from block connections when `build()` is called. This method is kept
    /// for backwards compatibility and explicit dependency specification.
    ///
    /// # Arguments
    ///
    /// * `source` - The source block (must execute before dest)
    /// * `dest` - The destination block (depends on source)
    ///
    /// # Errors
    ///
    /// Returns error if either BlockId is not found in the network.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Optional: dependencies are auto-discovered from connections
    /// net.connect(encoder_id, pooler_id)?;
    /// ```
    pub fn connect(&mut self, source: BlockId, dest: BlockId) -> Result<()> {
        if !self.blocks.contains_key(&source) {
            return Err(GnomicsError::Other(format!("Source block {:?} not found", source)));
        }
        if !self.blocks.contains_key(&dest) {
            return Err(GnomicsError::Other(format!("Destination block {:?} not found", dest)));
        }

        self.dependencies
            .entry(dest)
            .or_insert_with(Vec::new)
            .push(source);

        self.is_built = false;
        Ok(())
    }

    /// Build the execution plan by computing topological sort.
    ///
    /// This analyzes the dependency graph by auto-discovering dependencies
    /// from block inputs and computes the correct execution order.
    /// Must be called after adding all blocks and connecting inputs/outputs,
    /// and before calling `execute()`.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Cycle detected in dependency graph
    /// - Graph is malformed
    ///
    /// # Examples
    ///
    /// ```ignore
    /// net.add(encoder);
    /// net.add(pooler);
    /// // Connect outputs to inputs
    /// net.get_mut::<PatternPooler>(pooler)?.input_mut().add_child(encoder_out, 0);
    /// net.build()?;  // Auto-discover dependencies and compute execution order
    /// ```
    pub fn build(&mut self) -> Result<()> {
        // Auto-discover dependencies from block inputs
        self.discover_dependencies();

        self.execution_order = self.topological_sort()?;
        self.is_built = true;
        Ok(())
    }

    /// Execute all blocks in dependency order.
    ///
    /// Calls `execute(learn)` on each block in topologically sorted order.
    ///
    /// # Arguments
    ///
    /// * `learn` - Whether to enable learning (passed to each block's execute)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Network has not been built (call `build()` first)
    /// - Any block's execute() returns an error
    ///
    /// # Examples
    ///
    /// ```ignore
    /// net.build()?;
    /// net.execute(true)?;  // Execute with learning
    /// net.execute(false)?; // Execute without learning
    /// ```
    pub fn execute(&mut self, learn: bool) -> Result<()> {
        if !self.is_built {
            return Err(GnomicsError::Other(
                "Network not built. Call build() before execute()".into(),
            ));
        }

        for &block_id in &self.execution_order {
            self.blocks
                .get_mut(&block_id)
                .expect("Block in execution_order not found")
                .block_mut()
                .execute(learn)?;
        }

        // Record state after execution if recording is active
        self.record_current_state();

        Ok(())
    }

    /// Get mutable reference to a specific block by ID and type.
    ///
    /// This allows you to access block-specific methods (like `set_value()`)
    /// while the network owns the block.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The concrete block type to downcast to
    ///
    /// # Arguments
    ///
    /// * `id` - BlockId returned from `add()`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - BlockId not found in network
    /// - Block is not of type T
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let encoder_id = net.add(ScalarTransformer::new(...));
    /// // Later...
    /// net.get_mut::<ScalarTransformer>(encoder_id)?.set_value(42.0);
    /// ```
    pub fn get_mut<T: Block + 'static>(&mut self, id: BlockId) -> Result<&mut T> {
        self.blocks
            .get_mut(&id)
            .and_then(|wrapper| wrapper.as_any_mut().downcast_mut::<T>())
            .ok_or_else(|| GnomicsError::Other("Block not found or wrong type".into()))
    }

    /// Get immutable reference to a specific block by ID and type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The concrete block type to downcast to
    ///
    /// # Arguments
    ///
    /// * `id` - BlockId returned from `add()`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - BlockId not found in network
    /// - Block is not of type T
    pub fn get<T: Block + 'static>(&self, id: BlockId) -> Result<&T> {
        self.blocks
            .get(&id)
            .and_then(|wrapper| wrapper.as_any().downcast_ref::<T>())
            .ok_or_else(|| GnomicsError::Other("Block not found or wrong type".into()))
    }

    /// Get the computed execution order.
    ///
    /// Returns the list of BlockIds in the order they will be executed.
    /// Only valid after `build()` has been called.
    pub fn execution_order(&self) -> &[BlockId] {
        &self.execution_order
    }

    /// Get the number of blocks in the network.
    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Get the total memory usage of all blocks in the network.
    ///
    /// Returns the sum of memory_usage() for all blocks.
    pub fn memory_usage(&self) -> usize {
        self.blocks
            .values()
            .map(|wrapper| wrapper.block.memory_usage())
            .sum()
    }

    /// Check if the network has been built.
    pub fn is_built(&self) -> bool {
        self.is_built
    }

    /// Get an iterator over all block IDs in the network.
    ///
    /// Returns an iterator that yields BlockIds in arbitrary order.
    /// Use this to iterate through all blocks when you need to access
    /// multiple blocks by type.
    pub fn block_ids(&self) -> impl Iterator<Item = BlockId> + '_ {
        self.blocks.keys().copied()
    }

    /// Connect source block's output to target block's input.
    ///
    /// This is a simplified API that replaces the verbose pattern of getting
    /// outputs and inputs manually. It automatically handles type checking and
    /// provides clear error messages.
    ///
    /// # Arguments
    ///
    /// * `source` - BlockId of the source block (must have output)
    /// * `target` - BlockId of the target block (must have input)
    ///
    /// # Returns
    ///
    /// `Ok(())` if connection succeeded, `Err` if blocks not found or incompatible
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    /// let pooler = net.add(PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
    ///
    /// // Old way (5 lines):
    /// // {
    /// //     let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
    /// //     net.get_mut::<PatternPooler>(pooler)?.input_mut().add_child(enc_out, 0);
    /// // }
    ///
    /// // New way (1 line):
    /// net.connect_to_input(encoder, pooler)?;
    /// ```
    pub fn connect_to_input(&mut self, source: BlockId, target: BlockId) -> Result<()> {
        self.connect_to_input_with_offset(source, target, 0)
    }

    /// Connect source block's output to target block's context input.
    ///
    /// Only ContextLearner and SequenceLearner blocks have context inputs.
    ///
    /// # Arguments
    ///
    /// * `source` - BlockId of the source block (must have output)
    /// * `target` - BlockId of the target block (must have context input)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let input_enc = net.add(DiscreteTransformer::new(10, 512, 2, 0));
    /// let context_enc = net.add(DiscreteTransformer::new(5, 256, 2, 0));
    /// let learner = net.add(ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));
    ///
    /// net.connect_to_input(input_enc, learner)?;
    /// net.connect_to_context(context_enc, learner)?;
    /// ```
    pub fn connect_to_context(&mut self, source: BlockId, target: BlockId) -> Result<()> {
        self.connect_to_context_with_offset(source, target, 0)
    }

    /// Connect source block's output to target block's input with explicit offset.
    ///
    /// The offset parameter is used for advanced scenarios where you need to control
    /// the bit offset in the input concatenation. Most users should use `connect_to_input()`
    /// which defaults offset to 0.
    ///
    /// # Arguments
    ///
    /// * `source` - BlockId of the source block (must have output)
    /// * `target` - BlockId of the target block (must have input)
    /// * `offset` - Bit offset for add_child (typically 0)
    pub fn connect_to_input_with_offset(
        &mut self,
        source: BlockId,
        target: BlockId,
        offset: usize,
    ) -> Result<()> {
        // Step 1: Get source output
        let source_wrapper = self.blocks.get(&source).ok_or_else(|| {
            GnomicsError::Other(format!("Source block {} not found", source.as_usize()))
        })?;

        let source_output = {
            let block_any = source_wrapper.as_any();

            // Try each block type that has OutputAccess
            if let Some(b) = block_any.downcast_ref::<crate::blocks::ScalarTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::DiscreteTransformer>()
            {
                b.output()
            } else if let Some(b) =
                block_any.downcast_ref::<crate::blocks::PersistenceTransformer>()
            {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternClassifier>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::ContextLearner>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                b.output()
            } else {
                return Err(GnomicsError::Other(format!(
                    "Source block {} does not have output",
                    source.as_usize()
                )));
            }
        };

        // Step 2: Get target and add connection
        let target_wrapper = self.blocks.get_mut(&target).ok_or_else(|| {
            GnomicsError::Other(format!("Target block {} not found", target.as_usize()))
        })?;

        let block_any_mut = target_wrapper.as_any_mut();

        // Try each block type that has InputAccess
        if let Some(b) = block_any_mut.downcast_mut::<crate::blocks::PatternPooler>() {
            b.input_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<crate::blocks::PatternClassifier>() {
            b.input_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<crate::blocks::ContextLearner>() {
            b.input_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<crate::blocks::SequenceLearner>() {
            b.input_mut().add_child(source_output, offset);
        } else {
            return Err(GnomicsError::Other(format!(
                "Target block {} does not have input",
                target.as_usize()
            )));
        }

        Ok(())
    }

    /// Connect source block's output to target block's context input with explicit offset.
    ///
    /// Only ContextLearner and SequenceLearner blocks have context inputs.
    ///
    /// # Arguments
    ///
    /// * `source` - BlockId of the source block (must have output)
    /// * `target` - BlockId of the target block (must have context input)
    /// * `offset` - Bit offset for add_child (typically 0)
    pub fn connect_to_context_with_offset(
        &mut self,
        source: BlockId,
        target: BlockId,
        offset: usize,
    ) -> Result<()> {
        // Step 1: Get source output
        let source_wrapper = self.blocks.get(&source).ok_or_else(|| {
            GnomicsError::Other(format!("Source block {} not found", source.as_usize()))
        })?;

        let source_output = {
            let block_any = source_wrapper.as_any();

            // Try each block type that has OutputAccess
            if let Some(b) = block_any.downcast_ref::<crate::blocks::ScalarTransformer>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::DiscreteTransformer>()
            {
                b.output()
            } else if let Some(b) =
                block_any.downcast_ref::<crate::blocks::PersistenceTransformer>()
            {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternClassifier>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::ContextLearner>() {
                b.output()
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                b.output()
            } else {
                return Err(GnomicsError::Other(format!(
                    "Source block {} does not have output",
                    source.as_usize()
                )));
            }
        };

        // Step 2: Get target and add to CONTEXT (only ContextLearner and SequenceLearner)
        let target_wrapper = self.blocks.get_mut(&target).ok_or_else(|| {
            GnomicsError::Other(format!("Target block {} not found", target.as_usize()))
        })?;

        let block_any_mut = target_wrapper.as_any_mut();

        // Only ContextLearner and SequenceLearner have context
        if let Some(b) = block_any_mut.downcast_mut::<crate::blocks::ContextLearner>() {
            b.context_mut().add_child(source_output, offset);
        } else if let Some(b) = block_any_mut.downcast_mut::<crate::blocks::SequenceLearner>() {
            b.context_mut().add_child(source_output, offset);
        } else {
            return Err(GnomicsError::Other(format!(
                "Target block {} does not have context input",
                target.as_usize()
            )));
        }

        Ok(())
    }

    /// Connect multiple sources to a single target's input.
    ///
    /// Convenience method for connecting multiple encoder outputs to a single
    /// downstream block. Equivalent to calling `connect_to_input()` for each source.
    ///
    /// # Arguments
    ///
    /// * `sources` - Array of source BlockIds (must all have output)
    /// * `target` - Target BlockId (must have input)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let enc1 = net.add(ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 0));
    /// let enc2 = net.add(ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 1));
    /// let pooler = net.add(PatternPooler::new(2048, 80, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
    ///
    /// // Instead of:
    /// // net.connect_to_input(enc1, pooler)?;
    /// // net.connect_to_input(enc2, pooler)?;
    ///
    /// // Use:
    /// net.connect_many_to_input(&[enc1, enc2], pooler)?;
    /// ```
    pub fn connect_many_to_input(&mut self, sources: &[BlockId], target: BlockId) -> Result<()> {
        for &source in sources {
            self.connect_to_input(source, target)?;
        }
        Ok(())
    }

    /// Connect multiple sources to a single target's context input.
    ///
    /// Convenience method for connecting multiple encoder outputs to a single
    /// ContextLearner or SequenceLearner context. Equivalent to calling
    /// `connect_to_context()` for each source.
    ///
    /// # Arguments
    ///
    /// * `sources` - Array of source BlockIds (must all have output)
    /// * `target` - Target BlockId (must have context input)
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

    /// Start a fluent connection builder from a source block.
    ///
    /// Allows chaining multiple connections from a single source to multiple targets.
    /// This is useful when you want to connect one encoder to multiple downstream blocks.
    ///
    /// # Arguments
    ///
    /// * `source` - BlockId of the source block
    ///
    /// # Returns
    ///
    /// `ConnectionBuilder` that allows chaining `.to_input()` and `.to_context()` calls
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Connect encoder to multiple targets
    /// net.connect_from(encoder)
    ///     .to_input(pooler)?
    ///     .to_input(classifier)?
    ///     .to_context(learner)?;
    /// ```
    pub fn connect_from(&mut self, source: BlockId) -> ConnectionBuilder<'_> {
        ConnectionBuilder::new(self, source)
    }

    /// Auto-discover dependencies from block inputs.
    ///
    /// Calls get_dependencies() on each block to find which other blocks
    /// they depend on by checking source_block_id of their input children.
    ///
    /// Merges auto-discovered dependencies with manually set ones.
    fn discover_dependencies(&mut self) {
        // Discover dependencies from each block
        let block_ids: Vec<BlockId> = self.blocks.keys().copied().collect();

        for &block_id in &block_ids {
            let wrapper = self.blocks.get(&block_id).unwrap();
            let auto_discovered = wrapper.block().get_dependencies();

            // Merge with existing manual dependencies (if any)
            if !auto_discovered.is_empty() {
                let deps = self.dependencies.entry(block_id).or_insert_with(Vec::new);

                // Add auto-discovered dependencies, avoiding duplicates
                for source in auto_discovered {
                    if !deps.contains(&source) {
                        deps.push(source);
                    }
                }
            }
        }
    }

    /// Compute topological sort of the dependency graph.
    ///
    /// Uses Kahn's algorithm for topological sorting with cycle detection.
    ///
    /// # Returns
    ///
    /// Vec<BlockId> in topologically sorted order (dependencies before dependents)
    ///
    /// # Errors
    ///
    /// Returns error if a cycle is detected in the graph.
    fn topological_sort(&self) -> Result<Vec<BlockId>> {
        // Build reverse dependency graph: source -> [destinations]
        let mut reverse_deps: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        let mut in_degree: HashMap<BlockId, usize> = HashMap::new();

        // Initialize all blocks with zero in-degree
        for &block_id in self.blocks.keys() {
            in_degree.insert(block_id, 0);
            reverse_deps.insert(block_id, Vec::new());
        }

        // Build reverse graph and count in-degrees
        for (&dest, sources) in &self.dependencies {
            in_degree.insert(dest, sources.len());
            for &source in sources {
                reverse_deps.get_mut(&source).unwrap().push(dest);
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<BlockId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node);

            // For each block that depends on this one
            if let Some(neighbors) = reverse_deps.get(&node) {
                for &neighbor in neighbors {
                    let degree = in_degree.get_mut(&neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.blocks.len() {
            return Err(GnomicsError::Other(
                "Cycle detected in network dependency graph".into(),
            ));
        }

        Ok(result)
    }

    /// Clear all blocks and dependencies.
    ///
    /// Resets the network to an empty state.
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.dependencies.clear();
        self.execution_order.clear();
        self.is_built = false;
        self.recorder = None;
        self.block_names.clear();
    }

    /// Start recording execution for visualization.
    ///
    /// Creates an ExecutionRecorder that captures network state during execution.
    /// Must be called before executing the network to capture traces.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// net.start_recording();
    /// // ... execute network ...
    /// let trace = net.stop_recording();
    /// trace.to_json_file("trace.json")?;
    /// ```
    pub fn start_recording(&mut self) {
        let mut recorder = ExecutionRecorder::new();

        // Extract connection information
        let connections = self.extract_connections();
        recorder.set_connections(connections);

        self.recorder = Some(recorder);
    }

    /// Stop recording and return the execution trace.
    ///
    /// Returns the accumulated execution trace, or None if recording was not started.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// net.start_recording();
    /// for i in 0..100 {
    ///     // ... update inputs ...
    ///     net.execute(true)?;
    /// }
    /// let trace = net.stop_recording();
    /// if let Some(trace) = trace {
    ///     trace.to_json_file("trace.json")?;
    /// }
    /// ```
    pub fn stop_recording(&mut self) -> Option<ExecutionTrace> {
        self.recorder.take().map(|r| r.export_trace())
    }

    /// Check if recording is currently active.
    pub fn is_recording(&self) -> bool {
        self.recorder.as_ref().map_or(false, |r| r.is_recording())
    }

    /// Pause recording without losing accumulated data.
    pub fn pause_recording(&mut self) {
        if let Some(recorder) = &mut self.recorder {
            recorder.stop();
        }
    }

    /// Resume recording after pausing.
    pub fn resume_recording(&mut self) {
        if let Some(recorder) = &mut self.recorder {
            recorder.start();
        }
    }

    /// Set a human-readable name for a block (for visualization).
    ///
    /// # Arguments
    ///
    /// * `id` - BlockId of the block to name
    /// * `name` - Human-readable name (e.g., "Temperature Encoder", "Pooler Layer 1")
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    /// net.set_block_name(encoder, "Temperature Encoder");
    /// ```
    pub fn set_block_name(&mut self, id: BlockId, name: impl Into<String>) {
        self.block_names.insert(id, name.into());
    }

    /// Get the name of a block (returns default if not set).
    pub fn get_block_name(&self, id: BlockId) -> String {
        self.block_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Block_{}", id.as_usize()))
    }

    /// Extract connection information from the network for visualization.
    fn extract_connections(&self) -> Vec<BlockConnection> {
        let mut connections = Vec::new();

        for (&target_id, _) in &self.blocks {
            let wrapper = self.blocks.get(&target_id).unwrap();
            let block_any = wrapper.as_any();

            // Check blocks with InputAccess
            if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>() {
                for child in b.input().get_children() {
                    if let Some(source_id) = child.output.borrow().source_block_id() {
                        connections.push(BlockConnection {
                            source_id,
                            target_id,
                            connection_type: ConnectionType::Input,
                            time_offset: child.time_offset,
                        });
                    }
                }
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternClassifier>() {
                for child in b.input().get_children() {
                    if let Some(source_id) = child.output.borrow().source_block_id() {
                        connections.push(BlockConnection {
                            source_id,
                            target_id,
                            connection_type: ConnectionType::Input,
                            time_offset: child.time_offset,
                        });
                    }
                }
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::ContextLearner>() {
                // Input connections
                for child in b.input().get_children() {
                    if let Some(source_id) = child.output.borrow().source_block_id() {
                        connections.push(BlockConnection {
                            source_id,
                            target_id,
                            connection_type: ConnectionType::Input,
                            time_offset: child.time_offset,
                        });
                    }
                }
                // Context connections
                for child in b.context().get_children() {
                    if let Some(source_id) = child.output.borrow().source_block_id() {
                        connections.push(BlockConnection {
                            source_id,
                            target_id,
                            connection_type: ConnectionType::Context,
                            time_offset: child.time_offset,
                        });
                    }
                }
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                // Input connections
                for child in b.input().get_children() {
                    if let Some(source_id) = child.output.borrow().source_block_id() {
                        connections.push(BlockConnection {
                            source_id,
                            target_id,
                            connection_type: ConnectionType::Input,
                            time_offset: child.time_offset,
                        });
                    }
                }
                // Context connections (including self-feedback)
                for child in b.context().get_children() {
                    if let Some(source_id) = child.output.borrow().source_block_id() {
                        connections.push(BlockConnection {
                            source_id,
                            target_id,
                            connection_type: ConnectionType::Context,
                            time_offset: child.time_offset,
                        });
                    }
                }
            }
        }

        connections
    }

    /// Extract metadata for a single block.
    fn extract_block_metadata(&self, id: BlockId) -> Option<BlockMetadata> {
        let wrapper = self.blocks.get(&id)?;
        let block_any = wrapper.as_any();

        let (block_type, num_statelets, num_active) = if let Some(b) =
            block_any.downcast_ref::<crate::blocks::ScalarTransformer>()
        {
            (
                "ScalarTransformer",
                b.output().borrow().state.num_bits(),
                b.output().borrow().state.num_set(),
            )
        } else if let Some(b) = block_any.downcast_ref::<crate::blocks::DiscreteTransformer>() {
            (
                "DiscreteTransformer",
                b.output().borrow().state.num_bits(),
                b.output().borrow().state.num_set(),
            )
        } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PersistenceTransformer>()
        {
            (
                "PersistenceTransformer",
                b.output().borrow().state.num_bits(),
                b.output().borrow().state.num_set(),
            )
        } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>() {
            (
                "PatternPooler",
                b.output().borrow().state.num_bits(),
                b.output().borrow().state.num_set(),
            )
        } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternClassifier>() {
            (
                "PatternClassifier",
                b.output().borrow().state.num_bits(),
                b.output().borrow().state.num_set(),
            )
        } else if let Some(b) = block_any.downcast_ref::<crate::blocks::ContextLearner>() {
            (
                "ContextLearner",
                b.output().borrow().state.num_bits(),
                b.output().borrow().state.num_set(),
            )
        } else if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
            (
                "SequenceLearner",
                b.output().borrow().state.num_bits(),
                b.output().borrow().state.num_set(),
            )
        } else {
            return None;
        };

        Some(BlockMetadata {
            id,
            name: self.get_block_name(id),
            block_type: block_type.to_string(),
            num_statelets,
            num_active,
        })
    }

    /// Record current network state (called during execute if recording is active).
    fn record_current_state(&mut self) {
        // Check if recording first
        let is_recording = self.recorder.as_ref().map_or(false, |r| r.is_recording());
        if !is_recording {
            return;
        }

        // Extract data without holding a borrow on recorder
        let mut block_states = HashMap::new();
        let mut block_metadata = HashMap::new();

        for &block_id in self.blocks.keys() {
            // Extract state
            if let Some(wrapper) = self.blocks.get(&block_id) {
                let block_any = wrapper.as_any();

                // Try to extract output state from each block type
                let state_opt = if let Some(b) =
                    block_any.downcast_ref::<crate::blocks::ScalarTransformer>()
                {
                    Some(BitFieldSnapshot::from_bitfield(
                        &b.output().borrow().state,
                    ))
                } else if let Some(b) =
                    block_any.downcast_ref::<crate::blocks::DiscreteTransformer>()
                {
                    Some(BitFieldSnapshot::from_bitfield(
                        &b.output().borrow().state,
                    ))
                } else if let Some(b) =
                    block_any.downcast_ref::<crate::blocks::PersistenceTransformer>()
                {
                    Some(BitFieldSnapshot::from_bitfield(
                        &b.output().borrow().state,
                    ))
                } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>()
                {
                    Some(BitFieldSnapshot::from_bitfield(
                        &b.output().borrow().state,
                    ))
                } else if let Some(b) =
                    block_any.downcast_ref::<crate::blocks::PatternClassifier>()
                {
                    Some(BitFieldSnapshot::from_bitfield(
                        &b.output().borrow().state,
                    ))
                } else if let Some(b) =
                    block_any.downcast_ref::<crate::blocks::ContextLearner>()
                {
                    Some(BitFieldSnapshot::from_bitfield(
                        &b.output().borrow().state,
                    ))
                } else if let Some(b) =
                    block_any.downcast_ref::<crate::blocks::SequenceLearner>()
                {
                    Some(BitFieldSnapshot::from_bitfield(
                        &b.output().borrow().state,
                    ))
                } else {
                    None
                };

                if let Some(state) = state_opt {
                    block_states.insert(block_id, state);
                }
            }

            // Extract metadata
            if let Some(metadata) = self.extract_block_metadata(block_id) {
                block_metadata.insert(block_id, metadata);
            }
        }

        // Now record the step
        if let Some(recorder) = &mut self.recorder {
            recorder.record_step(block_states, block_metadata);
        }
    }

    /// Export network configuration (architecture only, no learned state).
    ///
    /// Extracts block configurations and topology to create a serializable
    /// representation of the network architecture.
    ///
    /// # Returns
    ///
    /// NetworkConfig containing block parameters and connections.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use gnomics::{Network, blocks::ScalarTransformer, Block, InputAccess, OutputAccess};
    /// # let mut net = Network::new();
    /// # let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    /// # net.build().unwrap();
    /// let config = net.to_config().unwrap();
    /// let json = config.to_json().unwrap();
    /// std::fs::write("network.json", json).unwrap();
    /// ```
    pub fn to_config(&self) -> Result<crate::network_config::NetworkConfig> {
        use crate::network_config::{BlockConfigurable, ConnectionConfig, InputType, NetworkConfig};
        use crate::{InputAccess, ContextAccess};

        // Create ordered list of block IDs for consistent indexing
        let mut block_ids: Vec<BlockId> = self.blocks.keys().copied().collect();
        block_ids.sort_by_key(|id| id.0);

        // Create BlockId -> index mapping
        let id_to_index: HashMap<BlockId, usize> = block_ids
            .iter()
            .enumerate()
            .map(|(idx, &id)| (id, idx))
            .collect();

        // Extract block configurations
        let mut block_configs = Vec::new();
        for &block_id in &block_ids {
            let wrapper = self.blocks.get(&block_id).unwrap();
            let block = wrapper.block();

            // Downcast to BlockConfigurable
            let config_any = block.as_any();

            // Try each block type
            let config = if let Some(b) = config_any.downcast_ref::<crate::blocks::ScalarTransformer>() {
                b.to_config()
            } else if let Some(b) = config_any.downcast_ref::<crate::blocks::DiscreteTransformer>() {
                b.to_config()
            } else if let Some(b) = config_any.downcast_ref::<crate::blocks::PersistenceTransformer>() {
                b.to_config()
            } else if let Some(b) = config_any.downcast_ref::<crate::blocks::PatternPooler>() {
                b.to_config()
            } else if let Some(b) = config_any.downcast_ref::<crate::blocks::PatternClassifier>() {
                b.to_config()
            } else if let Some(b) = config_any.downcast_ref::<crate::blocks::ContextLearner>() {
                b.to_config()
            } else if let Some(b) = config_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                b.to_config()
            } else {
                return Err(GnomicsError::Other("Unknown block type for serialization".into()));
            };

            block_configs.push(config);
        }

        // Extract connections by examining block inputs
        let mut connections = Vec::new();
        for (target_idx, &target_id) in block_ids.iter().enumerate() {
            let wrapper = self.blocks.get(&target_id).unwrap();
            let block_any = wrapper.as_any();

            // Check if block has InputAccess trait
            if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>() {
                let sources = b.input().get_source_blocks();
                for source_id in sources {
                    if let Some(&source_idx) = id_to_index.get(&source_id) {
                        connections.push(ConnectionConfig {
                            source_block: source_idx,
                            target_block: target_idx,
                            input_type: InputType::Input,
                            offset: 0,
                        });
                    }
                }
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternClassifier>() {
                let sources = b.input().get_source_blocks();
                for source_id in sources {
                    if let Some(&source_idx) = id_to_index.get(&source_id) {
                        connections.push(ConnectionConfig {
                            source_block: source_idx,
                            target_block: target_idx,
                            input_type: InputType::Input,
                            offset: 0,
                        });
                    }
                }
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::ContextLearner>() {
                // Input connections
                let sources = b.input().get_source_blocks();
                for source_id in sources {
                    if let Some(&source_idx) = id_to_index.get(&source_id) {
                        connections.push(ConnectionConfig {
                            source_block: source_idx,
                            target_block: target_idx,
                            input_type: InputType::Input,
                            offset: 0,
                        });
                    }
                }
                // Context connections
                let ctx_sources = b.context().get_source_blocks();
                for source_id in ctx_sources {
                    if let Some(&source_idx) = id_to_index.get(&source_id) {
                        connections.push(ConnectionConfig {
                            source_block: source_idx,
                            target_block: target_idx,
                            input_type: InputType::Context,
                            offset: 0,
                        });
                    }
                }
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                // Input connections only (context is self-feedback, handled separately)
                let sources = b.input().get_source_blocks();
                for source_id in sources {
                    if let Some(&source_idx) = id_to_index.get(&source_id) {
                        connections.push(ConnectionConfig {
                            source_block: source_idx,
                            target_block: target_idx,
                            input_type: InputType::Input,
                            offset: 0,
                        });
                    }
                }
                // Note: SequenceLearner's context self-feedback is handled in from_config()
            }
        }

        Ok(NetworkConfig::new(block_configs, connections))
    }

    /// Import network configuration to create a new network.
    ///
    /// Reconstructs a network from a NetworkConfig, creating all blocks
    /// and restoring the topology.
    ///
    /// # Arguments
    ///
    /// * `config` - NetworkConfig previously exported with `to_config()`
    ///
    /// # Returns
    ///
    /// A new Network with the same architecture (blocks will have fresh, untrained state).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use gnomics::{Network, NetworkConfig};
    /// let json = std::fs::read_to_string("network.json").unwrap();
    /// let config = NetworkConfig::from_json(&json).unwrap();
    /// let mut net = Network::from_config(&config).unwrap();
    /// net.build().unwrap();
    /// ```
    pub fn from_config(config: &crate::network_config::NetworkConfig) -> Result<Self> {
        use crate::network_config::{BlockConfig, InputType};
        use crate::{InputAccess, ContextAccess};

        let mut net = Network::new();
        let mut block_ids = Vec::new();

        // Determine which field to use (block_info is new, blocks is deprecated)
        let block_configs: Vec<&BlockConfig> = if !config.block_info.is_empty() {
            config.block_info.iter().map(|info| &info.config).collect()
        } else {
            config.blocks.iter().collect()
        };

        // Create all blocks
        for block_config in &block_configs {
            let block_id = match block_config {
                BlockConfig::ScalarTransformer { min_val, max_val, num_s, num_as, num_t, seed } => {
                    net.add(crate::blocks::ScalarTransformer::new(*min_val, *max_val, *num_s, *num_as, *num_t, *seed))
                }
                BlockConfig::DiscreteTransformer { num_v, num_s, num_t, seed } => {
                    net.add(crate::blocks::DiscreteTransformer::new(*num_v, *num_s, *num_t, *seed))
                }
                BlockConfig::PersistenceTransformer { min_val, max_val, num_s, num_as, max_step, num_t, seed } => {
                    net.add(crate::blocks::PersistenceTransformer::new(*min_val, *max_val, *num_s, *num_as, *max_step, *num_t, *seed))
                }
                BlockConfig::PatternPooler { num_s, num_as, perm_thr, perm_inc, perm_dec, pct_pool, pct_conn, pct_learn, always_update, num_t, seed } => {
                    net.add(crate::blocks::PatternPooler::new(*num_s, *num_as, *perm_thr, *perm_inc, *perm_dec, *pct_pool, *pct_conn, *pct_learn, *always_update, *num_t, *seed))
                }
                BlockConfig::PatternClassifier { num_l, num_s, num_as, perm_thr, perm_inc, perm_dec, pct_pool, pct_conn, pct_learn, num_t, seed } => {
                    net.add(crate::blocks::PatternClassifier::new(*num_l, *num_s, *num_as, *perm_thr, *perm_inc, *perm_dec, *pct_pool, *pct_conn, *pct_learn, *num_t, *seed))
                }
                BlockConfig::ContextLearner { num_c, num_spc, num_dps, num_rpd, d_thresh, perm_thr, perm_inc, perm_dec, num_t, always_update, seed } => {
                    net.add(crate::blocks::ContextLearner::new(*num_c, *num_spc, *num_dps, *num_rpd, *d_thresh, *perm_thr, *perm_inc, *perm_dec, *num_t, *always_update, *seed))
                }
                BlockConfig::SequenceLearner { num_c, num_spc, num_dps, num_rpd, d_thresh, perm_thr, perm_inc, perm_dec, num_t, always_update, seed } => {
                    net.add(crate::blocks::SequenceLearner::new(*num_c, *num_spc, *num_dps, *num_rpd, *d_thresh, *perm_thr, *perm_inc, *perm_dec, *num_t, *always_update, *seed))
                }
            };
            block_ids.push(block_id);
        }

        // Restore connections
        for conn in &config.connections {
            let source_id = block_ids[conn.source_block];
            let target_id = block_ids[conn.target_block];

            // Get output from source block (needs OutputAccess trait)
            let output = {
                let wrapper = net.blocks.get(&source_id).unwrap();
                let block_any = wrapper.as_any();

                // Try to get output from any block type that has OutputAccess
                if let Some(b) = block_any.downcast_ref::<crate::blocks::ScalarTransformer>() {
                    b.output()
                } else if let Some(b) = block_any.downcast_ref::<crate::blocks::DiscreteTransformer>() {
                    b.output()
                } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PersistenceTransformer>() {
                    b.output()
                } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>() {
                    b.output()
                } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternClassifier>() {
                    b.output()
                } else if let Some(b) = block_any.downcast_ref::<crate::blocks::ContextLearner>() {
                    b.output()
                } else if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                    b.output()
                } else {
                    return Err(GnomicsError::Other("Unknown block type for output access".into()));
                }
            };

            // Add child to target block's input
            let wrapper = net.blocks.get_mut(&target_id).unwrap();
            let block_any = wrapper.as_any_mut();

            match conn.input_type {
                InputType::Input => {
                    if let Some(b) = block_any.downcast_mut::<crate::blocks::PatternPooler>() {
                        b.input_mut().add_child(output, conn.offset);
                    } else if let Some(b) = block_any.downcast_mut::<crate::blocks::PatternClassifier>() {
                        b.input_mut().add_child(output, conn.offset);
                    } else if let Some(b) = block_any.downcast_mut::<crate::blocks::ContextLearner>() {
                        b.input_mut().add_child(output, conn.offset);
                    } else if let Some(b) = block_any.downcast_mut::<crate::blocks::SequenceLearner>() {
                        b.input_mut().add_child(output, conn.offset);
                    }
                }
                InputType::Context => {
                    if let Some(b) = block_any.downcast_mut::<crate::blocks::ContextLearner>() {
                        b.context_mut().add_child(output, conn.offset);
                    } else if let Some(b) = block_any.downcast_mut::<crate::blocks::SequenceLearner>() {
                        b.context_mut().add_child(output, conn.offset);
                    }
                }
            }
        }

        // Handle SequenceLearner self-feedback connections
        for (idx, &block_config) in block_configs.iter().enumerate() {
            if matches!(block_config, BlockConfig::SequenceLearner { .. }) {
                let block_id = block_ids[idx];
                let output = {
                    let wrapper = net.blocks.get(&block_id).unwrap();
                    let block_any = wrapper.as_any();
                    if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                        b.output()
                    } else {
                        continue;
                    }
                };

                let wrapper = net.blocks.get_mut(&block_id).unwrap();
                let block_any = wrapper.as_any_mut();
                if let Some(b) = block_any.downcast_mut::<crate::blocks::SequenceLearner>() {
                    b.context_mut().add_child(output, 1); // PREV time step
                }
            }
        }

        Ok(net)
    }

    /// Export network configuration with learned state.
    ///
    /// This method saves both the architecture (block configurations and topology)
    /// and the learned state (synaptic permanences) of all blocks. This allows
    /// you to save trained models and resume training or perform inference later.
    ///
    /// # Returns
    ///
    /// A NetworkConfig with both configuration and learned state.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use gnomics::{Network, blocks::PatternPooler, Block};
    /// # let mut net = Network::new();
    /// # let pooler = net.add(PatternPooler::new(512, 20, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
    /// # net.build().unwrap();
    /// // Train network...
    /// // net.execute(true)?;
    ///
    /// // Save trained model
    /// let config = net.to_config_with_state().unwrap();
    /// let json = config.to_json().unwrap();
    /// std::fs::write("trained_model.json", json).unwrap();
    /// ```
    pub fn to_config_with_state(&self) -> Result<crate::network_config::NetworkConfig> {
        use crate::network_config::BlockStateful;

        // First, get the base configuration
        let mut config = self.to_config()?;

        // Create ordered list of block IDs (same as in to_config)
        let mut block_ids: Vec<BlockId> = self.blocks.keys().copied().collect();
        block_ids.sort_by_key(|id| id.0);

        // Extract learned state from each block
        let mut states = Vec::new();
        for &block_id in &block_ids {
            let wrapper = self.blocks.get(&block_id).unwrap();
            let block_any = wrapper.as_any();

            // Try each block type and call to_state()
            let state = if let Some(b) = block_any.downcast_ref::<crate::blocks::ScalarTransformer>() {
                b.to_state()?
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::DiscreteTransformer>() {
                b.to_state()?
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PersistenceTransformer>() {
                b.to_state()?
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternPooler>() {
                b.to_state()?
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::PatternClassifier>() {
                b.to_state()?
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::ContextLearner>() {
                b.to_state()?
            } else if let Some(b) = block_any.downcast_ref::<crate::blocks::SequenceLearner>() {
                b.to_state()?
            } else {
                return Err(GnomicsError::Other("Unknown block type for state export".into()));
            };

            states.push(state);
        }

        // Add learned state to configuration
        config.learned_state = Some(states);

        Ok(config)
    }

    /// Import network configuration with learned state (fully automated).
    ///
    /// Reconstructs a network from a NetworkConfig and restores the learned
    /// state (synaptic permanences) of all blocks. This method automatically handles:
    /// 1. Creating all blocks from configuration
    /// 2. Building the network (establishing execution order)
    /// 3. Initializing learning blocks (allocating memory structures)
    /// 4. Restoring learned state into initialized blocks
    ///
    /// The returned network is fully initialized and ready for immediate use.
    ///
    /// # Arguments
    ///
    /// * `config` - NetworkConfig with learned state, previously exported with `to_config_with_state()`
    ///
    /// # Returns
    ///
    /// A new Network with the same architecture and learned state, fully initialized.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use gnomics::{Network, NetworkConfig};
    /// // Load trained model
    /// let json = std::fs::read_to_string("trained_model.json").unwrap();
    /// let config = NetworkConfig::from_json(&json).unwrap();
    /// let mut net = Network::from_config_with_state(&config).unwrap();
    ///
    /// // Ready to use immediately - no manual build/init needed!
    /// net.execute(false)?;  // Inference with trained weights
    /// ```
    pub fn from_config_with_state(config: &crate::network_config::NetworkConfig) -> Result<Self> {
        use crate::network_config::BlockStateful;

        // Step 1: Create the network from configuration
        let mut net = Self::from_config(config)?;

        // Step 2: Build the network (establish execution order)
        net.build()?;

        // Step 3: Initialize learning blocks (must be done before restoring state)
        // This allocates memory structures that will receive the learned state
        let block_ids: Vec<BlockId> = net.blocks.keys().copied().collect();
        for &block_id in &block_ids {
            let wrapper = net.blocks.get_mut(&block_id).unwrap();
            let block_any = wrapper.as_any_mut();

            // Initialize blocks that have memory (learning blocks)
            if let Some(b) = block_any.downcast_mut::<crate::blocks::PatternPooler>() {
                b.init()?;
            } else if let Some(b) = block_any.downcast_mut::<crate::blocks::PatternClassifier>() {
                b.init()?;
            } else if let Some(b) = block_any.downcast_mut::<crate::blocks::ContextLearner>() {
                b.init()?;
            } else if let Some(b) = block_any.downcast_mut::<crate::blocks::SequenceLearner>() {
                b.init()?;
            }
        }

        // Step 4: Restore learned state (if present)
        if let Some(states) = &config.learned_state {
            // Create ordered list of block IDs (same as in to_config)
            let mut sorted_ids: Vec<BlockId> = net.blocks.keys().copied().collect();
            sorted_ids.sort_by_key(|id| id.as_usize());

            // Verify we have the right number of states
            if states.len() != sorted_ids.len() {
                return Err(GnomicsError::Other(
                    format!("State count mismatch: {} states for {} blocks",
                            states.len(), sorted_ids.len()).into()
                ));
            }

            // Restore learned state to each block
            for (idx, &block_id) in sorted_ids.iter().enumerate() {
                let state = &states[idx];
                let wrapper = net.blocks.get_mut(&block_id).unwrap();
                let block_any = wrapper.as_any_mut();

                // Try each block type and call from_state()
                if let Some(b) = block_any.downcast_mut::<crate::blocks::ScalarTransformer>() {
                    b.from_state(state)?;
                } else if let Some(b) = block_any.downcast_mut::<crate::blocks::DiscreteTransformer>() {
                    b.from_state(state)?;
                } else if let Some(b) = block_any.downcast_mut::<crate::blocks::PersistenceTransformer>() {
                    b.from_state(state)?;
                } else if let Some(b) = block_any.downcast_mut::<crate::blocks::PatternPooler>() {
                    b.from_state(state)?;
                } else if let Some(b) = block_any.downcast_mut::<crate::blocks::PatternClassifier>() {
                    b.from_state(state)?;
                } else if let Some(b) = block_any.downcast_mut::<crate::blocks::ContextLearner>() {
                    b.from_state(state)?;
                } else if let Some(b) = block_any.downcast_mut::<crate::blocks::SequenceLearner>() {
                    b.from_state(state)?;
                } else {
                    return Err(GnomicsError::Other("Unknown block type for state import".into()));
                }
            }
        }

        Ok(net)
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

/// Fluent builder for chaining multiple connections from a single source.
///
/// Created by `Network::connect_from()`. Allows connecting one source to multiple
/// targets in a fluent/chainable style.
///
/// # Examples
///
/// ```ignore
/// net.connect_from(encoder)
///     .to_input(pooler)?
///     .to_input(classifier)?
///     .to_context(learner)?;
/// ```
pub struct ConnectionBuilder<'a> {
    network: &'a mut Network,
    source: BlockId,
}

impl<'a> ConnectionBuilder<'a> {
    fn new(network: &'a mut Network, source: BlockId) -> Self {
        Self { network, source }
    }

    /// Connect to target's input.
    ///
    /// Chainable - returns self to allow additional connections.
    pub fn to_input(self, target: BlockId) -> Result<Self> {
        self.network.connect_to_input(self.source, target)?;
        Ok(self)
    }

    /// Connect to target's context.
    ///
    /// Chainable - returns self to allow additional connections.
    pub fn to_context(self, target: BlockId) -> Result<Self> {
        self.network.connect_to_context(self.source, target)?;
        Ok(self)
    }

    /// Connect to target's input with explicit offset.
    ///
    /// Chainable - returns self to allow additional connections.
    pub fn to_input_with_offset(self, target: BlockId, offset: usize) -> Result<Self> {
        self.network
            .connect_to_input_with_offset(self.source, target, offset)?;
        Ok(self)
    }

    /// Connect to target's context with explicit offset.
    ///
    /// Chainable - returns self to allow additional connections.
    pub fn to_context_with_offset(self, target: BlockId, offset: usize) -> Result<Self> {
        self.network
            .connect_to_context_with_offset(self.source, target, offset)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockBase, BlockOutput, OutputAccess};
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;

    // Mock block for testing
    struct MockBlock {
        id: u32,
        base: BlockBase,
        output: Rc<RefCell<BlockOutput>>,
        execute_count: usize,
    }

    impl MockBlock {
        fn new(id: u32) -> Self {
            Self {
                id,
                base: BlockBase::new(id as u64),
                output: Rc::new(RefCell::new(BlockOutput::new())),
                execute_count: 0,
            }
        }
    }

    impl Block for MockBlock {
        fn init(&mut self) -> Result<()> {
            Ok(())
        }

        fn save(&self, _path: &Path) -> Result<()> {
            Ok(())
        }

        fn load(&mut self, _path: &Path) -> Result<()> {
            Ok(())
        }

        fn clear(&mut self) {}

        fn step(&mut self) {}

        fn pull(&mut self) {}

        fn compute(&mut self) {}

        fn store(&mut self) {
            self.output.borrow_mut().store();
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

        fn execute(&mut self, _learn: bool) -> Result<()> {
            self.execute_count += 1;
            Ok(())
        }
    }

    impl OutputAccess for MockBlock {
        fn output(&self) -> Rc<RefCell<BlockOutput>> {
            Rc::clone(&self.output)
        }
    }

    #[test]
    fn test_network_new() {
        let net = Network::new();
        assert_eq!(net.num_blocks(), 0);
        assert!(!net.is_built());
    }

    #[test]
    fn test_add_block() {
        let mut net = Network::new();
        let id1 = net.add(MockBlock::new(1));
        let id2 = net.add(MockBlock::new(2));

        assert_eq!(net.num_blocks(), 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_connect_blocks() {
        let mut net = Network::new();
        let id1 = net.add(MockBlock::new(1));
        let id2 = net.add(MockBlock::new(2));

        let result = net.connect(id1, id2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_connect_invalid_blocks() {
        let mut net = Network::new();
        let id1 = net.add(MockBlock::new(1));
        let invalid_id = BlockId(9999);

        let result = net.connect(id1, invalid_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_simple() {
        let mut net = Network::new();
        let id1 = net.add(MockBlock::new(1));
        let id2 = net.add(MockBlock::new(2));

        net.connect(id1, id2).unwrap();
        let result = net.build();

        assert!(result.is_ok());
        assert!(net.is_built());
        assert_eq!(net.execution_order().len(), 2);
        assert_eq!(net.execution_order()[0], id1);
        assert_eq!(net.execution_order()[1], id2);
    }

    #[test]
    fn test_build_cycle_detection() {
        let mut net = Network::new();
        let id1 = net.add(MockBlock::new(1));
        let id2 = net.add(MockBlock::new(2));

        // Create cycle: 1 -> 2 -> 1
        net.connect(id1, id2).unwrap();
        net.connect(id2, id1).unwrap();

        let result = net.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_execute() {
        let mut net = Network::new();
        let id1 = net.add(MockBlock::new(1));
        let id2 = net.add(MockBlock::new(2));

        net.connect(id1, id2).unwrap();
        net.build().unwrap();

        net.execute(false).unwrap();

        // Verify both blocks were executed
        let block1 = net.get::<MockBlock>(id1).unwrap();
        assert_eq!(block1.execute_count, 1);

        let block2 = net.get::<MockBlock>(id2).unwrap();
        assert_eq!(block2.execute_count, 1);
    }

    #[test]
    fn test_execute_without_build() {
        let mut net = Network::new();
        net.add(MockBlock::new(1));

        let result = net.execute(false);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_mut() {
        let mut net = Network::new();
        let id = net.add(MockBlock::new(42));

        let block = net.get_mut::<MockBlock>(id).unwrap();
        assert_eq!(block.id, 42);
    }

    #[test]
    fn test_topological_sort_complex() {
        let mut net = Network::new();

        // Create diamond dependency:
        //     1
        //    / \
        //   2   3
        //    \ /
        //     4
        let id1 = net.add(MockBlock::new(1));
        let id2 = net.add(MockBlock::new(2));
        let id3 = net.add(MockBlock::new(3));
        let id4 = net.add(MockBlock::new(4));

        net.connect(id1, id2).unwrap();
        net.connect(id1, id3).unwrap();
        net.connect(id2, id4).unwrap();
        net.connect(id3, id4).unwrap();

        net.build().unwrap();

        let order = net.execution_order();
        assert_eq!(order.len(), 4);

        // id1 must be first
        assert_eq!(order[0], id1);

        // id2 and id3 must come before id4
        let id2_pos = order.iter().position(|&x| x == id2).unwrap();
        let id3_pos = order.iter().position(|&x| x == id3).unwrap();
        let id4_pos = order.iter().position(|&x| x == id4).unwrap();

        assert!(id2_pos < id4_pos);
        assert!(id3_pos < id4_pos);
    }

    #[test]
    fn test_clear() {
        let mut net = Network::new();
        net.add(MockBlock::new(1));
        net.add(MockBlock::new(2));
        net.build().unwrap();

        net.clear();

        assert_eq!(net.num_blocks(), 0);
        assert!(!net.is_built());
    }
}
