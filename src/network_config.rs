//! Network configuration serialization.
//!
//! This module provides types and methods for serializing and deserializing
//! network architectures. It supports saving network configurations (block
//! parameters and topology) to various formats (JSON, YAML, binary).
//!
//! # Architecture
//!
//! The serialization system has three layers:
//! 1. **BlockConfig** - Enum representing configuration for each block type
//! 2. **ConnectionConfig** - Struct representing connections between blocks
//! 3. **NetworkConfig** - Top-level struct containing blocks and connections
//!
//! # Extension for Learned State (Future)
//!
//! The design supports extending to save/load learned state:
//! - Add `BlockState` enum with learned patterns/weights
//! - Add optional `learned_state: Option<Vec<BlockState>>` to NetworkConfig
//! - Implement `to_state()` / `from_state()` methods on blocks
//!
//! # Example
//!
//! ```rust,ignore
//! use gnomics::{Network, blocks::ScalarTransformer};
//!
//! // Create and configure network
//! let mut net = Network::new();
//! let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
//! // ... add more blocks and connections
//! net.build()?;
//!
//! // Save configuration
//! let config = net.to_config()?;
//! let json = serde_json::to_string_pretty(&config)?;
//! std::fs::write("network.json", json)?;
//!
//! // Load configuration
//! let json = std::fs::read_to_string("network.json")?;
//! let config: NetworkConfig = serde_json::from_str(&json)?;
//! let restored_net = Network::from_config(&config)?;
//! ```

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a specific block type.
///
/// This enum captures all the constructor parameters needed to recreate
/// a block. Each variant corresponds to a block type in `gnomics::blocks`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlockConfig {
    /// ScalarTransformer configuration
    ScalarTransformer {
        min_val: f64,
        max_val: f64,
        num_s: usize,
        num_as: usize,
        num_t: usize,
        seed: u64,
    },

    /// DiscreteTransformer configuration
    DiscreteTransformer {
        num_v: usize,
        num_s: usize,
        num_t: usize,
        seed: u64,
    },

    /// PersistenceTransformer configuration
    PersistenceTransformer {
        min_val: f64,
        max_val: f64,
        num_s: usize,
        num_as: usize,
        max_step: usize,
        num_t: usize,
        seed: u64,
    },

    /// PatternPooler configuration
    PatternPooler {
        num_s: usize,
        num_as: usize,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        pct_pool: f64,
        pct_conn: f64,
        pct_learn: f64,
        always_update: bool,
        num_t: usize,
        seed: u64,
    },

    /// PatternClassifier configuration
    PatternClassifier {
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
    },

    /// ContextLearner configuration
    ContextLearner {
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
    },

    /// SequenceLearner configuration
    SequenceLearner {
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
    },
}

/// Type of input connection on a block.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InputType {
    /// Main input (BlockInput)
    Input,
    /// Context input (for ContextLearner, SequenceLearner)
    Context,
}

/// Configuration for a connection between blocks.
///
/// Represents a connection from one block's output to another block's input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionConfig {
    /// Index of source block in the blocks array
    pub source_block: usize,
    /// Index of target block in the blocks array
    pub target_block: usize,
    /// Type of input on target block
    pub input_type: InputType,
    /// Offset parameter for add_child (typically 0)
    pub offset: usize,
}

/// Information about a block in the network.
///
/// Associates a human-readable name with a block configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockInfo {
    /// Human-readable name for this block
    pub name: String,
    /// Block configuration
    pub config: BlockConfig,
}

/// Learned state for blocks with synaptic memory.
///
/// Contains the trained synaptic permanence values that can be
/// saved and restored to preserve learned patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlockState {
    /// Transformer blocks have no learned state
    NoState,

    /// PatternPooler learned state (synaptic permanences)
    PatternPooler {
        /// Permanence values: [dendrite][receptor] -> 0-99
        permanences: Vec<Vec<u8>>,
    },

    /// PatternClassifier learned state
    PatternClassifier {
        /// Permanence values: [dendrite][receptor] -> 0-99
        permanences: Vec<Vec<u8>>,
    },

    /// ContextLearner learned state
    ContextLearner {
        /// Permanence values: [dendrite][receptor] -> 0-99
        permanences: Vec<Vec<u8>>,
    },

    /// SequenceLearner learned state
    SequenceLearner {
        /// Permanence values: [dendrite][receptor] -> 0-99
        permanences: Vec<Vec<u8>>,
    },
}

/// Complete network configuration.
///
/// Contains all information needed to reconstruct a network's architecture:
/// - Block configurations (types and parameters)
/// - Block names (human-readable identifiers)
/// - Connections (topology)
/// - Optional learned state (trained weights)
/// - Optional metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkConfig {
    /// Version of the serialization format
    pub version: String,

    /// Block configurations with names
    pub block_info: Vec<BlockInfo>,

    /// Connections between blocks (references blocks by index)
    pub connections: Vec<ConnectionConfig>,

    /// Optional learned state for each block
    #[serde(default)]
    pub learned_state: Option<Vec<BlockState>>,

    /// Optional metadata (name, description, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    // Deprecated: kept for backwards compatibility
    #[serde(default)]
    pub blocks: Vec<BlockConfig>,
}

