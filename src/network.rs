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

use crate::{Block, GnomicsError, OutputAccess, Result};
use std::any::Any;
use std::collections::{HashMap, VecDeque};

/// Unique identifier for a block in a Network.
///
/// BlockIds are automatically generated when blocks are added to a Network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

impl Network {
    /// Create a new empty Network.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            dependencies: HashMap::new(),
            execution_order: Vec::new(),
            is_built: false,
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

    /// Check if the network has been built.
    pub fn is_built(&self) -> bool {
        self.is_built
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
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
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
