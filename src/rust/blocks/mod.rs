//! Computational blocks for the Gnomics framework.
//!
//! This module contains implementations of various block types for encoding,
//! learning, and temporal processing.
//!
//! # Transformer Blocks
//!
//! Transformer blocks encode inputs into binary patterns (Sparse Distributed
//! Representations - SDRs):
//!
//! - `ScalarTransformer` - Encodes continuous scalars with overlapping patterns
//! - `DiscreteTransformer` - Encodes categorical values with distinct patterns
//! - `PersistenceTransformer` - Encodes temporal persistence of values
//!
//! # Learning Blocks
//!
//! Learning blocks learn representations and perform classification:
//!
//! - `PatternPooler` - Learns sparse representations via competitive learning
//! - `PatternClassifier` - Supervised classification of binary patterns
//!
//! # Examples
//!
//! ```
//! use gnomics::blocks::ScalarTransformer;
//! use gnomics::Block;
//!
//! let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
//! st.set_value(0.5);
//! st.feedforward(false).unwrap();
//!
//! // Output has 128 active bits representing 0.5
//! assert_eq!(st.output.state.num_set(), 128);
//! ```

pub mod scalar_transformer;
pub mod discrete_transformer;
pub mod persistence_transformer;
pub mod pattern_pooler;
pub mod pattern_classifier;

pub use scalar_transformer::ScalarTransformer;
pub use discrete_transformer::DiscreteTransformer;
pub use persistence_transformer::PersistenceTransformer;
pub use pattern_pooler::PatternPooler;
pub use pattern_classifier::PatternClassifier;