impl NetworkConfig {
    /// Create a new network configuration (backwards compatible).
    pub fn new(blocks: Vec<BlockConfig>, connections: Vec<ConnectionConfig>) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            block_info: blocks
                .iter()
                .enumerate()
                .map(|(i, config)| BlockInfo {
                    name: format!("block_{}", i),
                    config: config.clone(),
                })
                .collect(),
            connections,
            learned_state: None,
            metadata: HashMap::new(),
            blocks: Vec::new(), // Deprecated
        }
    }

    /// Create a new network configuration with named blocks.
    pub fn new_with_names(
        block_info: Vec<BlockInfo>,
        connections: Vec<ConnectionConfig>,
    ) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            block_info,
            connections,
            learned_state: None,
            metadata: HashMap::new(),
            blocks: Vec::new(),
        }
    }

    /// Create configuration with learned state.
    pub fn with_state(mut self, learned_state: Vec<BlockState>) -> Self {
        self.learned_state = Some(learned_state);
        self
    }

    /// Add metadata to the configuration.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| crate::GnomicsError::Other(e.to_string()))
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| crate::GnomicsError::Other(e.to_string()))
    }

    /// Serialize to binary (bincode).
    ///
    /// Uses bincode to create a compact binary representation.
    /// For very large learned states (>100MB), JSON may be more reliable.
    pub fn to_binary(&self) -> Result<Vec<u8>> {
        // Use bincode with default configuration
        bincode::serialize(self)
            .map_err(|e| crate::GnomicsError::Other(format!("Binary serialization failed: {}", e)))
    }

    /// Deserialize from binary (bincode).
    ///
    /// Restores a NetworkConfig from compact binary format.
    /// Note: For very large files, this may fail due to bincode's internal limits.
    /// In such cases, use JSON format instead.
    pub fn from_binary(data: &[u8]) -> Result<Self> {
        // Use bincode with default configuration
        bincode::deserialize(data)
            .map_err(|e| crate::GnomicsError::Other(format!("Binary deserialization failed: {}. Try using JSON format for large learned states.", e)))
    }
}

/// Trait for blocks that can export/import configuration.
///
/// Implemented by all block types to support serialization.
pub trait BlockConfigurable {
    /// Export block configuration.
    fn to_config(&self) -> BlockConfig;

    /// Get the type name of this block (for debugging).
    fn block_type_name(&self) -> &'static str;
}

/// Trait for blocks that can export/import learned state.
///
/// Implemented by learning blocks (PatternPooler, PatternClassifier,
/// ContextLearner, SequenceLearner) to support saving/loading trained weights.
pub trait BlockStateful {
    /// Export learned state (synaptic permanences).
    ///
    /// Returns None for blocks without learned state (transformers).
    fn to_state(&self) -> Result<BlockState>;

    /// Import learned state (synaptic permanences).
    ///
    /// Restores trained weights to this block.
    fn from_state(&mut self, state: &BlockState) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_config_serialization() {
        let config = BlockConfig::ScalarTransformer {
            min_val: 0.0,
            max_val: 100.0,
            num_s: 2048,
            num_as: 256,
            num_t: 2,
            seed: 0,
        };

        // Test JSON round-trip
        let json = serde_json::to_string(&config).unwrap();
        let restored: BlockConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, restored);

        // Test binary round-trip
        let binary = bincode::serialize(&config).unwrap();
        let restored: BlockConfig = bincode::deserialize(&binary).unwrap();
        assert_eq!(config, restored);
    }

    #[test]
    fn test_network_config_serialization() {
        let config = NetworkConfig::new(
            vec![
                BlockConfig::ScalarTransformer {
                    min_val: 0.0,
                    max_val: 100.0,
                    num_s: 2048,
                    num_as: 256,
                    num_t: 2,
                    seed: 0,
                },
                BlockConfig::PatternPooler {
                    num_s: 1024,
                    num_as: 40,
                    perm_thr: 20,
                    perm_inc: 2,
                    perm_dec: 1,
                    pct_pool: 0.8,
                    pct_conn: 0.5,
                    pct_learn: 0.3,
                    always_update: false,
                    num_t: 2,
                    seed: 0,
                },
            ],
            vec![ConnectionConfig {
                source_block: 0,
                target_block: 1,
                input_type: InputType::Input,
                offset: 0,
            }],
        );

        // Test JSON round-trip
        let json = config.to_json().unwrap();
        let restored = NetworkConfig::from_json(&json).unwrap();
        assert_eq!(config, restored);

        // Test binary round-trip
        let binary = config.to_binary().unwrap();
        let restored = NetworkConfig::from_binary(&binary).unwrap();
        assert_eq!(config, restored);
    }

    #[test]
    fn test_network_config_metadata() {
        let config = NetworkConfig::new(vec![], vec![])
            .with_metadata("name", "Test Network")
            .with_metadata("author", "Test User");

        assert_eq!(config.metadata.get("name").unwrap(), "Test Network");
        assert_eq!(config.metadata.get("author").unwrap(), "Test User");
    }
}
