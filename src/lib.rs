//! Gnomics - High-Performance Computational Neuroscience Framework
//!
//! Gnomics is a Rust framework for building scalable Machine Learning applications
//! using computational neuroscience principles. The framework models neuron activations
//! with **binary patterns** (vectors of 1s and 0s) that form a "cortical language"
//! for computation.
//!
//! # Key Characteristics
//!
//! - Memory-efficient binary pattern processing
//! - Low-level bitwise operations for performance
//! - Hierarchical block architecture
//! - Inspired by Hierarchical Temporal Memory (HTM) principles
//! - Focus on sparse distributed representations (SDRs)
//!
//! # Architecture
//!
//! The framework is built around several core components:
//!
//! - **BitField**: High-performance bit manipulation using 32-bit words
//! - **Block System**: Computational units with lifecycle management
//! - **Learning Blocks**: Pattern pooling, classification, and temporal learning
//! - **Transformers**: Encoding continuous/discrete values into binary patterns
//!
//! # Examples
//!
//! ## Basic BitField Usage
//!
//! ```
//! use gnomics::BitField;
//!
//! let mut ba = BitField::new(1024);
//! ba.set_bit(10);
//! ba.set_bit(20);
//! ba.set_bit(30);
//!
//! assert_eq!(ba.num_set(), 3);
//! assert_eq!(ba.get_acts(), vec![10, 20, 30]);
//!
//! // Bitwise operations
//! let mut ba2 = BitField::new(1024);
//! ba2.set_bit(20);
//! ba2.set_bit(40);
//!
//! let intersection = &ba & &ba2;
//! assert_eq!(intersection.num_set(), 1); // Only bit 20 is common
//! ```
//!
//! ## Random Pattern Generation
//!
//! ```
//! use gnomics::BitField;
//! use rand::SeedableRng;
//!
//! let mut rng = rand::rngs::StdRng::seed_from_u64(42);
//! let mut ba = BitField::new(2048);
//!
//! // Set 10% of bits randomly
//! ba.random_set_pct(&mut rng, 0.1);
//! assert!(ba.num_set() >= 190 && ba.num_set() <= 210);
//! ```
//!
//! # Performance
//!
//! Gnomics is designed for high performance:
//!
//! - BitField operations use hardware popcount instructions
//! - Word-level copying for efficient data movement
//! - Inline-optimized hot paths
//! - Zero-cost abstractions with Rust's type system
//!
//! Target performance (compared to C++ baseline):
//!
//! - `set_bit`: <3ns
//! - `get_bit`: <2ns
//! - `num_set` (1024 bits): <60ns
//! - Word-level copy (1024 bits): <60ns
//!
//! # Conversion Status
//!
//! Rust conversion progress:
//!
//! - ✅ **Phase 1**: BitField, utilities, error handling
//! - ✅ **Phase 2**: Block infrastructure (Block trait, BlockInput, BlockOutput, BlockMemory)
//! - ✅ **Phase 3**: Transformer blocks (ScalarTransformer, DiscreteTransformer, PersistenceTransformer)
//! - ✅ **Phase 4**: Learning blocks (PatternPooler, PatternClassifier)
//! - ✅ **Phase 5**: Temporal blocks (ContextLearner, SequenceLearner)
//!
//! # Safety
//!
//! Gnomics uses `debug_assert!` for bounds checking in hot paths, providing:
//!
//! - Zero-cost bounds checking in release builds
//! - Full validation during development and testing
//! - Memory safety guaranteed by Rust's type system

// Module declarations
pub mod bitfield;
pub mod error;
pub mod utils;

// Phase 2: Block Infrastructure
pub mod block;
pub mod block_base;
pub mod block_input;
pub mod block_output;
pub mod block_memory;

// Phase 3: Transformer Blocks
pub mod blocks;

// Re-exports for convenient access
pub use bitfield::{bitfield_copy_words, BitField, Word, BITS_PER_WORD};
pub use error::{GnomicsError, Result};

// Phase 2 re-exports
pub use block::Block;
pub use block_base::{BlockBase, BlockBaseAccess};
pub use block_input::BlockInput;
pub use block_output::{BlockOutput, CURR, PREV};
pub use block_memory::{BlockMemory, PERM_MAX, PERM_MIN};

// Phase 3+4+5 re-exports
pub use blocks::{
    ContextLearner, DiscreteTransformer, PatternClassifier, PatternPooler,
    PersistenceTransformer, ScalarTransformer, SequenceLearner,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Framework name
pub const NAME: &str = "Gnomics";

/// Get version string
pub fn version() -> String {
    format!("{} v{}", NAME, VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let ver = version();
        assert!(ver.contains("Gnomics"));
        assert!(ver.contains("1.0.0"));
    }

    #[test]
    fn test_re_exports() {
        // Verify re-exports are accessible
        let _ba = BitField::new(32);
        let _result: Result<()> = Ok(());
        assert_eq!(BITS_PER_WORD, 32);
    }
}
